//! One Lua mod's state: its scripts in a LuaJIT state with the game's sandbox, the `horizon`
//! table they register through, and the guard every call into a script goes through.
use mlua::{Function, HookTriggers, Lua, LuaSerdeExt, StdLib, Table, Value, VmState};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, RwLock};
use std::time::{Duration, Instant};

/// Whether LuaJIT compiles to machine code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JitMode {
    /// Where the platform allows it; the interpreter where it doesn't (iOS without JIT).
    #[default]
    Auto,
    /// The interpreter alone (`jit.off()`).
    Off,
}

/// The Lua runtime's limits, from `config/horizon-lua.json` in the game folder.
#[derive(Clone, Debug, PartialEq)]
pub struct LuaSettings {
    /// After this many errors in a row a script's hook is switched off until `/reload`.
    pub max_errors: u32,
    /// A call longer than this is written to the log.
    pub budget: Duration,
    /// A call still running after this is stopped with an error (interpreted code only: a
    /// JIT-compiled loop is not interrupted).
    pub hard_limit: Duration,
    pub jit: JitMode,
    /// Lets scripts use `io` and `os`. Off by default: they make a world depend on the machine.
    pub allow_io: bool,
}

impl Default for LuaSettings {
    fn default() -> Self {
        Self {
            max_errors: 5,
            budget: Duration::from_millis(2),
            hard_limit: Duration::from_millis(500),
            jit: JitMode::Auto,
            allow_io: false,
        }
    }
}

impl LuaSettings {
    /// The settings in `config/horizon-lua.json` under `game_dir`; the defaults for a key that
    /// is missing, and for a file that is missing or doesn't parse (with a line in the log).
    pub fn read(game_dir: &std::path::Path) -> Self {
        let path = game_dir.join("config/horizon-lua.json");
        let Ok(bytes) = std::fs::read(&path) else {
            return Self::default();
        };
        let json: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(json) => json,
            Err(e) => {
                log::warn!("Lua: {}: {e}; using the defaults", path.display());
                return Self::default();
            }
        };
        let defaults = Self::default();
        let millis = |key: &str, default: Duration| {
            json[key].as_u64().map_or(default, Duration::from_millis)
        };
        Self {
            max_errors: json["max_errors"]
                .as_u64()
                .map_or(defaults.max_errors, |n| {
                    n.clamp(1, u64::from(u32::MAX)) as u32
                }),
            budget: millis("budget_ms", defaults.budget),
            hard_limit: millis("hard_limit_ms", defaults.hard_limit),
            jit: match json["jit"].as_str() {
                Some("off") => JitMode::Off,
                _ => JitMode::Auto,
            },
            allow_io: json["allow_io"].as_bool().unwrap_or(false),
        }
    }
}

/// A mod's scripts: file name under `data/<id>/scripts/` and its source.
pub type Scripts = BTreeMap<String, String>;

/// Reads a mod's scripts again, for `/reload`.
pub type ScriptReader = Box<dyn Fn() -> Result<Scripts, String> + Send + Sync>;

/// Per-hook error and time accounting.
#[derive(Default)]
struct Guard {
    errors_in_a_row: u32,
    off: bool,
    over_budget: u64,
}

/// One Lua mod.
pub struct Script {
    pub(crate) id: String,
    entry: String,
    settings: LuaSettings,
    read: ScriptReader,
    lua: RwLock<Lua>,
    guards: Mutex<HashMap<String, Guard>>,
    /// When the running call must stop, in nanoseconds after `epoch`; 0 while none runs.
    deadline: Arc<AtomicU64>,
    epoch: Instant,
}

const PRELUDE: &str = r#"
local H = { _on = {}, _blocks = {}, _block_entities = {}, _transforms = {}, _recipes = {} }
local check = __horizon_check
__horizon_check = nil
function H.on(name, fn)
  check(name, false)
  assert(type(fn) == "function", "horizon.on: a function")
  H._on[name] = H._on[name] or {}
  table.insert(H._on[name], fn)
end
local function part(store, scoped_kind)
  return function(id, handlers)
    assert(type(id) == "string", scoped_kind .. ": an id")
    assert(type(handlers) == "table", scoped_kind .. ": a table of hooks")
    for name, fn in pairs(handlers) do
      check(name, true)
      assert(type(fn) == "function", scoped_kind .. ": " .. name .. " is not a function")
    end
    store[id] = handlers
  end
end
H.block = part(H._blocks, "horizon.block")
H.block_entity = part(H._block_entities, "horizon.block_entity")
function H.data_transform(kind, fn)
  assert(kind == "biome_modifier" or kind == "recipe", "horizon.data_transform: biome_modifier or recipe")
  assert(type(fn) == "function", "horizon.data_transform: a function")
  H._transforms[kind] = H._transforms[kind] or {}
  table.insert(H._transforms[kind], fn)
end
function H.crafting_recipe(name, kind)
  assert(type(name) == "string", "horizon.crafting_recipe: a serializer name")
  assert(type(kind) == "table" and type(kind.matches) == "function"
    and type(kind.assemble) == "function", "horizon.crafting_recipe: matches and assemble")
  H._recipes[name] = kind
end
-- pairs() order is not the same from run to run: walk keys sorted, numbers before strings.
function H.sorted_pairs(t)
  local keys = {}
  for k in pairs(t) do keys[#keys + 1] = k end
  table.sort(keys, function(a, b)
    local ta, tb = type(a), type(b)
    if ta ~= tb then return ta < tb end
    if ta == "number" or ta == "string" then return a < b end
    return tostring(a) < tostring(b)
  end)
  local i = 0
  return function()
    i = i + 1
    local k = keys[i]
    if k ~= nil then return k, t[k] end
  end
end
horizon = H
"#;

impl Script {
    /// The mod `id` with `scripts`, running `entry` (a file of `scripts`). A script that fails
    /// to load is the error, naming the file and line.
    pub fn load(
        id: &str,
        entry: &str,
        scripts: Scripts,
        read: ScriptReader,
        settings: LuaSettings,
    ) -> Result<Arc<Self>, String> {
        let deadline = Arc::new(AtomicU64::new(0));
        let epoch = Instant::now();
        let lua = state(id, entry, &scripts, &settings, deadline.clone(), epoch)?;
        Ok(Arc::new(Self {
            id: id.to_owned(),
            entry: entry.to_owned(),
            settings,
            read,
            lua: RwLock::new(lua),
            guards: Mutex::default(),
            deadline,
            epoch,
        }))
    }

    /// Read the scripts again and run them in a fresh state (`/reload`): their handlers
    /// replace the old ones, and every hook switched off for errors is on again. Scripts that
    /// no longer load leave the old state running, with the error in the log.
    pub fn reload(&self) {
        let fresh = (self.read)().and_then(|scripts| {
            state(
                &self.id,
                &self.entry,
                &scripts,
                &self.settings,
                self.deadline.clone(),
                self.epoch,
            )
        });
        match fresh {
            Ok(lua) => {
                *self.lua.write().unwrap_or_else(PoisonError::into_inner) = lua;
                self.guards
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clear();
            }
            Err(e) => log::error!("mod {}: Lua scripts did not reload: {e}", self.id),
        }
    }

    /// Whether the JIT compiler is on in this state.
    pub fn jit_on(&self) -> bool {
        self.with(|lua| {
            Ok(lua
                .load("return jit ~= nil and jit.status() == true")
                .eval::<bool>()
                .unwrap_or(false))
        })
        .unwrap_or(false)
    }

    /// The `horizon` table's `field` entries, keys sorted.
    pub(crate) fn declared(&self, field: &str) -> Vec<String> {
        self.with(|lua| {
            let table: Table = lua.globals().get::<Table>("horizon")?.get(field)?;
            let mut keys: Vec<String> = table
                .pairs::<String, Value>()
                .filter_map(|pair| pair.ok().map(|(key, _)| key))
                .collect();
            keys.sort();
            Ok(keys)
        })
        .unwrap_or_default()
    }

    /// Run `call` on the state, outside the guard (registration reads).
    fn with<R>(&self, call: impl FnOnce(&Lua) -> mlua::Result<R>) -> Option<R> {
        let lua = self.lua.read().unwrap_or_else(PoisonError::into_inner);
        call(&lua).ok()
    }

    /// Call into the scripts for hook `hook`: `pcall`'s boundary. An error is written to the log
    /// with the mod's id and the script's file and line, and gives `None`; after
    /// `max_errors` in a row the hook is off until `/reload`. A call over the budget is logged;
    /// one over the hard limit is stopped.
    pub(crate) fn call<R>(
        &self,
        hook: &str,
        call: impl FnOnce(&Lua) -> mlua::Result<R>,
    ) -> Option<R> {
        if self
            .guards
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(hook)
            .is_some_and(|guard| guard.off)
        {
            return None;
        }
        let lua = self.lua.read().unwrap_or_else(PoisonError::into_inner);
        let started = Instant::now();
        let stop = (started + self.settings.hard_limit).duration_since(self.epoch);
        self.deadline
            .store(stop.as_nanos().max(1) as u64, Ordering::Relaxed);
        let result = call(&lua);
        self.deadline.store(0, Ordering::Relaxed);
        let elapsed = started.elapsed();
        let mut guards = self.guards.lock().unwrap_or_else(PoisonError::into_inner);
        let guard = guards.entry(hook.to_owned()).or_default();
        if elapsed > self.settings.budget {
            guard.over_budget += 1;
            // The first time, then every power of two: the log shows a slow hook, not each call.
            if guard.over_budget.is_power_of_two() {
                log::warn!(
                    "mod {}: Lua hook {hook} took {elapsed:?}, over its budget of {:?} ({} times)",
                    self.id,
                    self.settings.budget,
                    guard.over_budget
                );
            }
        }
        match result {
            Ok(value) => {
                guard.errors_in_a_row = 0;
                Some(value)
            }
            Err(error) => {
                guard.errors_in_a_row += 1;
                log::error!("mod {}: Lua hook {hook}: {error}", self.id);
                if guard.errors_in_a_row >= self.settings.max_errors {
                    guard.off = true;
                    log::error!(
                        "mod {}: Lua hook {hook} is off until /reload after {} errors in a row",
                        self.id,
                        guard.errors_in_a_row
                    );
                }
                None
            }
        }
    }

    /// The handlers of global hook `name`, in registration order.
    pub(crate) fn global_handlers(lua: &Lua, name: &str) -> mlua::Result<Vec<Function>> {
        let on: Table = lua.globals().get::<Table>("horizon")?.get("_on")?;
        match on.get::<Option<Table>>(name)? {
            Some(list) => list.sequence_values::<Function>().collect(),
            None => Ok(Vec::new()),
        }
    }

    /// The handler for Part hook `hook` of thing `id` in `field` (`_blocks`, `_block_entities`).
    pub(crate) fn part_handler(
        lua: &Lua,
        field: &str,
        id: &str,
        hook: &str,
    ) -> mlua::Result<Option<Function>> {
        let store: Table = lua.globals().get::<Table>("horizon")?.get(field)?;
        match store.get::<Option<Table>>(id)? {
            Some(handlers) => handlers.get(hook),
            None => Ok(None),
        }
    }

    /// JSON to a Lua value: arrays keep their array marker, so they come back as arrays.
    pub(crate) fn to_lua(lua: &Lua, json: &serde_json::Value) -> mlua::Result<Value> {
        lua.to_value(json)
    }

    /// A Lua value back to JSON.
    pub(crate) fn from_lua(lua: &Lua, value: Value) -> mlua::Result<serde_json::Value> {
        lua.from_value(value)
    }
}

/// A fresh state for mod `id`: the safe libraries, the sandbox, the `horizon` table, then the
/// entry script.
fn state(
    id: &str,
    entry: &str,
    scripts: &Scripts,
    settings: &LuaSettings,
    deadline: Arc<AtomicU64>,
    epoch: Instant,
) -> Result<Lua, String> {
    let lua = Lua::new_with(StdLib::ALL_SAFE, mlua::LuaOptions::default())
        .map_err(|e| format!("mod {id}: Lua: {e}"))?;
    let setup = || -> mlua::Result<()> {
        let globals = lua.globals();
        // Determinism and the machine: no clock, no file system, no process, no global random.
        if !settings.allow_io {
            globals.set("io", Value::Nil)?;
            globals.set("os", Value::Nil)?;
        }
        for name in ["dofile", "loadfile"] {
            globals.set(name, Value::Nil)?;
        }
        let math: Table = globals.get("math")?;
        let refuse = lua.create_function(|_, ()| -> mlua::Result<()> {
            Err(mlua::Error::runtime(
                "math.random differs from machine to machine: use horizon.random(seed)",
            ))
        })?;
        math.set("random", refuse.clone())?;
        math.set("randomseed", refuse)?;
        // `require` reads the mod's own scripts, by module name (`util` is `util.lua`).
        let sources = scripts.clone();
        let mod_id = id.to_owned();
        let loaded = lua.create_table()?;
        let require = lua.create_function(move |lua, name: String| -> mlua::Result<Value> {
            if let Some(value) = loaded.get::<Option<Value>>(name.as_str())? {
                return Ok(value);
            }
            let file = format!("{}.lua", name.replace('.', "/"));
            let source = sources.get(&file).ok_or_else(|| {
                mlua::Error::runtime(format!("require: no script {file} in mod {mod_id}"))
            })?;
            let value: Value = lua
                .load(source.as_str())
                .set_name(format!("@data/{mod_id}/scripts/{file}"))
                .eval()?;
            let value = if value.is_nil() {
                Value::Boolean(true)
            } else {
                value
            };
            loaded.set(name.as_str(), value.clone())?;
            Ok(value)
        })?;
        globals.set("require", require)?;
        globals.set(
            "__horizon_check",
            lua.create_function(|_, (name, scoped): (String, bool)| {
                crate::catalog::check(&name, scoped).map_err(mlua::Error::runtime)
            })?,
        )?;
        lua.load(PRELUDE).set_name("@horizon").exec()?;
        let horizon: Table = globals.get("horizon")?;
        horizon.set(
            "random",
            lua.create_function(|_, seed: f64| Ok(crate::random::LuaRandom::new(seed as i64)))?,
        )?;
        let log_id = id.to_owned();
        horizon.set(
            "log",
            lua.create_function(move |_, text: String| {
                log::info!("mod {log_id}: {text}");
                Ok(())
            })?,
        )?;
        // JIT where it runs; the interpreter where it doesn't or the settings say so.
        let jit = match settings.jit {
            JitMode::Off => "if jit then jit.off() end",
            JitMode::Auto => "if jit and not pcall(jit.on) then jit.off() end",
        };
        lua.load(jit).set_name("@horizon").exec()?;
        Ok(())
    };
    setup().map_err(|e| format!("mod {id}: Lua: {e}"))?;
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(10_000),
        move |_, _| {
            let stop = deadline.load(Ordering::Relaxed);
            if stop != 0 && epoch.elapsed().as_nanos() as u64 > stop {
                return Err(mlua::Error::runtime(
                    "the call ran over its hard time limit",
                ));
            }
            Ok(VmState::Continue)
        },
    )
    .map_err(|e| format!("mod {id}: Lua: {e}"))?;
    let source = scripts
        .get(entry)
        .ok_or_else(|| format!("mod {id}: no Lua entry script {entry}"))?;
    lua.load(source.as_str())
        .set_name(format!("@data/{id}/scripts/{entry}"))
        .exec()
        .map_err(|e| format!("mod {id}: {e}"))?;
    Ok(lua)
}
