# Data files

A mod has one top-level `data/` tree and one `assets/` tree, with the same namespace and
content checks as a content mod. They are normal pack resources, not a second native-only data folder.
[Server data reload listeners](https://github.com/horizon-rust/horizon/blob/main/crates/horizon-mod-vocab/src/hooks/notes.rs#L2009) register through
`Hooks::add_server_reload_listeners`. Their `ServerReloadContext` lists namespaced IDs and reads the
winning bytes from selected datapacks and the mod's own `data/`. Parse once in prepare and publish in
apply, not on the tick. Server listener applies run before `data_pack_files`. A later refusal
keeps the old engine tables and Parts, but cannot undo effects of listeners or hooks that already
ran. Contributions share a list, so a failed file cannot be attributed to a particular mod.
The original pack trees remain separate from the launcher's code-only horizon-mixins input.

Client-side asset reads use [client asset reload listeners](https://github.com/horizon-rust/horizon/blob/main/crates/horizon-mod-vocab/src/hooks/notes.rs#L2103); server
listeners cannot read `assets/`. Packaged assets also participate in the game's normal resource
packs.

Outside a listener, `host.resource(pack_type, &identifier)` reads one file from the selected
stack: `PackType::ServerData` takes `mymod:machines/furnace.json` from `data/`, and
`PackType::ClientResources` takes `mymod:models/block/furnace.json` from `assets/`. As with
`ResourceManager.getResource`, the highest selected pack that supplies the file wins; filters,
a world's datapacks and a mod's own root data all apply. The answer is the winning bytes, or
`None` for a missing, unreadable or over-64-MiB file. Use a reload listener for a whole folder
or for parsing at reload time; a single read does not reload packs. Directory-pack
symlinks follow vanilla pack behavior: linked resource files are read through the pack.

## Tables built from the game's recipes

A machine whose recipes the game parses (`host.recipes_of_type("mymod:cooking")`, ingredients and
results already resolved, the world's packs merged and load conditions held) builds its table
from the game's recipe table, not from the files. A reload listener can't do that: its prepare
sees the new files, and the game's new recipe table is built after the listeners' applies. Build
it in `tags_updated` on `TagsUpdateCause::ServerDataLoad { .. }`: it fires once the server's
new tables are in, on a world's first open (`reload: false`) and after every `/reload`
(`reload: true`), whether or not anyone is online, and `recipes_of_type` reads the new table
there. A refused `/reload` keeps the old tables and does not fire it, so the machine's table
never runs ahead of the game's.

`datapack_sync` is the wrong place for this: it is asked once per player, and not at all on a
`/reload` with nobody online, so a table rebuilt there stays stale until someone joins. Keep
`datapack_sync` for what it is for: naming the recipe types a player's client needs.

Parse the machine's own files in a reload listener when the game doesn't read them (a type it
has no serializer for): prepare parses, apply publishes, and a refused reload keeps the old.

## Embedded data

Generate code-only tables as Rust source before packaging and include them under `src/`.
The launcher refuses mod build scripts with `E_NATIVE_BUILD_SCRIPT`. Embedded tables do not
participate in resource reloads.

## Package contents

A native bundle accepts the manifest, optional icon, root `Cargo.toml`, `Cargo.lock` and `build.rs`,
`.rs` and `.toml` files under `src/`, `assets/` and `data/`, and root text files (`README*`, `LICENSE*`,
`LICENCE*`, `COPYING*`, `NOTICE*`). Source and resource size limits are checked separately;
any other file causes the archive to be refused ([the manifest](../../reference/manifest.md)).

Although the archive accepts `build.rs`, a native build refuses it unless you disable it with
`build = false` in Cargo's `[package]` table and remove build dependencies.

A native `.hmod` carries both source and content: the launcher compiles the source, and the game
loads the package's `assets/` and `data/`. [Wasm packages](../wasm-mods.md) carry executable
`mod.wasm` bytes declared in `[wasm]`. All package kinds take their identity from `horizon-mod.toml`.
