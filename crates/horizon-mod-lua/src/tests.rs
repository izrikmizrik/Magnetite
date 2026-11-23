//! The Lua layer against the ТЗ's acceptance: a hook gives the same result from Lua as from a
//! native (or wasm) mod, a script's error is logged and contained, and the result is the same
//! with the JIT compiler on and with `jit.off()`.
use super::*;
use runtime::JitMode;

fn script(source: &str, jit: JitMode) -> Arc<Script> {
    let scripts: Scripts = [("main.lua".to_owned(), source.to_owned())].into();
    let again = scripts.clone();
    Script::load(
        "luatest",
        "main.lua",
        scripts,
        Box::new(move || Ok(again.clone())),
        LuaSettings {
            jit,
            max_errors: 3,
            ..LuaSettings::default()
        },
    )
    .unwrap()
}

/// A script's own computation: seeded draws, sorted walks, float math and string building.
const WORK: &str = r#"
function work()
  local r = horizon.random(42)
  local t = {}
  for i = 1, 2000 do t["k" .. r:next_int(500)] = (t["k" .. i % 500] or 0) + math.sin(i) * r:next_double() end
  local out = {}
  for k, v in horizon.sorted_pairs(t) do out[#out + 1] = k .. "=" .. string.format("%.12f", v) end
  return table.concat(out, ";")
end
"#;

#[test]
fn the_same_script_gives_the_same_result_with_the_jit_and_without() {
    let run = |jit| {
        let s = script(WORK, jit);
        let jit_on = s.jit_on();
        let out = s
            .call("test", |lua| {
                lua.globals().get::<Function>("work")?.call::<String>(())
            })
            .unwrap();
        (jit_on, out)
    };
    let (_, auto) = run(JitMode::Auto);
    let (off_jit, off) = run(JitMode::Off);
    assert!(!off_jit, "jit.off() leaves the interpreter");
    assert_eq!(auto, off);
    // The same seed draws the same numbers again.
    assert_eq!(run(JitMode::Auto).1, auto);
}

#[test]
fn a_scripts_error_is_contained_and_switches_the_hook_off_after_repeats() {
    let s = script(
        "function boom() error('broken pan') end\nfunction fine() return 7 end",
        JitMode::Auto,
    );
    let call = |name: &'static str| {
        s.call(name, |lua| {
            lua.globals().get::<Function>(name)?.call::<i64>(())
        })
    };
    // An error gives nothing and the game goes on; another hook is untouched.
    for _ in 0..3 {
        assert_eq!(call("boom"), None);
    }
    assert_eq!(call("fine"), Some(7));
    // After three errors in a row `boom` is off: the script is not even asked.
    let asked = std::sync::atomic::AtomicBool::new(false);
    assert_eq!(
        s.call("boom", |_| {
            asked.store(true, std::sync::atomic::Ordering::Relaxed);
            Ok(1)
        }),
        None
    );
    assert!(!asked.load(std::sync::atomic::Ordering::Relaxed));
    // `/reload` switches it on again.
    s.reload();
    assert_eq!(s.call("boom", |_| Ok(1)), Some(1));
}

#[test]
fn a_runaway_script_is_stopped_at_the_hard_limit() {
    let s = script("function spin() while true do end end", JitMode::Off);
    let started = std::time::Instant::now();
    assert_eq!(
        s.call("spin", |lua| lua
            .globals()
            .get::<Function>("spin")?
            .call::<()>(())),
        None
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[test]
fn the_sandbox_has_no_clock_files_or_global_random() {
    let s = script(
        r#"
        function probe()
          local ok = pcall(math.random)
          return tostring(os) .. "," .. tostring(io) .. "," .. tostring(ok) .. "," .. tostring(dofile)
        end
        "#,
        JitMode::Auto,
    );
    assert_eq!(
        s.call("probe", |lua| lua
            .globals()
            .get::<Function>("probe")?
            .call::<String>(()))
            .unwrap(),
        "nil,nil,false,nil"
    );
}

#[test]
fn a_hook_name_outside_the_catalog_refuses_the_script() {
    let scripts: Scripts = [(
        "main.lua".to_owned(),
        "horizon.on('no_such_hook', function() end)".to_owned(),
    )]
    .into();
    let error = Script::load(
        "luatest",
        "main.lua",
        scripts,
        Box::new(|| Err("unused".into())),
        LuaSettings::default(),
    )
    .err()
    .unwrap();
    assert!(error.contains("no_such_hook is not a hook"), "{error}");
    assert!(error.contains("main.lua:1"), "{error}");
}

/// The same biome-modifier transform written natively and as a script: the files that come out
/// of the game's transform runner are the same, byte for byte.
#[test]
fn a_transform_gives_the_same_files_from_lua_as_from_a_native_mod() {
    struct Native;
    impl mod_api::DataTransform for Native {
        fn transform(&self, mut data: mod_api::DataFiles) -> Result<mod_api::DataFiles, String> {
            for (_, text) in &mut data.files {
                let mut doc: serde_json::Value = serde_json::from_str(text).unwrap();
                if doc["type"] == "wild:add_features_by_filter" {
                    let tag = doc["allowed_biomes"]
                        .as_str()
                        .unwrap()
                        .trim_start_matches('#');
                    let biomes = &data.biome_tags.iter().find(|(id, _)| id == tag).unwrap().1;
                    doc = serde_json::json!({"type": "neoforge:add_features", "biomes": biomes,
                        "features": doc["features"], "step": doc["step"]});
                }
                *text = doc.to_string();
            }
            Ok(data)
        }
    }
    const LUA: &str = r#"
    horizon.data_transform("biome_modifier", function(files, context)
      for id, doc in horizon.sorted_pairs(files) do
        if doc.type == "wild:add_features_by_filter" then
          local tag = doc.allowed_biomes:gsub("^#", "")
          files[id] = { type = "neoforge:add_features", biomes = context.biome_tags[tag],
            features = doc.features, step = doc.step }
        end
      end
      return files
    end)
    "#;
    let input = || {
        let mut data = mod_api::DataFiles::new(
            mod_api::DataFiles::BIOME_MODIFIER,
            vec![(
                "wild:wild_rice".into(),
                r##"{"type":"wild:add_features_by_filter","allowed_biomes":"#minecraft:is_river","features":"wild:patch","step":"vegetal_decoration"}"##.into(),
            )],
        );
        data.biome_tags = vec![(
            "minecraft:is_river".into(),
            vec!["minecraft:river".into(), "minecraft:frozen_river".into()],
        )];
        data
    };
    let lua = LuaTransform {
        script: script(LUA, JitMode::Auto),
        kind: "biome_modifier".into(),
    };
    let from_lua: serde_json::Value = serde_json::from_str(
        &mod_api::DataTransform::transform(&lua, input())
            .unwrap()
            .files[0]
            .1,
    )
    .unwrap();
    let from_native: serde_json::Value = serde_json::from_str(
        &mod_api::DataTransform::transform(&Native, input())
            .unwrap()
            .files[0]
            .1,
    )
    .unwrap();
    assert_eq!(from_lua, from_native);
    assert_eq!(
        from_lua["biomes"],
        serde_json::json!(["minecraft:river", "minecraft:frozen_river"])
    );
}

/// No registries, tags or settings: a loot table hook here reads only its table.
struct NoReader;
impl mod_api::LoadReader for NoReader {
    fn registry(&self, _: &str) -> Option<Arc<[String]>> {
        None
    }
    fn fluid_tags(&self) -> Arc<[(String, Arc<[String]>)]> {
        Arc::from([])
    }
    fn setting(&self, _: &str, _: &str) -> Option<String> {
        None
    }
}

/// `loot_table_load` through the game's own dispatch: a native mod and a Lua mod that add the
/// same pool leave the same table, and the Lua mod is installed and asked like any other.
#[test]
fn the_same_hook_through_the_games_dispatch_gives_the_same_table_from_lua_and_native() {
    struct Native;
    #[mod_api::__subscriptions]
    impl mod_api::Hooks for Native {
        fn loot_table_load(
            &self,
            _: &dyn mod_api::DataLoadContext,
            event: &mut mod_api::LootTableLoadEvent<'_>,
        ) {
            if event.id == "minecraft:chests/simple_dungeon" {
                event.table["pools"].as_array_mut().unwrap().push(serde_json::json!({
                    "rolls": 1, "entries": [{"type": "minecraft:item", "name": "minecraft:onion"}]
                }));
            }
        }
    }
    const LUA: &str = r#"
    horizon.on("loot_table_load", function(id, table)
      if id == "minecraft:chests/simple_dungeon" then
        table.pools[#table.pools + 1] = { rolls = 1,
          entries = { { type = "minecraft:item", name = "minecraft:onion" } } }
      end
      return table
    end)
    "#;
    let table = || serde_json::json!({"pools": [{"rolls": 2, "entries": []}]});
    let dispatch = |entry: mod_api::ModEntry| {
        let mut out = table();
        mod_api::testing::with_mods(vec![entry].into(), || {
            let reader = NoReader;
            let context =
                mod_api::OwningLoad::new(mod_api::DataLoadOrigin::ServerDataLoad, &reader);
            mod_api::dispatch_loot_table_load(
                &context,
                "minecraft:chests/simple_dungeon",
                &mut out,
            );
        });
        out
    };
    let scripts: Scripts = [("main.lua".to_owned(), LUA.to_owned())].into();
    let lua = mod_entry(
        "luatest",
        "main.lua",
        scripts,
        Box::new(|| Err("unused".into())),
        LuaSettings::default(),
    )
    .unwrap();
    let from_lua = dispatch(lua);
    let from_native = dispatch(mod_api::ModEntry::new("native", Box::new(Native)));
    assert_eq!(from_lua, from_native);
    assert_eq!(from_lua["pools"].as_array().unwrap().len(), 2);
}

/// The cost of one guarded call into a script (`cargo test -p horizon-mod-lua --release --
/// --ignored --nocapture call_overhead`): the figure the ТЗ asks the report to give.
#[test]
#[ignore = "a measurement, not a check"]
fn call_overhead() {
    for jit in [JitMode::Auto, JitMode::Off] {
        let s = script("function tick(ctx) ctx.n = ctx.n + 1 end", jit);
        let calls = 200_000;
        let started = std::time::Instant::now();
        for _ in 0..calls {
            s.call("tick", |lua| {
                let ctx = lua.create_table()?;
                ctx.set("n", 1)?;
                lua.globals().get::<Function>("tick")?.call::<()>(ctx)
            });
        }
        println!("{jit:?}: {:?} per hook call", started.elapsed() / calls);
    }
}
