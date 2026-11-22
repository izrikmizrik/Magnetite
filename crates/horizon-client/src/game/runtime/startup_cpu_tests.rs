use super::*;

/// A sink is the game's once: a second one is refused, and none comes back after the first was
/// taken. A process has one event loop, and the app runs in the caller's it is handed to.
#[test]
fn the_app_sink_is_registered_once_and_taken_once() {
    fn first(_app: App) {}
    fn second(_app: App) {}
    fn later(_app: App) {}
    assert!(take_app_sink().is_none(), "none given yet");
    set_app_sink(first);
    // The first sink stays the game's; the second is never the one taken.
    set_app_sink(second);
    let taken = take_app_sink().expect("the sink given");
    assert!(
        std::ptr::fn_addr_eq(taken, first as fn(App)),
        "the first sink is the game's, not a later one"
    );
    assert!(take_app_sink().is_none(), "taken once");
    set_app_sink(later);
    assert!(take_app_sink().is_none(), "not after the app was given");
}

#[test]
fn a_folder_without_prepared_files_is_named_before_loading() {
    let root = std::env::temp_dir().join(format!("horizon-unprepared-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let unprepared = root.join(".local/horizon-client/26.3");
    let missing = missing_resources(&unprepared, None).expect("no manifest is reported");
    assert!(missing.contains("horizon-resources.json"), "{missing}");
    assert!(missing.contains("horizon-unprepared-"), "{missing}");
    assert!(missing.contains("horizon-prepare --prepare"), "{missing}");
    // No hook: no site, no launcher.
    assert!(!missing.contains("playhorizon"), "{missing}");
    assert!(!missing.contains("launcher"), "{missing}");
    let get = horizon_hooks::GetTheGame {
        address: "https://example.org/get",
        title: "",
        paragraphs: &[],
        button: "",
    };
    let missing = missing_resources(&unprepared, Some(get)).expect("no manifest is reported");
    assert!(
        missing.contains("launcher: download it at https://example.org/get"),
        "{missing}"
    );
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("horizon-resources.json"), "{}").unwrap();
    assert_eq!(missing_resources(&root, None), None);

    // A killed export leaves an empty file behind; the error names it
    // instead of serde's "expected value at line 1 column 1".
    std::fs::write(root.join("gameplay-facts.json"), "").unwrap();
    let damaged = missing_resources(&root, None).expect("an empty prepared file is reported");
    assert!(damaged.contains("gameplay-facts.json"), "{damaged}");
    assert!(damaged.contains("damaged"), "{damaged}");
    std::fs::write(root.join("gameplay-facts.json"), "{}").unwrap();
    assert_eq!(missing_resources(&root, None), None);

    std::fs::write(root.join("horizon-resources.json"), " \n").unwrap();
    let damaged = missing_resources(&root, None).expect("an empty manifest is reported");
    assert!(damaged.contains("horizon-resources.json"), "{damaged}");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn worker_prepares_owned_models_and_inventory_before_handoff() {
    let root = std::env::temp_dir().join(format!("horizon-startup-cpu-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("entity-models.json"),
        r#"{
        "minecraft:end_crystal#main": {}, "minecraft:ender_dragon#main": {},
        "minecraft:arrow#main": {}, "minecraft:trident#main": {},
        "minecraft:armor_stand#main": {}, "minecraft:cushion#main": {},
        "minecraft:leash_knot#main": {}, "minecraft:wind_charge#main": {},
        "minecraft:shulker_bullet#main": {}, "minecraft:wither_skull#main": {},
        "minecraft:evoker_fangs#main": {}, "minecraft:standing_banner#main": {},
        "minecraft:standing_banner#flag": {}
    }"#,
    )
    .unwrap();
    std::fs::write(
        root.join("creative-tabs.json"),
        r#"[{
        "id":"minecraft:building_blocks", "items":["minecraft:stone"]
    }]"#,
    )
    .unwrap();
    let worker_root = root.clone();
    let prepared = std::thread::spawn(move || {
        GameCpuResources::load(
            &horizon_assets::resource_source::ResourceRoot::directory(worker_root),
            None,
        )
        .map_err(|error| error.to_string())
    })
    .join()
    .unwrap()
    .unwrap();
    std::fs::remove_dir_all(&root).unwrap();
    assert!(prepared.crystal_model.is_some());
    assert!(prepared.cushion_model.is_some());
    let models = &prepared.entity_models;
    assert!(models.leash_knot.is_some() && models.wind_charge.is_some());
    assert!(models.shulker_bullet.is_some() && models.wither_skull.is_some());
    assert!(models.evoker_fangs.is_some() && models.banner.is_some());
    assert!(prepared.dragon_model.is_some());
    assert!(prepared.arrow_model.is_some());
    assert!(prepared.trident_model.is_some());
    assert!(prepared.armor_stand_assets.main.is_some());
    assert_eq!(
        prepared.creative_screen.tabs[0].items[0].item.as_ref(),
        "minecraft:stone"
    );
    // No documents from the old generation remain available to reload.
    assert!(
        GameCpuResources::load(
            &horizon_assets::resource_source::ResourceRoot::directory(root.clone()),
            None
        )
        .is_err()
    );
}
