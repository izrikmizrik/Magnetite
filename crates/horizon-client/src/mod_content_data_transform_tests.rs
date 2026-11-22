//! Mods' load-time transforms of biome modifiers: a filter modifier of a mod's own type becomes
//! feature modifiers the game reads, against the world's biomes and resolved tags.
use super::*;

/// Rewrites `wild:add_features_by_filter` (Farmer's Delight's shape: `allowed_biomes`,
/// optional `denied_biomes`, `features`, `step`) into `neoforge:add_features` with an explicit
/// biome list, in the order the allowed tag gives them.
struct ByFilter;
impl mod_api::DataTransform for ByFilter {
    fn transform(
        &self,
        mut data: mod_api::DataFiles,
    ) -> std::result::Result<mod_api::DataFiles, String> {
        let tag = |set: &serde_json::Value| -> Vec<String> {
            let id = set.as_str().unwrap_or_default();
            let id = id.strip_prefix('#').unwrap_or(id);
            data.biome_tags
                .iter()
                .find(|(tag, _)| tag == id)
                .map(|(_, members)| members.clone())
                .unwrap_or_default()
        };
        let mut out = Vec::new();
        for (id, text) in &data.files {
            let doc: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
            if doc["type"] != "wild:add_features_by_filter" {
                out.push((id.clone(), text.clone()));
                continue;
            }
            let denied = tag(&doc["denied_biomes"]);
            let biomes: Vec<String> = tag(&doc["allowed_biomes"])
                .into_iter()
                .filter(|b| !denied.contains(b))
                .collect();
            let rewritten = serde_json::json!({
                "type": "neoforge:add_features",
                "biomes": biomes,
                "features": doc["features"],
                "step": doc["step"],
            });
            out.push((id.clone(), rewritten.to_string()));
        }
        data.files = out;
        Ok(data)
    }
}
struct Wild;
#[mod_api::__subscriptions]
impl mod_api::Hooks for Wild {
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        r.register_data_transform(mod_api::DataFiles::BIOME_MODIFIER, ByFilter);
    }
}

fn biome(temperature: f64) -> J {
    J::Str(format!(r#"{{"temperature":{temperature},"features":[]}}"#))
}

#[test]
fn a_filter_modifier_becomes_feature_modifiers_the_game_reads() {
    let mods = vec![mod_api::ModEntry::new("wild", Box::new(Wild))];
    mod_platform::testing::with_mods(mods.into(), || {
        mod_platform::__private::part_tests::install_parts_fixture(&[mod_api::ModEntry::new(
            "wild",
            Box::new(Wild),
        )])
        .unwrap();
        // The shape of Farmer's Delight's `wild_rice.json`.
        let rice = r##"{"allowed_biomes":"#c:is_wet/overworld","denied_biomes":"#c:is_underground",
            "features":"wild:patch_wild_rice","step":"vegetal_decoration",
            "type":"wild:add_features_by_filter"}"##;
        let kept = r#"{"type":"neoforge:add_features","biomes":"minecraft:plains",
            "features":"other:flower","step":"vegetal_decoration"}"#;
        let sources = vec![
            [
                "other".to_owned(),
                "data/other/neoforge/biome_modifier/flower.json".to_owned(),
                kept.to_owned(),
            ],
            [
                "wild".to_owned(),
                "data/wild/neoforge/biome_modifier/wild_rice.json".to_owned(),
                rice.to_owned(),
            ],
        ];
        let doc = J::Obj(vec![
            ("minecraft:plains".into(), biome(0.8)),
            ("minecraft:river".into(), biome(0.5)),
            ("minecraft:swamp".into(), biome(0.8)),
            ("minecraft:lush_caves".into(), biome(0.5)),
        ]);
        let tags: BTreeMap<String, String> = [
            (
                "c:is_wet/overworld",
                r##"{"values":["minecraft:swamp","#minecraft:is_river","minecraft:lush_caves"]}"##,
            ),
            ("minecraft:is_river", r#"{"values":["minecraft:river"]}"#),
            ("c:is_underground", r#"{"values":["minecraft:lush_caves"]}"#),
        ]
        .into_iter()
        .map(|(id, text)| (id.to_owned(), text.to_owned()))
        .collect();
        let out = transformed_modifiers(sources, &doc, &tags);
        // Order and files are kept; the filter is now a feature modifier.
        assert_eq!(out[0][1], "data/other/neoforge/biome_modifier/flower.json");
        assert_eq!(
            out[1][1],
            "data/wild/neoforge/biome_modifier/wild_rice.json"
        );
        let rewritten: serde_json::Value = serde_json::from_str(&out[1][2]).unwrap();
        assert_eq!(
            rewritten["biomes"],
            serde_json::json!(["minecraft:swamp", "minecraft:river"])
        );
        // Both read with no warning: nothing is skipped.
        let mut warnings = Vec::new();
        for [m, file, text] in &out {
            assert!(
                biomes::Modifier::read(m, file, text, &mut warnings)
                    .unwrap()
                    .is_some()
            );
        }
        assert!(warnings.is_empty(), "{warnings:?}");
        // Run twice, the same files come out.
        assert_eq!(out, transformed_modifiers(out.clone(), &doc, &tags));
    });
}

/// A modifier of a type the content build doesn't read waits for the world open.
#[test]
fn a_mods_own_modifier_type_is_deferred_not_skipped() {
    assert!(!biomes::read_at_build(
        r#"{"type":"wild:add_features_by_filter"}"#
    ));
    assert!(biomes::read_at_build(r#"{"type":"neoforge:add_features"}"#));
    assert!(biomes::read_at_build(r#"{"type":"forge:remove_spawns"}"#));
    assert_eq!(
        biomes::modifier_id("data/wild/neoforge/biome_modifier/wild_rice.json"),
        "wild:wild_rice"
    );
}
