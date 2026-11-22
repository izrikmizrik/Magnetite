//! The `block_entity_collect_components` hook: declarations the mod API generator reads (see `hooks/mod.rs`).

pub trait Hooks {
    /// The item components the block entity at `pos` holds in its own data
    /// when its `id` is yours (`BlockEntity.collectImplicitComponents`): set
    /// `components` to them, read from `entity` alone (a pot's meal, a
    /// cabinet's contents). Asked on the server when a loot roll copies the
    /// broken block's components into its drop (`minecraft:copy_components`
    /// with `"source": "block_entity"`); the roll keeps those the function's
    /// `include` and `exclude` allow, over the block entity's own saved
    /// `components`. `entity` is the block entity as it was before the break.
    /// Leave it unset to add none.
    #[hook(
        side = "server",
        contract = "replace",
        out = "components",
        scope = "block_entity",
        key = "entity",
        neoforge = "none (vanilla BlockEntity.collectImplicitComponents; NeoForge adds no event or extension for it)"
    )]
    fn block_entity_collect_components();
}
