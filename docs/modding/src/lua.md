# Lua mods

A mod can be written in Lua as well as in Rust (native or wasm). A Lua mod implements the same
hooks: the game calls it through the same dispatch, and the hook names a script may use are
checked against the hook catalog the Rust surfaces are generated from
(`crates/horizon-mod-vocab/src/hooks`), at the same contract version.

The runtime is LuaJIT on every platform, so scripts see Lua 5.1 everywhere: numbers are doubles
(no integer type), there is no `//` and no bitwise operators (use `bit.band` and friends). The JIT
compiler is on where the platform allows it; elsewhere the interpreter runs the same code with
the same results. LuaJIT itself builds without the JIT compiler on iOS, so a Lua mod there always
runs interpreted.

## Layout

```toml
# horizon-mod.toml
[lua]
entry = "main.lua"
```

Scripts are `data/<id>/scripts/*.lua` in the package. `entry` runs when the game starts;
`require("util")` loads `util.lua` from the same folder. `/reload` reads the scripts again and
runs them in a fresh state.

## Hooks

```lua
-- A global hook.
horizon.on("loot_table_load", function(id, table)
  if id == "minecraft:chests/simple_dungeon" then
    table.pools[#table.pools + 1] = { rolls = 1,
      entries = { { type = "minecraft:item", name = "mymod:onion" } } }
  end
  return table
end)

-- After the server's data loads: `reload` is true after /reload, players online or not.
horizon.on("tags_updated", function(info) end)

-- A block's Part. `ctx.used` is the stack left in the hand: change its count, replace it or
-- set it to nil.
horizon.block("mymod:skillet", {
  use_item_on = function(ctx)
    if ctx.stack and ctx.stack.id == "minecraft:egg" then
      ctx.used = { id = "mymod:skillet_with_egg", count = 1 }
      return "success"
    end
  end,
})

-- A block entity type's Part: `ctx.data` (data slots) and `ctx.nbt` (saved data, as JSON) can
-- be changed; `ctx.dimension` is the level it ticks in.
horizon.block_entity("mymod:stove", {
  block_entity_tick = function(ctx)
    ctx.data[1] = ctx.data[1] + 1
  end,
})

-- A load-time transform (see Load-time data transforms).
horizon.data_transform("biome_modifier", function(files, context) return files end)

-- A crafting kind: the recipe's JSON and the grid come in decoded.
horizon.crafting_recipe("mymod:dough", {
  matches = function(recipe, grid) return true end,
  assemble = function(recipe, grid) return { id = "mymod:dough", count = 1 } end,
  remaining_items = function(recipe, grid) return {} end,
})
```

Hook contexts read the world: `ctx.block_at(x, y, z)` (a block id) and `ctx.biome_at(x, y, z)`.

Hooks available to Lua today: `loot_table_load`, `tags_updated`, `use_item_on` and
`block_entity_tick`. Naming any other hook is an error when the script loads, which says whether
the name is not a hook at all or not available to Lua yet.

## Errors and time

Every call into a script is protected: an error is written to the log with the mod's id and the
script's file and line, and the game goes on. After `max_errors` errors in a row a hook is off
until `/reload`. A call longer than `budget_ms` is logged (the first time, then every power of
two); one still running after `hard_limit_ms` is stopped with an error (interpreted code only).
One guarded call costs about half a microsecond on a desktop.

`config/horizon-lua.json` in the game folder:

```json
{ "max_errors": 5, "budget_ms": 2, "hard_limit_ms": 500, "jit": "auto", "allow_io": false }
```

`"jit": "off"` runs the interpreter everywhere.

## Determinism

Scripts that take part in loading or generation must give the same result on every machine:

- `os` and `io` are absent unless `allow_io` is on; `dofile` and `loadfile` are absent.
- `math.random` is an error: use `horizon.random(seed)` (`:next_int(bound)`, `:next_float()`,
  `:next_double()`, `:next_boolean()`), Java's `Random`, the same numbers for the same seed.
- `pairs()` order differs from run to run: walk with `horizon.sorted_pairs(t)` (numbers, then
  strings, each sorted).
- Data transform files come in and go back sorted by id.
