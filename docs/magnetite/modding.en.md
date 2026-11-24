# Modding Magnetite

Magnetite is Minecraft: Java Edition 26.3 in Rust for PC, Android and iOS. The three platforms
share one core, Horizon's, so a mod is written once and runs the same wherever its kind of mod
is supported. Magnetite's modding is Horizon's with differences: its own mod platform, prebuilt
native mods for phones, Lua, and hooks added for porting mods such as Farmer's Delight and Create.
This guide covers the kinds of mod, how they fit into the game, what Magnetite adds, and how to
publish a mod.

Every function is listed in the [reference](modding-reference.en.md): every hook and every `Host`
method, one line each. Horizon's detailed modding book is in `docs/modding/src` (mdBook); its
pages are named along the way. По-русски: [modding.md](modding.md).

## Kinds of mod

Every mod is one `.hmod` file: a zip holding the `horizon-mod.toml` manifest, the `assets/` and
`data/` folders and, for a mod with code, that code.

| Kind | What it holds | How it runs |
| --- | --- | --- |
| **Content** | `assets/` and `data/`: blocks, items, recipes, loot, tags, models, textures, world generation | The game reads it at start-up. No code. |
| **WebAssembly** | `mod.wasm`, built from Rust | The game runs it in a sandbox with time and memory limits. No compiler on the player's device. |
| **Lua** | Scripts in `data/<id>/scripts/*.lua` | LuaJIT, Lua 5.1 on every platform. Nothing to build. |
| **Native** | A Rust crate built for one exact game build | Full speed, full trust. On PC the launcher builds it; phones get prebuilt binaries from the mod farm. |

WebAssembly and native mods are written against the same API, `mod_api` (the `horizon-mod-api`
crate), with the same hooks, Parts, attachments and dependencies; only the build target differs.
A Lua mod calls the same hooks by their names from the same catalog.

### Where each kind runs

| Kind | PC | Android | iOS with JIT | iOS without JIT |
| --- | --- | --- | --- | --- |
| Content | yes | yes | yes | yes |
| WebAssembly | yes (Cranelift) | yes | yes (Cranelift) | yes (the Pulley interpreter, slower) |
| Lua | yes (JIT) | yes (JIT) | yes (interpreter) | yes (interpreter) |
| Native | built by the launcher | a prebuilt binary from the farm | a prebuilt binary, loaded directly | no (or re-signed through ferrite-signer) |

LuaJIT is always built without its JIT compiler on iOS, so Lua is always interpreted there.

Magnetite's rule: the world comes out **bit for bit the same** on PC, on a phone, with JIT and
without it, and without JIT the game is slower, never broken. So **WebAssembly** is the main path
for new mods: one `.wasm` for every platform, no per-platform builds. The native path is for heavy
mods (ports such as Create) where every millisecond counts.

## The package and the manifest

```text
mymod.hmod
├── horizon-mod.toml          # the manifest: id, name, version, api, kind of code
├── assets/mymod/             # models, textures, sounds, lang/en_us.json
├── data/mymod/               # blocks, recipes, loot, tags, generation
│   └── scripts/main.lua      # a Lua mod only
└── mod.wasm                  # a WebAssembly mod only
```

```toml
# horizon-mod.toml
id = "mymod"              # [a-z0-9_], up to 64 characters: folder, log and settings key
name = "My Mod"
version = "1.0.0"
description = "What the mod does."
authors = ["Nick"]
license = "MIT"
api = "0.2"               # the content API's version

[lua]                     # a Lua mod only
entry = "main.lua"
```

The `[wasm]` table (the hash of `mod.wasm` and the toolchain's version) is written by the
WebAssembly build; never write it by hand. The SDK build the code is made for is pinned in
`Cargo.toml` (`[package.metadata.horizon] sdk`). Every manifest field:
`docs/modding/src/reference/manifest.md`.

The tool to make and build mods is `cargo horizon-mod`:

```sh
cargo horizon-mod new mymod --template wasm     # a new WebAssembly mod
cargo horizon-mod build --wasm                  # build mod.wasm and pack the .hmod
cargo horizon-mod new mymod --template native   # a new native mod (on PC the launcher builds it)
cargo horizon-mod update                        # move to a newer SDK build
```

First mods, step by step: `start/first-content-mod.md`, `start/first-mod.md` (native) and
`start/first-wasm-mod.md` in the book.

## How a mod reaches the game

### Hooks

A **hook** is one moment in the game: a tick, a block broken, a player joining. A mod implements
the hooks it needs as methods of `mod_api::Hooks`, and the game calls every subscribed mod with the
moment's **event** and a **host**, its typed view of the game.

```rust
#[derive(Default)]
struct MyMod;

#[mod_api::horizon_mod]
impl mod_api::Hooks for MyMod {
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        // Once, at start-up: keys, Parts, settings.
    }
}
```

Every hook has a **contract**, which says what a mod's answer means:

| Contract | What the mod does | Its answer |
| --- | --- | --- |
| listen | is told of the event and may change the event's fields | nothing |
| cancel | may stop the game's action | `Flow::Cancel` stops it and skips later mods |
| replace | may replace vanilla's answer | the first mod to answer `Some` wins |
| collect | adds its own entries | every mod's entries are gathered into a list |

Mods run by listener priority, highest first, then in dependency order. A mod that must hear of
cancelled events too adds `receive_cancelled`:

```rust
#[listener(priority = 100)]
fn break_block(&self, host: &mut dyn ServerHost, event: &mut BreakBlockEvent<'_>) -> Flow {
    if self.protected(host, event.pos) { Flow::Cancel } else { Flow::Continue }
}
```

A hook has a **side**: the server (the world's logic), the client (screens, rendering, sound,
input) or both. Server hooks run in single player too, whose server lives inside the client.

### Parts

When a moment belongs to one object, it is a method of a **Part**, not a global hook. A Part is
attached to an object type in `register`, and the game calls it for those objects only.

| Part | Attached to |
| --- | --- |
| `BlockPart` | a block type, your own or vanilla's |
| `BlockEntityPart` | a block with a block entity (a furnace, a chest, a machine) |
| `ItemPart` | an item type |
| `EntityPart` | an entity type: a mob, a projectile |
| `MenuPart` | a menu type (a container's window) |
| `FluidPart` | a mod's fluid |

```rust
#[mod_api::horizon_part]
impl mod_api::BlockPart for Gauge {
    fn analog_output(&self, _: &dyn mod_api::LevelHost, event: &mut mod_api::AnalogOutputEvent) {
        event.output = 15; // a comparator always reads 15 from this block
    }
}
// in register:
let stone = r.block_key("minecraft:stone");
r.block_part(stone, Gauge);
```

### Host

`Host` is everything a hook may ask of the game or do in it: read and place a block, find an
entity, give an item, send a message, play a sound. `ServerHost` exists on the server only,
`ClientHost` on the client only, `Host` on both sides. Game objects come as **handles**
(`LevelRef`, `EntityRef`, `PlayerRef`...): a handle is read through the host, and a stale handle
gives `None`, not a crash.

### Load phases

A mod's code runs in phases, each with its own access:

| Phase | What is available |
| --- | --- |
| Packages | package discovery, no host |
| Registration | declarations through `RegistrationContext`: keys, Parts, settings; no host |
| Setup | asset, data and registry reads; no world |
| Config | settings and the mod's config files |
| Reloading | a data reload (`/reload`, a resource pack change) |
| World | the full `ServerHost` or `ClientHost` |

### Beyond the named hooks: `[[inject]]` and library mods

- **`[[inject]]`** attaches a typed listener to a function of the game without compiling game
  code: `head` (before the body: change the arguments, cancel, or give the result), `return`
  (after it: change the returned value) or `replace` (the mod owns the function). Points exist for
  the functions of `horizon-client`'s `game/` module and those marked `#[replaceable]` below it.
  Native and WebAssembly mods both use it; a WebAssembly mod with `[[inject]]` rows is rebuilt for
  every new game build. See `concepts/injections.md`.
- **A library mod** publishes its own functions and hooks for other mods: they list it in
  `[dependencies]` and call it as ordinary Rust (`my_library::function()`). The game loads one copy
  shared by every mod that uses it.
- **The native route** (`[[native.mixin]]`, `[[native.loop]]`, `[[native.bridge]]`,
  `[[native.module]]`) changes code inside a function. Native mods only.

On phones, `[[inject]]` points exist only in a build staged as the SDK; the APK and IPA are not
yet built that way, so for now a mod that needs `[[inject]]` runs on PC only.

More on hooks, Parts, phases, attachments, capabilities and transactions: the book's Concepts.

## What Magnetite adds to Horizon

These were added for porting mods (Farmer's Delight, Create). They work the same for native,
WebAssembly and, where noted, Lua mods.

### Load-time data transforms

Before the game parses a kind of data files, a mod can read them and rewrite them: add, remove or
change entries. A mod's own file formats become ones the game reads, without changing its data
pack.

```rust
r.register_data_transform(mod_api::DataFiles::BIOME_MODIFIER, MyFilterModifiers);
```

| Kind | Files | When |
| --- | --- | --- |
| `DataFiles::BIOME_MODIFIER` | `data/<ns>/neoforge/biome_modifier/` of every mod | when a world opens, once its biomes and tags are in |
| `DataFiles::RECIPE` | `data/<ns>/recipe/` | every load and `/reload` |

For example, `farmersdelight:add_features_by_filter`, which the game doesn't read, is rewritten
into `neoforge:add_features` with an explicit biome list; a recipe with an OR group of ingredients
is split into one recipe per alternative.

Rules:
- transforms run in mod load order, each seeing what the one before left;
- a transform is a pure function of its input: no clock, no randomness, no file reads; the files
  come in and go back sorted by id;
- an error or a panic is logged with the mod's id, and the files stay as they were.

More: `docs/modding/src/data-transforms.md`.

### Crafting kinds of your own

A recipe with its own serializer answers three questions: does the grid match (`matches`), what
does it make (`assemble`), and what stays in the grid after crafting (`remaining_items`).

```rust
r.register_crafting_recipe_serializer("my_mod:dough", Dough);
```

`remaining_items` gives `Some(stack)` or `None` (the item's own remainder) per cell. The remainder
goes back into the cell its input used up, as a water bucket does after dough. It works in the
inventory's 2x2 grid and the crafting table's 3x3, once per take on a shift-click. The callbacks
must be pure: the client's prediction runs them too. A machine that reads its own recipes uses
`register_recipe_type` and `register_raw_recipe_serializer`. More:
`docs/modding/src/crafting-recipe-kinds.md`.

### The block entity tick knows its level

`block_entity_tick` gets `level`, the level the block entity ticks in: a mod's stations work in the
Nether and the End as they do in the Overworld. Write to the world through `level`, not through a
fixed dimension's handle.

### The stack in the hand can change

In a block Part's `use_item_on`, the event carries `used`, the stack the click was made with. The
mod may replace it, change its count or its components (`stack_mut`), and the game puts the result
in the hand after the call: a skillet takes the egg and leaves the hand empty, or gives another
item back.

### `/reload` for a mod's tables

A mod's machine recipe tables are built in `tags_updated` on a server data load. It runs after
every load and every `/reload`, with nobody online too, and `recipes_of_type` already reads the new
table. Server reload listeners are registered in `add_server_reload_listeners`.

### `copy_components` in loot

`minecraft:copy_components` with `"source": "block_entity"` carries a broken mod block entity's
components onto the drop: its saved `components` and those its mod gives through the Part hook
`block_entity_collect_components`. A cooking pot keeps its name and meal when broken.

### Trades from a mod's namespace

A mod's `data/<ns>/villager_trade/` and `trade_set` are read from its own namespace: a profession
can buy the mod's crop, the wandering trader can sell its seeds.

### The microphone

A client mod can record the player's microphone (for voice chat, say). The system asks the
player: Android's `RECORD_AUDIO`, iOS's question with the line from `NSMicrophoneUsageDescription`;
a PC asks nothing of the game.

| What | How |
| --- | --- |
| The permission | `host.microphone_permission()`: `Granted`, `Denied`, `Undetermined` or `Unavailable` |
| Ask the player | `host.request_microphone()`: the system's question; the answer comes as the `microphone_permission_changed` hook |
| Start recording | `host.start_microphone()` gives the sample rate (48000, say) or a `MicrophoneError` |
| The sound | the `microphone_input(mod_id, samples, sample_rate)` hook once a client tick: mono, -1 to 1 |
| Stop | `host.stop_microphone()`; the device closes when no mod records |

```rust
#[listener]
fn client_setup(host: &mut impl ClientHost) {
    if host.request_microphone() == MicrophonePermission::Granted {
        let _ = host.start_microphone();
    }
}

#[listener]
fn microphone_permission_changed(host: &mut impl ClientHost, permission: MicrophonePermission) {
    if permission == MicrophonePermission::Granted {
        let _ = host.start_microphone();
    }
}

#[listener]
fn microphone_input(host: &mut impl ClientHost, mod_id: &str, samples: &[f32], rate: u32) {
    // samples: all that was recorded since the last tick
}
```

Only the mod that started recording hears its sound. A mod that does not take it keeps at most
two seconds; older samples are dropped. Native and WebAssembly mods; not Lua yet.

## Lua mods

```toml
[lua]
entry = "main.lua"
```

Scripts are `data/<id>/scripts/*.lua`. `entry` runs when the game starts; `require("util")` loads
`util.lua` from the same folder. `/reload` reads the scripts again into a fresh state.

```lua
-- A global hook.
horizon.on("loot_table_load", function(id, table)
  if id == "minecraft:chests/simple_dungeon" then
    table.pools[#table.pools + 1] = { rolls = 1,
      entries = { { type = "minecraft:item", name = "mymod:onion" } } }
  end
  return table
end)

-- A block's Part: ctx.used is the stack in the hand; change it, replace it or set it to nil.
horizon.block("mymod:skillet", {
  use_item_on = function(ctx)
    if ctx.stack and ctx.stack.id == "minecraft:egg" then
      ctx.used = { id = "mymod:skillet_with_egg", count = 1 }
      return "success"
    end
  end,
})

-- A block entity's Part: ctx.data (data slots) and ctx.nbt can change; ctx.dimension is its level.
horizon.block_entity("mymod:stove", {
  block_entity_tick = function(ctx) ctx.data[1] = ctx.data[1] + 1 end,
})

-- A data transform and a crafting kind.
horizon.data_transform("biome_modifier", function(files, context) return files end)
horizon.crafting_recipe("mymod:dough", {
  matches = function(recipe, grid) return true end,
  assemble = function(recipe, grid) return { id = "mymod:dough", count = 1 } end,
  remaining_items = function(recipe, grid) return {} end,
})
```

A hook's context reads the world: `ctx.block_at(x, y, z)` and `ctx.biome_at(x, y, z)`.

Lua has these hooks today: `loot_table_load`, `tags_updated`, `use_item_on` and
`block_entity_tick`, plus data transforms and crafting kinds. Any other hook name is an error when
the script loads, saying whether it is no hook at all or not open to Lua yet. The list will grow;
the names are those of the [reference](modding-reference.en.md).

**Robustness.** Every call into a script is protected: an error is logged with the mod's id, file
and line, and the game goes on. After `max_errors` errors in a row a hook is off until `/reload`.
A call longer than `budget_ms` is logged; one longer than `hard_limit_ms` is stopped. The settings
are `config/horizon-lua.json` in the game folder:

```json
{ "max_errors": 5, "budget_ms": 2, "hard_limit_ms": 500, "jit": "auto", "allow_io": false }
```

**LuaJIT (Lua 5.1).** Numbers are doubles only, with no integer type. There is no `//` and no
bitwise operators: use `bit.band` and the other `bit` functions.

More: `docs/modding/src/lua.md`.

## Determinism

Whatever takes part in loading data and generating the world must give the same result on every
machine:
- no clock, `math.random` or file reads in transforms, generation and recipe callbacks;
- in Lua, randomness only through `horizon.random(seed)` (the same generator as
  `java.util.Random`), and walk tables with `horizon.sorted_pairs(t)`, since `pairs()` order
  changes from run to run; `os` and `io` are off unless the config allows them;
- in Rust, sorted collections (`BTreeMap`, `Vec`) wherever order changes the result.

## Limits on phones and in the sandbox

- **WebAssembly:** no file system, threads, direct GPU access, chunk generators of its own or
  mixins; `[[inject]]` points take the mixins' place. Extending the WebAssembly API (chunk
  generators, a mod folder of its own, parallel calls, a safe shader API) is planned.
- **Native mods on a phone** come only as prebuilt binaries for the exact game build and platform:
  nothing compiles on a phone. With no binary for your build, the mod doesn't install, and the
  platform says so.
- **iOS without JIT:** no native mods (or re-signed through ferrite-signer); WebAssembly runs on
  Pulley, Lua on the interpreter. A future manifest field
  `ios_jit = "required" | "recommended" | "optional"` will say whether a mod needs JIT.

## Touch for mods (planned)

A mod will get the screen's touches on every platform, PCs with a touch screen included: the
finger's id, the phase (start, move, end, cancel), the position and the pressure where the device
gives it. A mod can take a touch away from the game's touch controls. The order: the mod's screen,
then mods' HUDs, then the game's touch controls. Buttons of a touch layout can already press mods'
keys.

## Magnetite's mod platform

Mods are distributed through [izrikmizrik/magnetite-mods](https://github.com/izrikmizrik/magnetite-mods).
The game and the phone launcher read its index at
`https://izrikmizrik.github.io/magnetite-mods/index.json` (Horizon's registry format, schema 1);
the mod files are the repository's releases. Horizon's own registry is not used.

A mod's entry in the index:

```json
{
  "id": "mymod", "kind": "mod", "name": "My Mod", "summary": "One line.",
  "description": "Markdown.", "authors": ["Nick"], "license": "MIT",
  "source": "https://github.com/nick/mymod", "categories": ["utility"], "tags": ["hud"],
  "versions": [{
    "version": "1.0.0", "published": "2026-11-23", "changelog": "",
    "api": "0.2", "side": "both", "capabilities": ["registries"], "game": "26.3",
    "file": { "name": "mymod-1.0.0.hmod",
              "url": "https://github.com/izrikmizrik/magnetite-mods/releases/download/mymod-1.0.0/mymod-1.0.0.hmod",
              "size": 12345, "sha256": "…" }
  }]
}
```

- `kind`: `mod` for content, WebAssembly and Lua mods; `native-mod` for native ones; also
  `modpack`, `resourcepack`, `shader`, `shaderpack` and `datapack`.
- Every version is pinned by its file's SHA-256. A project with a minisign key signs every version
  with it.
- Prebuilt native mods for Android and iOS go in a version's `native_binaries`: records of `build`
  (the exact game build id), `target` (`aarch64-linux-android`, `aarch64-apple-ios`) and `file`.
  The mod farm adds them, and builds only mods whose license allows shipping binaries.

To publish: release your `.hmod` in your own repository or send it to magnetite-mods, and open a
pull request adding your entry to `index.json`. The whole index format:
`docs/modding/src/reference/registry-index.md`.

Installing in the game:
- on PC: Options > Mods > Find mods;
- on a phone: the launcher > Mods > Find mods.

WebAssembly, Lua and content mods install and turn on at once, and load at the game's next start.

## Further reading

- [The function reference](modding-reference.en.md): every hook and `Host` method.
- The full API with parameters: `cargo doc -p horizon-mod-api --no-deps --open`.
- Horizon's book, `docs/modding/src`: Concepts (hooks, Parts, attachments, transactions,
  injections), How-to (the world, items, entities, the client), Reference (manifest, index, SDK).
- An example mod: <https://github.com/arismoko/horizon-machine-sample>.
