//! The accounts a player keeps on one device, and which one plays: what a [`crate::SecretStore`]
//! holds as one JSON document, `{"current": i, "accounts": [...]}`, each account as
//! [`crate::Account::stored_json`] writes it. A store written before there could be more than one
//! holds a single account; it reads as a keyring of that one.
use crate::{Account, Stored};
use serde_json::{Value, json};

/// The kept accounts and the one that plays.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keyring {
    pub accounts: Vec<Stored>,
    /// An index into `accounts`; `None` with none kept.
    pub current: Option<usize>,
}

impl Keyring {
    /// A store's text. Accounts that no longer read are left out (with the reason, for the log);
    /// the current one falls back to the first that reads.
    pub fn parse(text: &str) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let Ok(v) = serde_json::from_str::<Value>(text) else {
            return (
                Self::default(),
                vec!["the account store is not valid JSON".into()],
            );
        };
        let (items, current) = match v.get("accounts").and_then(Value::as_array) {
            Some(items) => (
                items.clone(),
                v.get("current").and_then(Value::as_u64).map(|i| i as usize),
            ),
            // Before the keyring: the one account itself.
            None => (vec![v], Some(0)),
        };
        let mut accounts = Vec::new();
        let mut kept_current = None;
        for (i, item) in items.iter().enumerate() {
            match Account::from_stored_json(&item.to_string()) {
                Ok(stored) => {
                    if current == Some(i) {
                        kept_current = Some(accounts.len());
                    }
                    accounts.push(stored);
                }
                Err(e) => problems.push(e),
            }
        }
        let current = kept_current.or((!accounts.is_empty()).then_some(0));
        (Self { accounts, current }, problems)
    }

    /// The store's text for these accounts.
    pub fn to_json(&self) -> String {
        let accounts: Vec<Value> = self
            .accounts
            .iter()
            .map(|s| {
                json!({
                    "kind": s.kind,
                    "name": s.name,
                    "uuid": s.uuid,
                    "refresh_secret": s.refresh_secret,
                    "client_token": s.client_token,
                })
            })
            .collect();
        json!({ "current": self.current, "accounts": accounts }).to_string()
    }

    /// The account that plays.
    pub fn current(&self) -> Option<&Stored> {
        self.current.and_then(|i| self.accounts.get(i))
    }

    /// Keeps `stored` (a new sign-in or a renewed one: the same kind and uuid replaces the old
    /// entry) and makes it the one that plays.
    pub fn keep(&mut self, stored: Stored) {
        let at = self
            .accounts
            .iter()
            .position(|s| s.kind == stored.kind && s.uuid == stored.uuid);
        let i = match at {
            Some(i) => {
                self.accounts[i] = stored;
                i
            }
            None => {
                self.accounts.push(stored);
                self.accounts.len() - 1
            }
        };
        self.current = Some(i);
    }

    /// Forgets the account at `i`; the current one stays current (the first, if it was this one).
    pub fn remove(&mut self, i: usize) {
        if i >= self.accounts.len() {
            return;
        }
        self.accounts.remove(i);
        self.current = match self.current {
            _ if self.accounts.is_empty() => None,
            Some(c) if c == i => Some(0),
            Some(c) if c > i => Some(c - 1),
            other => other,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::Keyring;
    use crate::{Kind, Stored};

    fn stored(name: &str, uuid: &str) -> Stored {
        Stored {
            kind: Kind::Microsoft,
            name: name.into(),
            uuid: uuid.into(),
            refresh_secret: "r".into(),
            client_token: String::new(),
        }
    }

    #[test]
    fn a_single_account_store_reads_as_a_keyring_of_one() {
        let old = r#"{"kind":"microsoft","name":"Steve","uuid":"u1","refresh_secret":"r","client_token":""}"#;
        let (ring, problems) = Keyring::parse(old);
        assert!(problems.is_empty());
        assert_eq!(ring.accounts, vec![stored("Steve", "u1")]);
        assert_eq!(ring.current, Some(0));
    }

    #[test]
    fn keeping_replaces_the_same_account_and_makes_it_current() {
        let mut ring = Keyring::default();
        ring.keep(stored("Steve", "u1"));
        ring.keep(stored("Alex", "u2"));
        ring.keep(stored("Steve2", "u1"));
        assert_eq!(ring.accounts.len(), 2);
        assert_eq!(ring.current().map(|s| s.name.as_str()), Some("Steve2"));
        let (back, _) = Keyring::parse(&ring.to_json());
        assert_eq!(back, ring);
    }

    #[test]
    fn removing_keeps_the_current_one_where_it_can() {
        let mut ring = Keyring::default();
        for (n, u) in [("A", "1"), ("B", "2"), ("C", "3")] {
            ring.keep(stored(n, u));
        }
        ring.current = Some(2);
        ring.remove(0);
        assert_eq!(ring.current().map(|s| s.name.as_str()), Some("C"));
        ring.remove(1);
        assert_eq!(ring.current().map(|s| s.name.as_str()), Some("B"));
        ring.remove(0);
        assert_eq!(ring.current, None);
    }
}
