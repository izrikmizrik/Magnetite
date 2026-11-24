//! Whether a newer Magnetite is released (the launcher's Updates page): the project's releases on
//! GitHub, the newest against this build's own version.
use serde_json::Value;

/// This build's version: the release tag's, as the workflows give it, or the crate's own.
pub const VERSION: &str = match option_env!("MAGNETITE_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};
/// The project's page.
pub const PROJECT: &str = "https://github.com/izrikmizrik/Magnetite";
/// The releases, newest first; pre-releases too, which is what the project makes so far.
pub const RELEASES: &str =
    "https://api.github.com/repos/izrikmizrik/Magnetite/releases?per_page=20";
/// Larger than that list should be.
pub const MAX_RELEASES: u64 = 2 << 20;

/// A release: its version and its page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub page: String,
}

/// The newest of GitHub's `releases` whose tag is a version (`v0.1.2`); none when there is none.
pub fn newest(releases: &str) -> Result<Option<Release>, String> {
    let value: Value = serde_json::from_str(releases).map_err(|e| e.to_string())?;
    let list = value
        .as_array()
        .ok_or("the list of releases is not a list")?;
    Ok(list
        .iter()
        .filter(|r| r["draft"] != true)
        .filter_map(|r| {
            let version = r["tag_name"].as_str()?.strip_prefix('v')?;
            number(version)?;
            // Only the project's own pages are ever opened.
            let page = r["html_url"].as_str().filter(|u| u.starts_with(PROJECT))?;
            Some(Release {
                version: version.to_owned(),
                page: page.to_owned(),
            })
        })
        .max_by_key(|r| number(&r.version)))
}

/// `1.2.3`, and whether it is the release itself rather than a build before it (`1.2.3-dev.4`).
fn number(version: &str) -> Option<([u32; 3], bool)> {
    let (plain, before) = match version.split_once('-') {
        Some((plain, _)) => (plain, true),
        None => (version, false),
    };
    let mut parts = plain.split('.').map(|n| n.parse().ok());
    let out = [
        parts.next()??,
        parts.next()??,
        parts.next().unwrap_or(Some(0))?,
    ];
    parts.next().is_none().then_some((out, !before))
}

/// Whether `release` is newer than `current`. A build whose version is no number never knows.
pub fn is_newer(release: &str, current: &str) -> bool {
    match (number(release), number(current)) {
        (Some(release), Some(current)) => release > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_release_is_newer_than_the_builds_before_it() {
        assert!(is_newer("0.1.2", "0.1.1"));
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("0.10.0", "0.9.0"));
        assert!(is_newer("0.1.0", "0.1.0-dev.33"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("0.1.2", "local"));
    }

    #[test]
    fn the_newest_version_is_taken_from_the_list() {
        let list = serde_json::json!([
            {"tag_name": "v0.1.1", "draft": false, "html_url": format!("{PROJECT}/releases/tag/v0.1.1")},
            {"tag_name": "v0.3.0", "draft": true, "html_url": format!("{PROJECT}/releases/tag/v0.3.0")},
            {"tag_name": "tables-1", "draft": false, "html_url": format!("{PROJECT}/releases/tag/tables-1")},
            {"tag_name": "v0.2.0", "draft": false, "html_url": "https://example.com/v0.2.0"},
            {"tag_name": "v0.1.10", "draft": false, "html_url": format!("{PROJECT}/releases/tag/v0.1.10")},
        ]);
        let found = newest(&list.to_string()).unwrap().unwrap();
        assert_eq!(found.version, "0.1.10");
        assert_eq!(newest("[]").unwrap(), None);
        assert!(newest("{}").is_err());
    }
}
