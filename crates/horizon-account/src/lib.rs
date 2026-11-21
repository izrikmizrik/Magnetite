//! Platform-neutral account sign-in for every Horizon front end (docs/unification/fork-feature-map.md
//! §2.4): the desktop launcher, the game on a phone, the future in-game screen. One crate for the
//! flows both share — Microsoft (OAuth device code, Xbox Live, XSTS, the Minecraft services,
//! ownership, profile) and Ely.by (Yggdrasil authenticate/refresh, with 2FA) — plus the session
//! file the game reads and a store that keeps only the refresh secret.
//!
//! The Azure app (client ID) is configuration, never hardcoded: `MAGNETITE_MSA_CLIENT_ID` at run
//! time, then at build time ([`client_id`]). Without one, Microsoft sign-in says it is not
//! configured. All URLs live in [`Endpoints`], whose [`Endpoints::live`] names the real services,
//! so tests point them at a mock server.
//!
//! Tokens never reach logs, errors or output: [`Account`]'s `Debug` hides them, and error strings
//! name the service and what went wrong, never a token or a password.

mod ely;
mod microsoft;
mod store;

pub use ely::{ElyTokens, NEEDS_2FA, REFUSED, ely_refresh, ely_sign_in};
pub use microsoft::{
    DeviceCode, MinecraftLogin, MsTokens, Poll, Profile, device_code, finish_sign_in,
    minecraft_login, needs_renewal, oauth_error, owns_game, poll, profile, refresh_ms, sign_in,
    wait_for_sign_in, xsts_error,
};
pub use store::{FileStore, SecretStore};

#[cfg(test)]
mod tests;

use serde_json::{Value, json};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The OAuth scope the device-code flow asks for.
pub const SCOPE: &str = "XboxLive.signin offline_access";

/// What sign-in says when this build has no Azure app ([`client_id`]).
pub const NOT_CONFIGURED: &str = "This build has no Microsoft sign-in (MAGNETITE_MSA_CLIENT_ID), so it can't sign in with Microsoft.";

/// The `user_type` of a session file for a Microsoft account, as the game's session parser
/// (`horizon-client-render`'s `auth.rs`) accepts it.
pub const MSA_USER_TYPE: &str = "msa";
/// The `user_type` of a session file for an Ely.by account.
pub const ELY_USER_TYPE: &str = "elyby";

/// The configured Azure app, if any: the run-time variable wins over the build-time one, and an
/// empty value means none.
pub fn client_id() -> Option<String> {
    std::env::var("MAGNETITE_MSA_CLIENT_ID")
        .ok()
        .or_else(|| option_env!("MAGNETITE_MSA_CLIENT_ID").map(str::to_owned))
        .map(|id| id.trim().to_owned())
        .filter(|id| !id.is_empty())
}

/// Where the requests go: the real services, or a mock in tests.
#[derive(Clone, Debug)]
pub struct Endpoints {
    /// The OAuth 2.0 v2 endpoints for personal accounts (`…/devicecode`, `…/token`).
    pub microsoft: String,
    pub xbox_user: String,
    pub xsts: String,
    /// api.minecraftservices.com.
    pub minecraft: String,
    /// authserver.ely.by.
    pub ely: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self::live()
    }
}

impl Endpoints {
    /// The real services.
    pub fn live() -> Self {
        Self {
            microsoft: "https://login.microsoftonline.com/consumers/oauth2/v2.0".into(),
            xbox_user: "https://user.auth.xboxlive.com/user/authenticate".into(),
            xsts: "https://xsts.auth.xboxlive.com/xsts/authorize".into(),
            minecraft: "https://api.minecraftservices.com".into(),
            ely: "https://authserver.ely.by".into(),
        }
    }
}

/// Which provider signed the account in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Microsoft,
    Ely,
}

impl Kind {
    /// The session file's `user_type` for this kind.
    pub fn user_type(self) -> &'static str {
        match self {
            Kind::Microsoft => MSA_USER_TYPE,
            Kind::Ely => ELY_USER_TYPE,
        }
    }
}

/// A signed-in account. The access token lives in memory only; only the refresh secret is ever
/// kept (in a [`SecretStore`]), and the store's JSON holds no access token.
pub struct Account {
    pub kind: Kind,
    /// The player name.
    pub name: String,
    /// The profile UUID without dashes.
    pub uuid: String,
    /// The Xbox user id (Microsoft accounts only).
    pub xuid: Option<String>,
    /// The game session token (Microsoft's Minecraft access token, Ely.by's access token).
    access_token: String,
    /// What renews `access_token`: the Microsoft refresh token, or Ely.by's access token.
    refresh_secret: String,
    /// The Ely.by client token the access token belongs to (Microsoft accounts have none).
    client_token: String,
    /// The access token's expiry, epoch seconds.
    pub expires_at: u64,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("uuid", &self.uuid)
            .field("xuid", &self.xuid)
            .field("access_token", &"<hidden>")
            .field("refresh_secret", &"<hidden>")
            .field("expires_at", &self.expires_at)
            .finish()
    }
}

impl Account {
    /// For tests and harnesses.
    pub fn new(
        kind: Kind,
        name: &str,
        uuid: &str,
        access_token: &str,
        refresh_secret: &str,
        expires_at: u64,
    ) -> Self {
        Self {
            kind,
            name: name.to_owned(),
            uuid: uuid.to_owned(),
            xuid: None,
            access_token: access_token.to_owned(),
            refresh_secret: refresh_secret.to_owned(),
            client_token: String::new(),
            expires_at,
        }
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    /// The secret a [`SecretStore`] keeps to sign in again without the player.
    pub fn refresh_secret(&self) -> &str {
        &self.refresh_secret
    }

    /// The Ely.by client token (empty for Microsoft accounts).
    pub fn client_token(&self) -> &str {
        &self.client_token
    }

    /// The session file's JSON, in the format the game's `AuthSession::parse` reads:
    /// `{"name", "uuid", "access_token", "user_type", "xuid"}`.
    pub fn session_json(&self) -> String {
        let mut v = json!({
            "name": self.name,
            "uuid": self.uuid,
            "access_token": self.access_token,
            "user_type": self.kind.user_type(),
        });
        if let Some(xuid) = &self.xuid {
            v["xuid"] = xuid.clone().into();
        }
        v.to_string()
    }

    /// What a [`SecretStore`] keeps for this account: the refresh secret, the kind, the name and
    /// the uuid — never the access token. Loaded again, it signs in through
    /// [`refresh_account`].
    pub fn stored_json(&self) -> String {
        json!({
            "kind": self.kind,
            "name": self.name,
            "uuid": self.uuid,
            "refresh_secret": self.refresh_secret,
            "client_token": self.client_token,
        })
        .to_string()
    }

    /// The account a stored JSON names, without signing in. The `refresh_secret` is what
    /// [`refresh_account`] needs; the access token is gone (it was never stored) and the new one
    /// comes from the provider.
    pub fn from_stored_json(text: &str) -> Result<Stored, String> {
        let v: Value = serde_json::from_str(text)
            .map_err(|_| "the stored account is not valid JSON".to_string())?;
        let field = |k: &str| v.get(k).and_then(Value::as_str);
        let kind = match field("kind") {
            Some("microsoft") => Kind::Microsoft,
            Some("ely") | Some(ELY_USER_TYPE) => Kind::Ely,
            _ => return Err("the stored account has no known kind".into()),
        };
        Ok(Stored {
            kind,
            name: field("name")
                .filter(|s| !s.is_empty())
                .ok_or("the stored account has no name")?
                .to_owned(),
            uuid: field("uuid")
                .filter(|s| !s.is_empty())
                .ok_or("the stored account has no uuid")?
                .to_owned(),
            refresh_secret: field("refresh_secret")
                .filter(|s| !s.is_empty())
                .ok_or("the stored account has no refresh secret")?
                .to_owned(),
            client_token: field("client_token").unwrap_or_default().to_owned(),
        })
    }
}

/// An account as a store keeps it: everything but the access token.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stored {
    pub kind: Kind,
    pub name: String,
    pub uuid: String,
    pub refresh_secret: String,
    /// The Ely.by client token (empty for Microsoft accounts).
    pub client_token: String,
}

/// Signs a stored account in again, without the player: Microsoft through its refresh token,
/// Ely.by through its access token and client token.
pub fn refresh_account(ep: &Endpoints, stored: &Stored) -> Result<Account, String> {
    match stored.kind {
        Kind::Microsoft => {
            let client_id = client_id().ok_or_else(|| NOT_CONFIGURED.to_string())?;
            let ms = refresh_ms(ep, &client_id, &stored.refresh_secret)?;
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
        Kind::Ely => ely_refresh(
            ep,
            &stored.refresh_secret,
            &stored.client_token,
            &stored.name,
            &stored.uuid,
        ),
    }
}

/// The agent sign-in requests start from: a 20 s timeout and a `horizon-account` user agent.
pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .user_agent(concat!("horizon-account/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// One request's result: the status and the JSON body (null when there is none). Transport
/// errors name the service, never the URL's query or the request.
pub(crate) fn send(
    request: ureq::Request,
    body: Option<Body>,
    service: &str,
) -> Result<(u16, Value), String> {
    let result = match body {
        None => request.call(),
        Some(Body::Form(fields)) => {
            let refs: Vec<(&str, &str)> = fields
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            request.send_form(&refs)
        }
        Some(Body::Json(value)) => request
            .set("Content-Type", "application/json")
            .set("Accept", "application/json")
            .send_string(&value.to_string()),
    };
    let response = match result {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(ureq::Error::Transport(_)) => {
            return Err(format!(
                "Could not reach {service}. Check your internet connection."
            ));
        }
    };
    let status = response.status();
    let text = response.into_string().unwrap_or_default();
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

pub(crate) enum Body {
    Form(Vec<(String, String)>),
    Json(Value),
}

pub(crate) fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// Epoch seconds now.
pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
