//! Ely.by's documented Yggdrasil API (https://docs.ely.by/en/minecraft-auth.html): authenticate
//! with a login and password (a two-factor account adds the code to the password as
//! `password:otp`), refresh with the kept access and client tokens. There is no Minecraft
//! ownership check: Ely.by has no Mojang services.
//!
//! The password is used for one request only: never persisted, never in an error, never sent to
//! the game.

use crate::{Account, Body, Endpoints, Kind, agent, now, send, str_field};
use serde_json::{Value, json};

/// What Ely.by says when it refuses the credentials.
pub const REFUSED: &str = "Ely.by sign-in was refused. Check your credentials or sign in again.";
/// What Ely.by says when the account has two-factor sign-in and no code was given.
pub const NEEDS_2FA: &str = "Enter your Ely.by two-factor code and try again.";

/// The tokens of an Ely.by answer: the access token, and the client token it belongs to.
pub struct ElyTokens {
    pub access_token: String,
    pub client_token: String,
    pub name: String,
    /// Without dashes.
    pub uuid: String,
}

impl std::fmt::Debug for ElyTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElyTokens")
            .field("name", &self.name)
            .field("uuid", &self.uuid)
            .field("access_token", &"<hidden>")
            .field("client_token", &"<hidden>")
            .finish()
    }
}

/// What Ely.by says when it refuses: its own message, or the status.
pub(crate) fn ely_error(status: u16, body: &Value) -> String {
    str_field(body, "errorMessage")
        .unwrap_or_else(|| format!("Ely.by refused the sign-in ({status})."))
}

/// One POST to an Ely.by endpoint: 200's JSON, or the mapped error. A 401 with the two-factor
/// message becomes [`NEEDS_2FA`].
fn exchange(ep: &Endpoints, endpoint: &str, body: Value) -> Result<Value, String> {
    let (status, value) = send(
        agent().post(&format!("{}/{endpoint}", ep.ely)),
        Some(Body::Json(body)),
        "Ely.by",
    )?;
    match status {
        200 => Ok(value),
        401 if value["errorMessage"].as_str()
            == Some("Account protected with two factor auth.") =>
        {
            Err(NEEDS_2FA.into())
        }
        400 | 401 | 403 => Err(REFUSED.into()),
        429 => Err("Ely.by is receiving too many requests. Try again later.".into()),
        other => Err(ely_error(other, &value)),
    }
}

/// A player's name and id from an Ely.by answer's `selectedProfile`, validated like the launcher
/// does: 32 hex digits, and a 1-16 character alphanumeric name.
pub(crate) fn decode_profile(value: &Value) -> Result<(String, String), String> {
    let profile = &value["selectedProfile"];
    let uuid = str_field(profile, "id")
        .unwrap_or_default()
        .replace('-', "")
        .to_lowercase();
    let name = str_field(profile, "name").unwrap_or_default();
    if uuid.len() != 32
        || !uuid.bytes().all(|b| b.is_ascii_hexdigit())
        || !(1..=16).contains(&name.len())
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        return Err("Ely.by returned an invalid Minecraft profile.".into());
    }
    Ok((name, uuid))
}

pub(crate) fn decode(value: &Value, client: &str) -> Result<ElyTokens, String> {
    let token = str_field(value, "accessToken")
        .filter(|t| !t.is_empty())
        .ok_or("Ely.by returned no game session.")?;
    // The answer must belong to the client token we sent: it identifies this install.
    if value
        .get("clientToken")
        .and_then(Value::as_str)
        .is_some_and(|t| t != client)
    {
        return Err("Ely.by returned a different client identity.".into());
    }
    let (name, uuid) = decode_profile(value)?;
    Ok(ElyTokens {
        access_token: token,
        client_token: client.to_owned(),
        name,
        uuid,
    })
}

/// A fresh client token: 16 random bytes as hex, as Yggdrasil expects.
fn new_client_token() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| "Could not create a client identity.".to_string())?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Signs an Ely.by account in with its login (name or e-mail) and password. `otp` is the
/// two-factor code, empty when the account has none: Ely.by expects it as `password:otp`.
pub fn ely_sign_in(
    ep: &Endpoints,
    username: &str,
    password: &str,
    otp: &str,
) -> Result<Account, String> {
    if username.trim().is_empty() || password.is_empty() {
        return Err("Enter your Ely.by username and password.".into());
    }
    if !otp.is_empty() && (otp.len() != 6 || !otp.bytes().all(|b| b.is_ascii_digit())) {
        return Err("The two-factor code must contain six digits.".into());
    }
    let client = new_client_token()?;
    let password = if otp.is_empty() {
        password.to_owned()
    } else {
        format!("{password}:{otp}")
    };
    let value = exchange(
        ep,
        "auth/authenticate",
        json!({
            "username": username.trim(),
            "password": password,
            "clientToken": client,
            "requestUser": false,
        }),
    )?;
    let tokens = decode(&value, &client)?;
    to_account(tokens, 3600)
}

/// A new Ely.by access token for a kept one. The refresh answer may leave the profile out: the
/// saved name and uuid stand.
pub fn ely_refresh(
    ep: &Endpoints,
    access_token: &str,
    client_token: &str,
    saved_name: &str,
    saved_uuid: &str,
) -> Result<Account, String> {
    let value = exchange(
        ep,
        "auth/refresh",
        json!({
            "accessToken": access_token,
            "clientToken": client_token,
            "selectedProfile": {"id": saved_uuid, "name": saved_name},
        }),
    )?;
    let (name, uuid) = match value.get("selectedProfile") {
        Some(_) => decode_profile(&value)?,
        None => (saved_name.to_owned(), saved_uuid.to_owned()),
    };
    if uuid != saved_uuid {
        return Err("Ely.by returned another profile. Sign in again.".into());
    }
    let mut tokens = decode(&value, client_token)?;
    tokens.name = name;
    tokens.uuid = uuid;
    to_account(tokens, 3600)
}

fn to_account(tokens: ElyTokens, expires_in: u64) -> Result<Account, String> {
    Ok(Account {
        kind: Kind::Ely,
        name: tokens.name,
        uuid: tokens.uuid,
        xuid: None,
        access_token: tokens.access_token.clone(),
        refresh_secret: tokens.access_token,
        client_token: tokens.client_token,
        expires_at: now().saturating_add(expires_in),
    })
}
