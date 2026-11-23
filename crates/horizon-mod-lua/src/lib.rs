//! Lua mods: scripts on LuaJIT (Lua 5.1 semantics on every platform, the JIT compiler where it
//! runs and the interpreter where it doesn't) that implement the same hooks as native and wasm
//! mods. A Lua mod is a `ModEntry` like any other: the game dispatches to it through the
//! generated `Hooks` and Part traits, so a hook means the same thing whatever language
//! answers it. The hook names a script may use are checked against the hook catalog the native
//! and wasm surfaces are generated from (`catalog.rs`, written by `build.rs`).
//!
//! A mod's scripts are `data/<id>/scripts/*.lua` in its package; `[lua] entry` in its manifest
//! names the one that runs first. They are read again on `/reload`.
mod catalog;
mod random;
mod runtime;

pub use runtime::{JitMode, LuaSettings, ScriptReader, Scripts};

use mlua::{Function, Lua, Table, Value};
use mod_api::ServerHost;
use runtime::Script;
use std::sync::Arc;

/// Load mod `id`'s scripts and make the mod: the entry script runs now, and what it registers
/// (hooks, Parts, data transforms, crafting kinds) is what the mod does. `read` reads the scripts
/// again for `/reload`.
pub fn mod_entry(
    id: &str,
    entry: &str,
    scripts: Scripts,
    read: ScriptReader,
    settings: LuaSettings,
) -> Result<mod_api::ModEntry, String> {
    let script = Script::load(id, entry, scripts, read, settings)?;
    log::info!(
        "mod {id}: Lua scripts loaded, {}",
        if script.jit_on() {
            "JIT compiler on"
        } else {
            "interpreter (no JIT)"
        }
    );
    let id: &'static str = Box::leak(id.to_owned().into_boxed_str());
    Ok(mod_api::ModEntry::new(id, Box::new(LuaMod { script })))
}

/// A Lua mod's hooks: each finds the script's handler at the call, so `/reload` swaps them.
struct LuaMod {
    script: Arc<Script>,
}

#[mod_api::__subscriptions]
impl mod_api::Hooks for LuaMod {
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        for block in self.script.declared("_blocks") {
            let key = r.block_key(&block);
            r.block_part(
                key,
                LuaBlockPart {
                    script: self.script.clone(),
                    id: block,
                },
            );
        }
        for kind in self.script.declared("_block_entities") {
            let key = r.block_entity_type_key(&kind);
            if let Err(e) = r.block_entity_part(
                &key,
                LuaBlockEntityPart {
                    script: self.script.clone(),
                    id: kind.clone(),
                },
            ) {
                log::error!("mod {}: block entity {kind}: {e:?}", self.script.id);
            }
        }
        for kind in self.script.declared("_transforms") {
            r.register_data_transform(
                &kind,
                LuaTransform {
                    script: self.script.clone(),
                    kind: kind.clone(),
                },
            );
        }
        for name in self.script.declared("_recipes") {
            r.register_crafting_recipe_serializer(
                &name,
                LuaRecipeKind {
                    script: self.script.clone(),
                    name: name.clone(),
                },
            );
        }
    }

    fn loot_table_load(
        &self,
        _: &dyn mod_api::DataLoadContext,
        event: &mut mod_api::LootTableLoadEvent<'_>,
    ) {
        let id = event.id;
        let table = &*event.table;
        let rewritten = self.script.call("loot_table_load", |lua| {
            let handlers = Script::global_handlers(lua, "loot_table_load")?;
            if handlers.is_empty() {
                return Ok(None);
            }
            let mut value = Script::to_lua(lua, table)?;
            for handler in handlers {
                let returned: Value = handler.call((id, value.clone()))?;
                if !returned.is_nil() {
                    value = returned;
                }
            }
            Ok(Some(Script::from_lua(lua, value)?))
        });
        if let Some(Some(rewritten)) = rewritten {
            *event.table = rewritten;
        }
    }

    fn tags_updated(&self, _: mod_api::HostSide<'_>, event: &mut mod_api::TagsUpdatedEvent) {
        // `/reload` reads the scripts again before telling them.
        if event.cause == (mod_api::TagsUpdateCause::ServerDataLoad { reload: true }) {
            self.script.reload();
        }
        let cause = event.cause;
        self.script.call("tags_updated", |lua| {
            for handler in Script::global_handlers(lua, "tags_updated")? {
                let info = lua.create_table()?;
                match cause {
                    mod_api::TagsUpdateCause::ServerDataLoad { reload } => {
                        info.set("cause", "server_data_load")?;
                        info.set("reload", reload)?;
                    }
                    mod_api::TagsUpdateCause::ClientPacketReceived { integrated_server } => {
                        info.set("cause", "client_packet_received")?;
                        info.set("integrated_server", integrated_server)?;
                    }
                    _ => info.set("cause", "other")?,
                }
                handler.call::<()>(info)?;
            }
            Ok(())
        });
    }
}

/// A stack as a script sees it: `{ id = "minecraft:bucket", count = 1 }`; nil for none.
fn stack_to_lua(lua: &Lua, stack: Option<&mod_api::ItemStack>) -> mlua::Result<Value> {
    let Some(stack) = stack else {
        return Ok(Value::Nil);
    };
    let table = lua.create_table()?;
    table.set(
        "id",
        mod_api::__private::item_key_id(stack.item()).unwrap_or("minecraft:air"),
    )?;
    table.set("count", stack.count())?;
    Ok(Value::Table(table))
}

/// A script's stack back: nil (or a count of 0) is none. A stack of the same item as `like`
/// keeps `like`'s components with the new count.
fn stack_from_lua(
    value: &Value,
    like: Option<&mod_api::ItemStack>,
) -> mlua::Result<Option<mod_api::ItemStack>> {
    let Value::Table(table) = value else {
        return Ok(None);
    };
    let id: String = table.get("id")?;
    let count: u32 = table.get::<Option<u32>>("count")?.unwrap_or(1);
    if count == 0 {
        return Ok(None);
    }
    let key = mod_api::__private::item_key(&id)
        .ok_or_else(|| mlua::Error::runtime(format!("unknown item {id}")))?;
    let mut stack = match like {
        Some(like) if like.item() == key => like.clone(),
        _ => mod_api::ItemStack::new(key, 1)
            .ok_or_else(|| mlua::Error::runtime(format!("{id}: not a stack")))?,
    };
    stack
        .set_count(count)
        .map_err(|e| mlua::Error::runtime(format!("{id} x{count}: {e:?}")))?;
    Ok(Some(stack))
}

/// `block_at(x, y, z)` and `biome_at(x, y, z)` on `ctx`, reading `level` through `host` for
/// the call alone.
fn add_world_reads<'scope, 'env>(
    scope: &'scope mlua::Scope<'scope, 'env>,
    ctx: &Table,
    host: &'env dyn mod_api::Host,
    level: Option<mod_api::LevelRef>,
) -> mlua::Result<()> {
    ctx.set(
        "block_at",
        scope.create_function(move |_, (x, y, z): (i32, i32, i32)| {
            Ok(level
                .and_then(|level| level.block(host, [x, y, z]))
                .and_then(mod_api::__private::block_key_id)
                .map(str::to_owned))
        })?,
    )?;
    ctx.set(
        "biome_at",
        scope.create_function(move |_, (x, y, z): (i32, i32, i32)| {
            Ok(level.and_then(|level| level.biome_at(host, [x, y, z])))
        })?,
    )?;
    Ok(())
}

/// A script's answer to `use_item_on`.
fn interaction(answer: &Value) -> mlua::Result<Option<mod_api::InteractionResult>> {
    use mod_api::InteractionResult as R;
    Ok(match answer {
        Value::Nil => None,
        Value::String(text) => Some(match &*text.to_str()? {
            "success" => R::Success,
            "success_server" => R::SuccessServer,
            "consume" => R::Consume,
            "fail" => R::Fail,
            "pass" => R::Pass,
            other => {
                return Err(mlua::Error::runtime(format!(
                    "use_item_on: {other:?} is not success, success_server, consume, fail or pass"
                )));
            }
        }),
        _ => {
            return Err(mlua::Error::runtime(
                "use_item_on: answer with a string or nil",
            ));
        }
    })
}

/// The Part of a block a script handles.
struct LuaBlockPart {
    script: Arc<Script>,
    id: String,
}

#[mod_api::horizon_part]
impl mod_api::BlockPart for LuaBlockPart {
    /// `ctx`: `block`, `pos`, `hand` (`main_hand`, `off_hand`), `dimension`, `stack` (what was
    /// in the hand before), `used` (the stack to leave in the hand: change its `count`, replace
    /// or clear it), and `block_at`/`biome_at` reads. Answer `"success"`, `"consume"`, `"fail"`,
    /// `"pass"` or nil.
    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut mod_api::UseItemOnEvent<'_>,
    ) -> Option<mod_api::InteractionResult> {
        let click = event.click;
        let before = event.used.stack().cloned();
        let host: &dyn ServerHost = host;
        let level = host.level(click.dimension.key().id());
        let answer = self.script.call("use_item_on", |lua| {
            let Some(handler) = Script::part_handler(lua, "_blocks", &self.id, "use_item_on")?
            else {
                return Ok((None, None));
            };
            lua.scope(|scope| {
                let ctx = lua.create_table()?;
                ctx.set("block", self.id.as_str())?;
                ctx.set("pos", click.pos)?;
                ctx.set(
                    "hand",
                    match click.hand {
                        mod_api::Hand::Main => "main_hand",
                        _ => "off_hand",
                    },
                )?;
                ctx.set("dimension", click.dimension.key().id())?;
                ctx.set("stack", stack_to_lua(lua, click.stack)?)?;
                let used = stack_to_lua(lua, before.as_ref())?;
                ctx.set("used", used.clone())?;
                add_world_reads(scope, &ctx, host, level)?;
                let answer: Value = handler.call(ctx.clone())?;
                let left: Value = ctx.get("used")?;
                let changed = !left.equals(&used)?
                    || match (&left, &used) {
                        (Value::Table(a), Value::Table(b)) => {
                            Script::from_lua(lua, Value::Table(a.clone()))?
                                != Script::from_lua(lua, Value::Table(b.clone()))?
                        }
                        _ => false,
                    };
                let used = if changed {
                    Some(stack_from_lua(&left, before.as_ref())?)
                } else {
                    None
                };
                Ok((interaction(&answer)?, used))
            })
        });
        let (answer, used) = answer.unwrap_or((None, None));
        if let Some(stack) = used {
            event.used.replace(stack);
        }
        answer
    }
}

/// The Part of a block entity type a script handles.
struct LuaBlockEntityPart {
    script: Arc<Script>,
    id: String,
}

#[mod_api::horizon_part]
impl mod_api::BlockPart for LuaBlockEntityPart {}

#[mod_api::horizon_part]
impl mod_api::BlockEntityPart for LuaBlockEntityPart {
    /// `ctx`: `type`, `pos`, `dimension` (the level it ticks in), `data` (its data slots) and
    /// `nbt` (its saved data as JSON) to read and change, and `block_at`/`biome_at` reads.
    fn block_entity_tick(
        &self,
        host: &mut dyn ServerHost,
        event: &mut mod_api::BlockEntityTickEvent<'_>,
    ) {
        let host: &dyn ServerHost = host;
        let level = event.level;
        let data = event.entity.data.clone();
        let nbt = horizon_core::json_nbt::tag_to_json(&horizon_core::nbt::Tag::Compound(
            event.entity.nbt.clone(),
        ))
        .unwrap_or(serde_json::Value::Null);
        let edits = self.script.call("block_entity_tick", |lua| {
            let Some(handler) =
                Script::part_handler(lua, "_block_entities", &self.id, "block_entity_tick")?
            else {
                return Ok(None);
            };
            lua.scope(|scope| {
                let ctx = lua.create_table()?;
                ctx.set("type", self.id.as_str())?;
                ctx.set("pos", event.pos)?;
                ctx.set("dimension", level.dimension_id())?;
                ctx.set("data", lua.create_sequence_from(data.iter().copied())?)?;
                ctx.set("nbt", Script::to_lua(lua, &nbt)?)?;
                add_world_reads(scope, &ctx, host, Some(level))?;
                handler.call::<()>(ctx.clone())?;
                let new_data: Vec<i32> = ctx
                    .get::<Table>("data")?
                    .sequence_values()
                    .collect::<mlua::Result<_>>()?;
                let new_nbt = Script::from_lua(lua, ctx.get("nbt")?)?;
                Ok(Some((new_data, new_nbt)))
            })
        });
        let Some(Some((new_data, new_nbt))) = edits else {
            *event.reply = mod_api::BlockEntityTickReply::Unchanged;
            return;
        };
        let mut changed = false;
        if new_data != data && new_data.len() == data.len() {
            event.entity.data = new_data;
            changed = true;
        }
        if new_nbt != nbt
            && let Ok(horizon_core::nbt::Tag::Compound(compound)) =
                horizon_core::json_nbt::json_to_tag(&new_nbt)
        {
            event.entity.nbt = compound;
            changed = true;
        }
        if !changed {
            *event.reply = mod_api::BlockEntityTickReply::Unchanged;
        }
    }
}

/// A script's load-time transform of one kind of data.
struct LuaTransform {
    script: Arc<Script>,
    kind: String,
}

impl mod_api::DataTransform for LuaTransform {
    /// Each handler gets `files` (id to decoded JSON) and `context` (`biomes`, id to decoded
    /// JSON, and `biome_tags`, id to a list of biomes, for biome modifiers) and returns the files
    /// (or changes them in place).
    fn transform(&self, mut data: mod_api::DataFiles) -> Result<mod_api::DataFiles, String> {
        let files = self
            .script
            .call(&format!("data_transform:{}", self.kind), |lua| {
                let mut files = Value::Table(lua.create_table()?);
                if let Value::Table(table) = &files {
                    for (id, text) in &data.files {
                        let json: serde_json::Value = serde_json::from_str(text)
                            .map_err(|e| mlua::Error::runtime(format!("{id}: {e}")))?;
                        table.set(id.as_str(), Script::to_lua(lua, &json)?)?;
                    }
                }
                let context = lua.create_table()?;
                let biomes = lua.create_table()?;
                for (id, text) in &data.biomes {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                        biomes.set(id.as_str(), Script::to_lua(lua, &json)?)?;
                    }
                }
                context.set("biomes", biomes)?;
                let tags = lua.create_table()?;
                for (id, members) in &data.biome_tags {
                    tags.set(
                        id.as_str(),
                        lua.create_sequence_from(members.iter().cloned())?,
                    )?;
                }
                context.set("biome_tags", tags)?;
                let transforms: Table =
                    lua.globals().get::<Table>("horizon")?.get("_transforms")?;
                let handlers: Vec<Function> =
                    match transforms.get::<Option<Table>>(self.kind.as_str())? {
                        Some(list) => list.sequence_values().collect::<mlua::Result<_>>()?,
                        None => Vec::new(),
                    };
                for handler in handlers {
                    let returned: Value = handler.call((files.clone(), context.clone()))?;
                    if !returned.is_nil() {
                        files = returned;
                    }
                }
                let Value::Table(table) = files else {
                    return Err(mlua::Error::runtime("a transform returns the files table"));
                };
                let mut out = Vec::new();
                for pair in table.pairs::<String, Value>() {
                    let (id, value) = pair?;
                    let json = Script::from_lua(lua, value)?;
                    out.push((id, json.to_string()));
                }
                Ok(out)
            });
        match files {
            Some(files) => {
                data.files = files;
                Ok(data)
            }
            None => Err(format!(
                "the {} transform's script failed (see the line above)",
                self.kind
            )),
        }
    }
}

/// A script's crafting kind: its value is the recipe's JSON text, and the script's `matches`,
/// `assemble` and `remaining_items` get it decoded with the grid.
struct LuaRecipeKind {
    script: Arc<Script>,
    name: String,
}

impl LuaRecipeKind {
    fn grid(lua: &Lua, grid: &mod_api::CraftingGrid) -> mlua::Result<Table> {
        let table = lua.create_table()?;
        table.set("width", grid.width)?;
        table.set("height", grid.height)?;
        let cells = lua.create_table()?;
        for (i, cell) in grid.cells.iter().enumerate() {
            let value = match stack_to_lua(lua, cell.as_ref())? {
                Value::Nil => Value::Boolean(false),
                stack => stack,
            };
            cells.set(i + 1, value)?;
        }
        table.set("cells", cells)?;
        Ok(table)
    }

    fn handler(lua: &Lua, name: &str, callback: &str) -> mlua::Result<Option<Function>> {
        let recipes: Table = lua.globals().get::<Table>("horizon")?.get("_recipes")?;
        match recipes.get::<Option<Table>>(name)? {
            Some(kind) => kind.get(callback),
            None => Ok(None),
        }
    }

    fn recipe(lua: &Lua, value: &str) -> mlua::Result<Value> {
        let json: serde_json::Value =
            serde_json::from_str(value).map_err(|e| mlua::Error::runtime(e.to_string()))?;
        Script::to_lua(lua, &json)
    }
}

impl mod_api::CraftingRecipeKind for LuaRecipeKind {
    type Value = String;
    fn codec(&self) -> Option<mod_api::CraftingRecipeCodec<String>> {
        Some(mod_api::CraftingRecipeCodec::wire())
    }
    fn decode(&self, json: &str) -> Result<String, String> {
        serde_json::from_str::<serde_json::Value>(json).map_err(|e| e.to_string())?;
        Ok(json.to_owned())
    }
    fn write(&self, value: &String) -> Result<String, String> {
        Ok(value.clone())
    }
    fn matches(&self, value: &String, grid: mod_api::CraftingGrid) -> bool {
        self.script
            .call(&format!("crafting:{}", self.name), |lua| {
                let Some(matches) = Self::handler(lua, &self.name, "matches")? else {
                    return Ok(false);
                };
                matches.call::<bool>((Self::recipe(lua, value)?, Self::grid(lua, &grid)?))
            })
            .unwrap_or(false)
    }
    fn assemble(&self, value: &String, grid: mod_api::CraftingGrid) -> Option<mod_api::ItemStack> {
        self.script
            .call(&format!("crafting:{}", self.name), |lua| {
                let Some(assemble) = Self::handler(lua, &self.name, "assemble")? else {
                    return Ok(None);
                };
                let stack: Value =
                    assemble.call((Self::recipe(lua, value)?, Self::grid(lua, &grid)?))?;
                stack_from_lua(&stack, None)
            })
            .flatten()
    }
    fn remaining_items(
        &self,
        value: &String,
        grid: mod_api::CraftingGrid,
    ) -> Option<Vec<Option<mod_api::ItemStack>>> {
        let cells = grid.cells.len();
        self.script
            .call(&format!("crafting:{}", self.name), |lua| {
                let Some(remaining) = Self::handler(lua, &self.name, "remaining_items")? else {
                    return Ok(None);
                };
                let left: Value =
                    remaining.call((Self::recipe(lua, value)?, Self::grid(lua, &grid)?))?;
                let Value::Table(left) = left else {
                    return Ok(None);
                };
                (1..=cells)
                    .map(|i| stack_from_lua(&left.get::<Value>(i)?, None))
                    .collect::<mlua::Result<Vec<_>>>()
                    .map(Some)
            })
            .flatten()
    }
}

#[cfg(test)]
mod tests;
