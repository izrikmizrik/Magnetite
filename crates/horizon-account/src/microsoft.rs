//! The Microsoft account flow: the OAuth device-code flow, then Xbox Live, XSTS and the
//! Minecraft services, the game ownership check and the profile (docs/microsoft-account.md).
//! The flows mirror the desktop launcher's `launcher/src/account.rs`, minus its keychain and
//! proxy layers, so a phone's game runs the same sign-in as the PC launcher.

use crate::{Account, Body, Endpoints, Kind, SCOPE, agent, now, send, str_field};
use serde_json::{Value, json};
use std::time::Duration;

/// The Minecraft token is renewed when it has less than this left.
const RENEW_BEFORE: u64 = 3600;

/// What the player types at the verification page.
#[derive(Clone, PartialEq)]
pub struct DeviceCode {
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_at: u64,
    device_code: String,
}

impl std::fmt::Debug for DeviceCode {
    // The device code can be traded for tokens once the player approves: never shown.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceCode")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .finish_non_exhaustive()
    }
}

impl DeviceCode {
    /// The code as JSON, for a host that may be ended while the player is at the page (Android
    /// unloads a backgrounded app): kept in a private file, it lets the next start wait for the
    /// same approval. It holds the device code, which buys tokens once the player approves and
    /// until it expires: keep it like the refresh secret.
    pub fn to_json(&self) -> String {
        serde_json::json!({
            "user_code": self.user_code,
            "verification_uri": self.verification_uri,
            "interval": self.interval,
            "expires_at": self.expires_at,
            "device_code": self.device_code,
        })
        .to_string()
    }

    /// A kept code ([`DeviceCode::to_json`]) with at least a minute left; `None` for one that
    /// expired or is unreadable.
    pub fn from_json(text: &str) -> Option<Self> {
        let v: Value = serde_json::from_str(text).ok()?;
        let code = Self {
            user_code: str_field(&v, "user_code")?,
            verification_uri: str_field(&v, "verification_uri")?,
            interval: v.get("interval")?.as_u64()?.clamp(1, 60),
            expires_at: v.get("expires_at")?.as_u64()?,
            device_code: str_field(&v, "device_code")?,
        };
        (code.expires_at > now().saturating_add(60)).then_some(code)
    }
}

/// The Microsoft tokens; the access token only lasts for the Xbox step.
pub struct MsTokens {
    pub access: String,
    pub refresh: String,
}

impl std::fmt::Debug for MsTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MsTokens(<hidden>)")
    }
}

/// One poll of the token endpoint.
pub enum Poll {
    Pending,
    /// The server asked to poll less often: wait 5 s more.
    SlowDown,
    Done(MsTokens),
}

/// Asks Microsoft for a device code. `client_id` comes from [`crate::client_id`].
pub fn device_code(ep: &Endpoints, client_id: &str) -> Result<DeviceCode, String> {
    let (status, v) = send(
        agent().post(&format!("{}/devicecode", ep.microsoft)),
        Some(Body::Form(vec![
            ("client_id".into(), client_id.to_owned()),
            ("scope".into(), SCOPE.to_owned()),
        ])),
        "Microsoft",
    )?;
    parse_device_code(status, &v)
}

/// A device-code answer: the code to type, where to type it, and how long it lasts.
pub(crate) fn parse_device_code(status: u16, v: &Value) -> Result<DeviceCode, String> {
    match (
        status,
        str_field(v, "user_code"),
        str_field(v, "device_code"),
    ) {
        (200, Some(user_code), Some(device_code)) => Ok(DeviceCode {
            user_code,
            verification_uri: str_field(v, "verification_uri")
                .unwrap_or_else(|| "https://www.microsoft.com/link".into()),
            interval: v
                .get("interval")
                .and_then(Value::as_u64)
                .unwrap_or(5)
                .clamp(1, 60),
            // The service's numbers, bounded: now + a huge interval would overflow later.
            expires_at: now().saturating_add(
                v.get("expires_in")
                    .and_then(Value::as_u64)
                    .unwrap_or(900)
                    .min(3600),
            ),
            device_code,
        }),
        _ => Err(oauth_error(v)),
    }
}

/// The OAuth error codes as messages. `expired_token` and `authorization_declined` are terminal;
/// `authorization_pending` and `slow_down` are handled by [`poll`].
pub fn oauth_error(v: &Value) -> String {
    match v.get("error").and_then(Value::as_str) {
        Some("authorization_declined") => "Sign-in was declined in the browser.".into(),
        Some("expired_token") => "The sign-in code expired. Start again.".into(),
        Some("bad_verification_code") => "The sign-in code was not recognised. Start again.".into(),
        Some("invalid_grant") => "Your Microsoft sign-in expired. Sign in again.".into(),
        Some("invalid_client" | "unauthorized_client") => {
            "Microsoft refused this app's registration (client ID).".into()
        }
        Some(other) => format!("Microsoft sign-in failed ({other})."),
        None => "Microsoft sign-in failed.".into(),
    }
}

/// One poll of the token endpoint while the player signs in.
pub fn poll(ep: &Endpoints, client_id: &str, code: &DeviceCode) -> Result<Poll, String> {
    let (status, v) = send(
        agent().post(&format!("{}/token", ep.microsoft)),
        Some(Body::Form(vec![
            (
                "grant_type".into(),
                "urn:ietf:params:oauth:grant-type:device_code".into(),
            ),
            ("client_id".into(), client_id.to_owned()),
            ("device_code".into(), code.device_code.clone()),
        ])),
        "Microsoft",
    )?;
    match v.get("error").and_then(Value::as_str) {
        Some("authorization_pending") => return Ok(Poll::Pending),
        Some("slow_down") => return Ok(Poll::SlowDown),
        _ => {}
    }
    ms_tokens(status, &v).map(Poll::Done)
}

pub(crate) fn ms_tokens(status: u16, v: &Value) -> Result<MsTokens, String> {
    match (
        status,
        str_field(v, "access_token"),
        str_field(v, "refresh_token"),
    ) {
        (200, Some(access), Some(refresh)) => Ok(MsTokens { access, refresh }),
        _ => Err(oauth_error(v)),
    }
}

/// Waits for the player to finish in the browser, polling as asked, until the code expires or
/// `cancelled` says stop.
pub fn wait_for_sign_in(
    ep: &Endpoints,
    client_id: &str,
    code: &DeviceCode,
    cancelled: &dyn Fn() -> bool,
) -> Result<MsTokens, String> {
    let mut interval = code.interval;
    loop {
        let until = std::time::Instant::now() + Duration::from_secs(interval);
        while std::time::Instant::now() < until {
            if cancelled() {
                return Err("Sign-in cancelled.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if now() > code.expires_at {
            return Err("The sign-in code expired. Start again.".into());
        }
        match poll(ep, client_id, code)? {
            Poll::Pending => {}
            Poll::SlowDown => interval = (interval + 5).min(60),
            Poll::Done(tokens) => return Ok(tokens),
        }
    }
}

/// New Microsoft tokens for a kept refresh token. Microsoft can rotate the refresh credential;
/// the caller keeps the new one.
pub fn refresh_ms(ep: &Endpoints, client_id: &str, refresh: &str) -> Result<MsTokens, String> {
    let (status, v) = send(
        agent().post(&format!("{}/token", ep.microsoft)),
        Some(Body::Form(vec![
            ("grant_type".into(), "refresh_token".into()),
            ("client_id".into(), client_id.to_owned()),
            ("refresh_token".into(), refresh.to_owned()),
            ("scope".into(), SCOPE.to_owned()),
        ])),
        "Microsoft",
    )?;
    ms_tokens(status, &v)
}

/// The XSTS `XErr` codes, as plain messages: no Xbox profile, a child account, a region block.
pub fn xsts_error(code: u64) -> String {
    match code {
        2148916233 => {
            "This Microsoft account has no Xbox profile yet. Sign in once at xbox.com to create one, then try again.".into()
        }
        2148916235 => "Xbox Live is not available in your country or region.".into(),
        2148916236 | 2148916237 => {
            "This account needs adult verification on xbox.com before it can play.".into()
        }
        2148916238 => {
            "This is a child account: an adult has to add it to a Microsoft family first.".into()
        }
        other => format!("Xbox Live refused the sign-in (error {other})."),
    }
}

/// The Minecraft token from the Microsoft one, through Xbox Live and XSTS.
pub struct MinecraftLogin {
    pub token: String,
    pub expires_at: u64,
    pub xuid: Option<String>,
}

/// The Minecraft profile (no secrets).
pub struct Profile {
    /// Without dashes.
    pub uuid: String,
    pub name: String,
}

/// Xbox Live, XSTS, then the Minecraft services login (`login_with_xbox`).
pub fn minecraft_login(ep: &Endpoints, ms_access: &str) -> Result<MinecraftLogin, String> {
    let (status, xbl) = send(
        agent().post(&ep.xbox_user),
        Some(Body::Json(json!({
            "Properties": {
                "AuthMethod": "RPS",
                "SiteName": "user.auth.xboxlive.com",
                "RpsTicket": format!("d={ms_access}"),
            },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT",
        }))),
        "Xbox Live",
    )?;
    let xbl_token = str_field(&xbl, "Token")
        .filter(|_| status == 200)
        .ok_or("Xbox Live refused the sign-in.")?;
    let (status, xsts) = send(
        agent().post(&ep.xsts),
        Some(Body::Json(json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT",
        }))),
        "Xbox Live",
    )?;
    if let Some(code) = xsts.get("XErr").and_then(Value::as_u64) {
        return Err(xsts_error(code));
    }
    let claims = xsts.pointer("/DisplayClaims/xui/0");
    let (Some(xsts_token), Some(uhs)) = (
        str_field(&xsts, "Token").filter(|_| status == 200),
        claims.and_then(|c| str_field(c, "uhs")),
    ) else {
        return Err("Xbox Live refused the sign-in.".into());
    };
    let xuid = claims.and_then(|c| str_field(c, "xid"));
    let (status, mc) = send(
        agent().post(&format!("{}/authentication/login_with_xbox", ep.minecraft)),
        Some(Body::Json(
            json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }),
        )),
        "the Minecraft services",
    )?;
    match (status, str_field(&mc, "access_token")) {
        (200, Some(token)) => {
            let expires_in = mc
                .get("expires_in")
                .and_then(Value::as_u64)
                .unwrap_or(86_400)
                .min(7 * 86_400);
            Ok(MinecraftLogin {
                token,
                expires_at: now().saturating_add(expires_in),
                xuid,
            })
        }
        // New Azure apps are refused until Mojang approves them (Minecraft API access).
        (403, _) => Err(
            "The Minecraft services refused this app: its app is not approved by Mojang yet."
                .into(),
        ),
        (429, _) => Err("Too many sign-ins in a short time. Wait a minute and try again.".into()),
        _ => Err("The Minecraft services refused the sign-in.".into()),
    }
}

/// Whether the account owns Minecraft: Java Edition (Game Pass included).
pub fn owns_game(ep: &Endpoints, token: &str) -> Result<bool, String> {
    let (status, v) = send(
        agent()
            .get(&format!("{}/entitlements/mcstore", ep.minecraft))
            .set("Authorization", &format!("Bearer {token}")),
        None,
        "the Minecraft services",
    )?;
    if status != 200 {
        return Err("Could not check which games this account owns.".into());
    }
    Ok(owns_game_answer(&v))
}

/// An entitlements answer: whether it lists Minecraft: Java Edition (Game Pass included).
pub(crate) fn owns_game_answer(v: &Value) -> bool {
    v.get("items")
        .and_then(Value::as_array)
        .is_some_and(|items| {
            items.iter().any(|i| {
                matches!(
                    i.get("name").and_then(Value::as_str),
                    Some("game_minecraft" | "product_minecraft")
                )
            })
        })
}

/// The Minecraft profile; none when the account has none.
pub fn profile(ep: &Endpoints, token: &str) -> Result<Option<Profile>, String> {
    let (status, v) = send(
        agent()
            .get(&format!("{}/minecraft/profile", ep.minecraft))
            .set("Authorization", &format!("Bearer {token}")),
        None,
        "the Minecraft services",
    )?;
    if status == 404 {
        return Ok(None);
    }
    let (Some(uuid), Some(name)) = (str_field(&v, "id"), str_field(&v, "name")) else {
        return Err("Could not read the Minecraft profile.".into());
    };
    Ok(Some(Profile {
        uuid: uuid.replace('-', "").to_lowercase(),
        name,
    }))
}

/// From a Microsoft access token to a ready login and profile: Xbox, XSTS, Minecraft,
/// ownership, profile. The ownership check is Microsoft's owner gate; Ely.by has no Mojang
/// services and skips it.
pub fn finish_sign_in(
    ep: &Endpoints,
    ms_access: &str,
) -> Result<(MinecraftLogin, Profile), String> {
    let mc = minecraft_login(ep, ms_access)?;
    if !owns_game(ep, &mc.token)? {
        return Err("This account doesn't own Minecraft: Java Edition.".into());
    }
    let profile = profile(ep, &mc.token)?.ok_or(
        "This account has no Minecraft profile yet. Create one at minecraft.net, then sign in again.",
    )?;
    Ok((mc, profile))
}

/// The whole Microsoft sign-in from device code to a ready [`Account`]: polls until the player
/// finishes, then Xbox, XSTS, Minecraft, ownership and profile.
pub fn sign_in(
    ep: &Endpoints,
    client_id: &str,
    code: &DeviceCode,
    cancelled: &dyn Fn() -> bool,
) -> Result<Account, String> {
    let ms = wait_for_sign_in(ep, client_id, code, cancelled)?;
    let (mc, profile) = finish_sign_in(ep, &ms.access)?;
    Ok(Account {
        kind: Kind::Microsoft,
        name: profile.name,
        uuid: profile.uuid,
        xuid: mc.xuid,
        access_token: mc.token,
        refresh_secret: ms.refresh,
        client_token: String::new(),
        expires_at: mc.expires_at,
    })
}

/// Whether the account's Minecraft token needs renewing before it is used.
pub fn needs_renewal(account: &Account) -> bool {
    account.expires_at <= now() + RENEW_BEFORE
}
