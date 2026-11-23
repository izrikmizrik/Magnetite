//! The hooks a Lua script may name: the catalog's, generated from the hook declarations by
//! `build.rs`, and the subset this runtime carries to Lua today.

/// One hook of the catalog.
#[derive(Clone, Copy, Debug)]
pub(crate) struct CatalogHook {
    pub(crate) name: &'static str,
    pub(crate) version: u64,
    pub(crate) scoped: bool,
}

include!(concat!(env!("OUT_DIR"), "/lua_catalog.rs"));

/// The catalog hooks a script can handle, each with the contract version the runtime speaks:
/// a script handler is held to the same contract as a native or wasm mod's.
pub(crate) const BRIDGED: &[(&str, u64)] = &[
    // Global hooks: `horizon.on(name, fn)`.
    ("loot_table_load", 2),
    ("tags_updated", 1),
    // A block's Part: `horizon.block(id, { use_item_on = fn })`.
    ("use_item_on", 2),
    // A block entity type's Part: `horizon.block_entity(id, { block_entity_tick = fn })`.
    ("block_entity_tick", 2),
];

/// Why a script's hook name can't be used.
pub(crate) fn check(name: &str, scoped: bool) -> Result<(), String> {
    let Some(hook) = CATALOG.iter().find(|hook| hook.name == name) else {
        return Err(format!("{name} is not a hook"));
    };
    if hook.scoped != scoped {
        return Err(if hook.scoped {
            format!("{name} is a Part's hook: attach it with horizon.block or horizon.block_entity")
        } else {
            format!("{name} is a global hook: register it with horizon.on")
        });
    }
    match BRIDGED.iter().find(|(bridged, _)| *bridged == name) {
        Some((_, version)) if *version == hook.version => Ok(()),
        Some((_, version)) => Err(format!(
            "{name} is at contract version {} and Lua speaks {version}",
            hook.version
        )),
        None => Err(format!("{name} is not available to Lua scripts yet")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every hook the runtime carries is the catalog's, at the catalog's contract version: a
    /// change to one of them fails here until the Lua side follows.
    #[test]
    fn every_bridged_hook_is_in_the_catalog_at_its_version() {
        for (name, version) in BRIDGED {
            let hook = CATALOG
                .iter()
                .find(|hook| hook.name == *name)
                .unwrap_or_else(|| panic!("{name} left the catalog"));
            assert_eq!(hook.version, *version, "{name}");
        }
        assert!(check("use_item_on", true).is_ok());
        assert!(check("use_item_on", false).is_err());
        assert!(check("no_such_hook", false).is_err());
        assert!(CATALOG.len() > 700);
    }
}
