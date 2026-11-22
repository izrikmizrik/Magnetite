//! `CopyComponentsFunction` in a block's table: the broken block entity's
//! components (`LootPlace::components`) that `include` and `exclude` allow
//! land on the drop.
use crate::items::{BlockLootRoll, Items};
use horizon_core::nbt::{Compound, Tag};
use horizon_core::random::LegacyRandom;
use horizon_world::loot::{LootPlace, PackOverlay, with_native_loot_place};
use horizon_world::registry::{BlockRegistry, BlockStateId};
use std::fs;

const TABLE: &str = r#"{"type":"minecraft:block","pools":[{"rolls":1,"entries":[
    {"type":"minecraft:item","name":"test:pot","modifier":[
        {"type":"minecraft:copy_components","source":"block_entity",
         "include":["custom_name","test:meal","test:container"],"exclude":["test:container"]}]}]}]}"#;

#[test]
fn a_block_table_copies_the_allowed_block_entity_components() {
    let root = std::env::temp_dir().join(format!("horizon-copy-components-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("data/minecraft/loot_table/blocks")).unwrap();
    fs::write(root.join("items.json"), "{}").unwrap();
    fs::write(
        root.join("data/minecraft/loot_table/blocks/pot.json"),
        TABLE,
    )
    .unwrap();
    let defaults = horizon_world::item_defaults::shared(&root).unwrap();
    let items = Items::catalog(&root, &PackOverlay::NONE, &defaults).unwrap();
    let facts = br#"{"defaults":{"minecraft:air":0,"minecraft:pot":1},
        "states":[["air",true,false,"","minecraft:air"],
                  ["pot",false,false,"","minecraft:pot"]]}"#;
    let registry = BlockRegistry::from_facts_json(&facts[..]).unwrap();
    let roll = || {
        items.block_drops(
            &registry,
            BlockStateId(1),
            BlockLootRoll::default(),
            &mut LegacyRandom::new(7),
            None,
        )
    };
    let mut components = Compound::new();
    components.insert(
        "minecraft:custom_name".into(),
        Tag::String("Stew Pot".into()),
    );
    components.insert("test:meal".into(), Tag::Int(3));
    components.insert(
        "test:container".into(),
        Tag::String("minecraft:bowl".into()),
    );
    components.insert("test:heat".into(), Tag::Int(9));
    let place = LootPlace {
        components: Some(components),
        ..LootPlace::default()
    };
    let drops = with_native_loot_place(place, roll);
    assert_eq!(drops.len(), 1);
    let data = drops[0]
        .data()
        .expect("the drop carries the copied components");
    assert_eq!(data.custom_name, Some(Tag::String("Stew Pot".into())));
    assert_eq!(data.other("test:meal"), Some(&Tag::Int(3)));
    // Excluded, and not included: neither is copied.
    assert_eq!(data.other("test:container"), None);
    assert_eq!(data.other("test:heat"), None);
    // Without a block entity's components the drop is the plain item.
    let plain = roll();
    assert!(
        plain[0]
            .data()
            .is_none_or(|d| d.other("test:meal").is_none())
    );
    fs::remove_dir_all(root).unwrap();
}
