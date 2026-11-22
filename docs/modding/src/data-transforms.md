# Load-time data transforms

A mod can read and rewrite one kind of data files before the game parses them, so a mod's own
file types that the game doesn't read can become ones it does, without changing the mod's data
pack. Register a `DataTransform` in `Hooks::register`:

```rust
registrar.register_data_transform(mod_api::DataFiles::BIOME_MODIFIER, MyFilterModifiers);
```

`transform` gets a `DataFiles`: the kind's files as `(id, json text)` pairs in id order, and
gives it back changed (entries added, removed or rewritten). It works the same for native and
wasm mods.

| Kind | Files | When | Context |
|---|---|---|---|
| `DataFiles::BIOME_MODIFIER` | `data/<ns>/neoforge/biome_modifier/` of every mod, and those declared in code | when a world opens, after its data packs' biomes and tags are in | `biomes` (id and JSON, after the packs) and `biome_tags` (each tag's biomes, nested tags resolved) |
| `DataFiles::RECIPE` | `data/<ns>/recipe/`, the world's packs over the game's own; vanilla's are `minecraft:` ids | every load and `/reload`, on a server and on a client that loads its own | none |

A biome modifier of a type the game doesn't read (`farmersdelight:add_features_by_filter`)
is not skipped when the content builds: it waits for the world open, where a transform can
rewrite it into `neoforge:add_features` with an explicit biome list. Only one still unread
there is reported in the log. A recipe whose `OR` group a type can't read can be split into one
recipe per alternative.

Loot tables have their own per-table hook (`loot_table_load`), and tags and other data files are
added with `data_pack_files`.

## Rules

- Transforms run in mod load order, each seeing what the one before left. The result is cached
  with the load (the world's generator data, the server's recipe table), not recomputed.
- Keep a transform a pure function of what it is given: no clock, no random, no file reads.
  The files come in id order and go back sorted by id, so the same input gives the same world
  on every machine.
- A transform that returns an error or panics is written to the log with the mod's id, and the
  files stay as they were: the load goes on.
- A file a recipe transform leaves that isn't JSON is dropped with a line in the log.
