//! Mods' load-time transforms of the recipe files: a recipe with an `OR` group of ingredients
//! becomes one recipe per alternative, before the loader reads any of them.
use super::*;

/// Splits a recipe whose `ingredients` hold one `{"or": [...]}` group into one recipe per
/// alternative, `<id>_<n>`.
struct SplitOr;
impl mod_api::DataTransform for SplitOr {
    fn transform(
        &self,
        mut data: mod_api::DataFiles,
    ) -> std::result::Result<mod_api::DataFiles, String> {
        let mut out = Vec::new();
        for (id, text) in &data.files {
            let doc: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
            let Some(group) = doc["ingredients"][0]["or"].as_array() else {
                out.push((id.clone(), text.clone()));
                continue;
            };
            for (n, alternative) in group.iter().enumerate() {
                let mut split = doc.clone();
                split["ingredients"][0] = alternative.clone();
                out.push((format!("{id}_{n}"), split.to_string()));
            }
        }
        data.files = out;
        Ok(data)
    }
}
struct Salvage;
#[mod_api::__subscriptions]
impl mod_api::Hooks for Salvage {
    fn register(&self, r: &mut dyn mod_api::Registrar, _: &mod_api::RegistrationContext) {
        r.register_data_transform(mod_api::DataFiles::RECIPE, SplitOr);
    }
}

#[test]
fn an_or_group_recipe_becomes_one_recipe_per_alternative() {
    let mods = vec![mod_api::ModEntry::new("salvage", Box::new(Salvage))];
    mod_platform::testing::with_mods(mods.into(), || {
        mod_platform::__private::part_tests::install_parts_fixture(&[mod_api::ModEntry::new(
            "salvage",
            Box::new(Salvage),
        )])
        .unwrap();
        let sources = vec![
            (
                "stick".to_owned(),
                serde_json::json!({"type": "minecraft:crafting_shapeless"}),
            ),
            (
                "salvage:saddle".to_owned(),
                serde_json::json!({"type": "salvage:cutting",
                    "ingredients": [{"or": ["minecraft:saddle", "minecraft:leather"]}]}),
            ),
        ];
        let out = transformed_recipes(sources);
        let names: Vec<&str> = out.iter().map(|(name, _)| name.as_str()).collect();
        // Vanilla's bare name comes back bare; the split recipes take the group's place.
        assert_eq!(names, ["stick", "salvage:saddle_0", "salvage:saddle_1"]);
        assert_eq!(out[2].1["ingredients"][0], "minecraft:leather");
    });
}
