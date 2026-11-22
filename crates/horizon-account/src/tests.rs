//! Response-parsing tests (no network: the recorded answer shapes, mock tokens only), the
//! session file for both kinds, and the FileStore round-trip in a temporary folder.

use super::*;
use serde_json::{Value, json};

const UUID: &str = "069a79f444e94726a5befca90e38aaf5";

// ----- Microsoft answer parsing ---------------------------------------------------------------

#[test]
fn device_code_answer() {
    let v = json!({
        "user_code": "ABCD-EFGH",
        "device_code": "dev-1",
        "verification_uri": "https://www.microsoft.com/link",
        "expires_in": 900,
        "interval": 5,
    });
    let code = microsoft::parse_device_code(200, &v).unwrap();
    assert_eq!(code.user_code, "ABCD-EFGH");
    assert_eq!(code.verification_uri, "https://www.microsoft.com/link");
    assert_eq!(code.interval, 5);
    assert!(code.expires_at > now() + 800);
    assert!(!format!("{code:?}").contains("dev-1"));
    // Defaults and bounds: no interval, a huge expires_in.
    let v = json!({"user_code": "C", "device_code": "d", "expires_in": 10_000_000});
    let code = microsoft::parse_device_code(200, &v).unwrap();
    assert_eq!(code.interval, 5);
    assert_eq!(code.expires_at, now().saturating_add(3600));
    assert_eq!(code.verification_uri, "https://www.microsoft.com/link");
    // An error answer.
    assert!(microsoft::parse_device_code(400, &json!({"error": "invalid_client"})).is_err());
}

#[test]
fn a_kept_device_code_comes_back_until_it_is_about_to_expire() {
    let v =
        json!({"user_code": "ABCD-EFGH", "device_code": "dev-1", "expires_in": 900, "interval": 5});
    let code = microsoft::parse_device_code(200, &v).unwrap();
    let back = DeviceCode::from_json(&code.to_json()).unwrap();
    assert_eq!(back, code);
    // Under a minute left: the next start asks for a new code instead.
    let v = json!({"user_code": "C", "device_code": "d", "expires_in": 30});
    let short = microsoft::parse_device_code(200, &v).unwrap();
    assert!(DeviceCode::from_json(&short.to_json()).is_none());
    assert!(DeviceCode::from_json("not json").is_none());
}

#[test]
fn token_answer_and_poll_errors() {
    let tokens = microsoft::ms_tokens(
        200,
        &json!({"access_token": "a", "refresh_token": "r", "expires_in": 3600}),
    )
    .unwrap();
    assert_eq!(
        (tokens.access.as_str(), tokens.refresh.as_str()),
        ("a", "r")
    );
    assert!(!format!("{tokens:?}").contains("a"));
    for (error, message) in [
        ("authorization_pending", "pending"),
        ("slow_down", "slow"),
        ("authorization_declined", "declined"),
        ("expired_token", "expired"),
        ("invalid_grant", "expired. Sign in again"),
        ("invalid_client", "client ID"),
        ("something_else", "something_else"),
    ] {
        let err = microsoft::oauth_error(&json!({"error": error}));
        assert!(
            err.contains(message),
            "error {error} should mention {message:?}, got {err:?}"
        );
    }
    assert_eq!(
        microsoft::oauth_error(&json!({})),
        "Microsoft sign-in failed.".to_string()
    );
}

#[test]
fn xsts_errors_are_plain_messages() {
    assert!(xsts_error(2148916233).contains("no Xbox profile"));
    assert!(xsts_error(2148916235).contains("country or region"));
    assert!(xsts_error(2148916236).contains("adult verification"));
    assert!(xsts_error(2148916237).contains("adult verification"));
    assert!(xsts_error(2148916238).contains("child account"));
    assert!(xsts_error(2148916999).contains("2148916999"));
}

#[test]
fn entitlements_answer() {
    let owns = json!({"items": [
        {"name": "product_minecraft", "signature": "x"},
        {"name": "game_minecraft", "signature": "x"},
    ]});
    assert!(microsoft::owns_game_answer(&owns));
    let game_pass_only = json!({"items": [{"name": "product_minecraft"}]});
    assert!(microsoft::owns_game_answer(&game_pass_only));
    assert!(!microsoft::owns_game_answer(&json!({"items": []})));
    assert!(!microsoft::owns_game_answer(
        &json!({"items": [{"name": "product_gamepass"}]})
    ));
    assert!(!microsoft::owns_game_answer(&json!({})));
}

// ----- Ely.by answer parsing ------------------------------------------------------------------

#[test]
fn ely_decode_and_errors() {
    let value = json!({
        "accessToken": "ely-access",
        "clientToken": "client-1",
        "selectedProfile": {"id": "069A79F4-44E9-4726-A5BE-FCA90E38AAF5", "name": "Player"},
    });
    let tokens = ely::decode(&value, "client-1").unwrap();
    assert_eq!(tokens.access_token, "ely-access");
    assert_eq!(tokens.client_token, "client-1");
    assert_eq!(tokens.uuid, UUID);
    assert_eq!(tokens.name, "Player");
    assert!(!format!("{tokens:?}").contains("ely-access"));
    // A different client token: refused (the install's identity check).
    assert!(ely::decode(&value, "client-2").is_err());
    // No access token.
    assert!(
        ely::decode(
            &json!({"clientToken": "c", "selectedProfile": {"id": UUID, "name": "P"}}),
            "c"
        )
        .is_err()
    );
    // A malformed profile: path traversal in the id, bad name.
    assert!(ely::decode(&json!({"accessToken": "x", "clientToken": "c", "selectedProfile": {"id": "../bad", "name": "Player"}}), "c").is_err());
    assert!(ely::decode(&json!({"accessToken": "x", "clientToken": "c", "selectedProfile": {"id": UUID, "name": "bad name!"}}), "c").is_err());
    // The two-factor message maps to its own text.
    assert_eq!(
        ely::ely_error(
            401,
            &json!({"errorMessage": "Account protected with two factor auth."})
        ),
        "Account protected with two factor auth."
    );
    assert_eq!(
        ely::ely_error(403, &json!({})),
        "Ely.by refused the sign-in (403)."
    );
}

// ----- The session file and the stored account -------------------------------------------------

#[test]
fn session_json_for_both_kinds() {
    let ms = Account::new(
        Kind::Microsoft,
        "Notch",
        UUID,
        "mc-token",
        "ms-refresh",
        now() + 86400,
    );
    let v: Value = serde_json::from_str(&ms.session_json()).unwrap();
    assert_eq!(v["name"], "Notch");
    assert_eq!(v["uuid"], UUID);
    assert_eq!(v["access_token"], "mc-token");
    assert_eq!(v["user_type"], "msa");
    assert!(v.get("xuid").is_none());

    let ely = Account::new(
        Kind::Ely,
        "Player",
        UUID,
        "ely-token",
        "ely-token",
        now() + 3600,
    );
    let v: Value = serde_json::from_str(&ely.session_json()).unwrap();
    assert_eq!(v["user_type"], "elyby");
    assert_eq!(v["access_token"], "ely-token");
}

#[test]
fn stored_json_keeps_no_access_token_and_round_trips() {
    let account = Account::new(
        Kind::Ely,
        "Player",
        UUID,
        "ely-access",
        "ely-refresh",
        now() + 3600,
    );
    let v: Value = serde_json::from_str(&account.stored_json()).unwrap();
    assert_eq!(v["kind"], "ely");
    assert_eq!(v["refresh_secret"], "ely-refresh");
    assert!(
        v.get("access_token").is_none(),
        "the store must never hold the access token"
    );
    let stored = Account::from_stored_json(&account.stored_json()).unwrap();
    assert_eq!(
        stored,
        Stored {
            kind: Kind::Ely,
            name: "Player".into(),
            uuid: UUID.into(),
            refresh_secret: "ely-refresh".into(),
            client_token: String::new(),
        }
    );
    // Microsoft: kind "microsoft".
    let ms = Account::new(Kind::Microsoft, "Notch", UUID, "mc", "ms-refresh", 0);
    let stored = Account::from_stored_json(&ms.stored_json()).unwrap();
    assert_eq!(stored.kind, Kind::Microsoft);
    assert_eq!(stored.refresh_secret, "ms-refresh");
    // Broken stored JSON.
    assert!(Account::from_stored_json("not json").is_err());
    assert!(Account::from_stored_json(&json!({"kind": "steam"}).to_string()).is_err());
    assert!(
        Account::from_stored_json(&json!({"kind": "ely", "name": "P", "uuid": UUID}).to_string())
            .is_err()
    );
}

#[test]
fn account_debug_hides_secrets() {
    let account = Account::new(Kind::Microsoft, "Notch", UUID, "mc-token", "ms-refresh", 0);
    let debug = format!("{account:?}");
    assert!(!debug.contains("mc-token"));
    assert!(!debug.contains("ms-refresh"));
    assert!(debug.contains("Notch"));
}

// ----- FileStore --------------------------------------------------------------------------------

#[test]
fn file_store_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("account.json");
    let store = FileStore::new(&path);

    // Nothing stored yet.
    assert_eq!(store.load().unwrap(), None);

    let account = Account::new(
        Kind::Microsoft,
        "Notch",
        UUID,
        "mc-token",
        "ms-refresh",
        now() + 86400,
    );
    store.save(&account.stored_json()).unwrap();
    let loaded = store.load().unwrap().unwrap();
    let stored = Account::from_stored_json(&loaded).unwrap();
    assert_eq!(stored.uuid, UUID);
    assert_eq!(stored.refresh_secret, "ms-refresh");
    assert!(
        !loaded.contains("mc-token"),
        "the access token must not reach the file"
    );

    // Overwrite: the new value wins, atomically.
    let renewed = Account::new(
        Kind::Microsoft,
        "Notch",
        UUID,
        "mc-token-2",
        "ms-refresh-2",
        now() + 86400,
    );
    store.save(&renewed.stored_json()).unwrap();
    let stored = Account::from_stored_json(&store.load().unwrap().unwrap()).unwrap();
    assert_eq!(stored.refresh_secret, "ms-refresh-2");

    // No temporary files are left behind.
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".tmp"))
        .collect();
    assert!(left.is_empty(), "temporary files left behind: {left:?}");

    // Clear.
    store.clear().unwrap();
    assert_eq!(store.load().unwrap(), None);
    store.clear().unwrap(); // clearing again is fine

    #[cfg(unix)]
    {
        store.save(&renewed.stored_json()).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "the file must be 0600 on Unix");
    }
}

#[test]
fn file_store_in_a_new_folder() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("dir").join("account.json");
    let store = FileStore::new(&path);
    store.save("{}").unwrap();
    assert_eq!(store.load().unwrap().as_deref(), Some("{}"));
}
