# Mod function reference

Generated from the mod API's sources by `tools/modding_reference.py` (do not edit by hand).
Every catalog hook and every `Host` method: its name, side, contract and one line on what
it does. The full documentation with parameters is the rustdoc of `horizon-mod-api`
(`cargo doc -p horizon-mod-api --no-deps --open`). How it all fits together: the
[modding guide](modding.en.md). По-русски: [modding-reference.md](modding-reference.md).

Hooks: **761**, Host methods: **364**.

**A hook's contract:** *listen*: told of the event; *cancel*: may stop the game's
action; *collect*: every mod's answers are gathered; *replace*: the first mod to answer
replaces vanilla's answer.

## Hooks

### Global hooks (`Hooks`)

#### Startup, settings and config

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `client_setup` | client | listen | NeoForge's `FMLClientSetupEvent`: the client has started. |
| `client_stopped` | client | listen | The client has finished shutting down, after `client_stopping`. |
| `client_stopping` | client | listen | The client is closing: it fires once per game process, on the render thread, when the game shuts down, after the world the player was in is left and saved, while the game still holds its settings and resources, so a listener can read them through the `ClientHost` and save its own client state. |
| `component_types` | both | collect | Register namespaced NBT component types before content loads. |
| `config_loading` | both | listen | A registered config was read (`ModConfigEvent.Loading`): `config` says whose, which file and its text, which the mod parses (TOML by convention). |
| `config_reloading` | both | listen | A loaded config's file changed on disk (`ModConfigEvent.Reloading`): its new text. |
| `config_unloading` | server | listen | A `Server` config is dropped as its server stops (`ModConfigEvent.Unloading`), after `server_stopping`; the next world reads it again. |
| `console_line` | server | listen | A line the server logged (what its console prints and `logs/latest.log` keeps): `level` is `INFO`, `WARN` or `ERROR`, `thread` the thread it names (`Server thread`), `message` the line without its time and thread. |
| `default_components_bound` | both | listen | Items' default components are bound and readable (NeoForge's `DefaultDataComponentsBoundEvent`); `cause` says how, as `tags_updated`'s does. |
| `difficulty_changed` | server | listen | The server's difficulty changed, from `from` to `to` (`peaceful`, `easy`, `normal`, `hard`; NeoForge's `DifficultyChangeEvent`). |
| `extend_poi_types` | server | collect | Add block states to an existing POI type (NeoForge `ExtendPoiTypesEvent`): a mod's bed to `minecraft:home`, its workstation to `minecraft:armorer`. |
| `game_rule_changed` | server | listen | A server game rule `rule` (`minecraft:keep_inventory`) was set to `value` (`GameRuleChangedEvent`, from `GameRules.set` with a server: `/gamerule`, the game rules screen, the management protocol, a test environment, `server.properties`). |
| `game_shutting_down` | both | listen | The game process is shutting down, after the client or dedicated server has stopped. |
| `map_decoration_types` | both | collect | The map decoration types a mod adds (NeoForge's `RegisterEvent` for `Registries.MAP_DECORATION_TYPE`): push a `MapDecorationType` per type. |
| `mod_mismatch` | both | collect | A saved mod list (code and content mods) was read while opening a world. |
| `permissions_changed` | server | cancel | NeoForge's `PermissionsChangedEvent`: the operator list is about to give player `player` (`name`) the permission level `new` in place of `old`, through `PlayerList.op` (`/op`, the console's `op`, the management API's `operators/add` and `operators/set`) or `PlayerList.deop` (`/deop` and the same on the console and the management API; `new` is then `All`). |
| `scheduled_task` | server | listen | A task a mod scheduled with `schedule_task` is due, at the start of the server tick it was due on, after `server_tick`'s start phase, in the order the tasks were scheduled. |
| `setting_changed` | client | listen | The player changed setting `key` of mod `mod_id` on its page in Options > Mods (`Registrar::register_setting`); the new value is kept already, so read it with `setting_bool` and its siblings. |
| `whitelist_state_update` | server | listen | The dedicated server's whitelist changed to `enabled`. |

#### Registration and data loading

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `add_attribute_tooltips` | client | listen | `AddAttributeTooltipsEvent`: the lines that follow a stack's attribute lines (`ItemStack.addAttributeTooltips`), after the modifiers the stack shows and `gather_skipped_attribute_tooltips` left in. |
| `add_client_reload_listeners` | client | listen | Register client asset reload listeners before a reload stages its assets (NeoForge's `AddClientReloadListenersEvent`, the client's counterpart of `add_server_reload_listeners`): the first load on the loader thread, then every later reload — the player changed resource packs, F3+T was pressed, or a server sent its packs. |
| `add_pack_finders` | both | listen | Add discoverable resource or data packs before a repository selects its packs (`AddPackFindersEvent`). |
| `add_section_geometry` | client | listen | Extra geometry in chunk section meshes (NeoForge's `AddSectionGeometryEvent`): `register(renderer)` adds the mod's own `SectionGeometry`, which the mesher asks to add its quads every time it meshes a section, on its worker threads. |
| `add_server_reload_listeners` | server | listen | Register server data reload listeners after packs are selected. |
| `build_creative_mode_tab_contents` | client | collect | The stacks a mod adds to the creative tab `tab` (`minecraft:ingredients`, or another mod's `<mod id>:<name>`) while the tabs' contents are built (NeoForge's `BuildCreativeModeTabContentsEvent`, during vanilla's `CreativeModeTabs.buildAllTabContents`): push each stack to `items`, in order (`accept`). |
| `create_fluid_source` | server | listen | A fluid source at `pos` (`state`) is about to count towards turning the flowing fluid beside it into a source (NeoForge's `CreateFluidSourceEvent`, `FlowingFluid.getNewLiquid`): two counted sources beside a block over a solid block or a source make it one. |
| `create_skull_models` | client | collect | Skull models for a mod's own skull types (NeoForge's `EntityRenderersEvent.CreateSkullModels`): push a `SkullModel` per type, its `kind`, the model layer it is baked from and its default texture. |
| `create_spawn_position` | server | cancel | Before a new server level selects its initial world spawn (NeoForge's `LevelEvent.CreateSpawnPosition`). |
| `data_map_types` | server | collect | The server is assembling its data tables, as for `data_pack_files` (a world's first open and every `/reload`): push the data map types your mod registers (NeoForge's `RegisterDataMapTypesEvent`). |
| `data_maps_updated` | both | listen | The data maps of a registry were reloaded on the server or synced to the client (NeoForge's `DataMapsUpdatedEvent`): the place to build caches from the new values or post-process them. |
| `data_pack_files` | server | collect | The server is assembling its data tables from the world's data packs (`ReloadableServerResources`): a world is opening, or `/reload` was run, and the recipes, tags, loot and advancements are built after this. |
| `datapack_sync` | server | collect | Collect recipe types once the joining player exists, after admission and before that player's recipes are sent. |
| `gather_effect_screen_tooltips` | client | listen | `GatherEffectScreenTooltipsEvent`: the tooltip of the effect panel the cursor is over beside the survival or creative inventory. |
| `gather_skipped_attribute_tooltips` | client | listen | `GatherSkippedAttributeTooltipsEvent`: the attribute modifiers a stack's tooltip leaves out (`ItemStack.addAttributeTooltips`), asked once before its attribute lines are built. |
| `loot_condition` | server | collect | A loot condition of a mod's own kind: the entry named a `type` in a namespace outside `minecraft:`. |
| `loot_drops` | server | collect | A block broke or a mob died, and `drops` are the items about to drop, in the order they drop: what the loot tables rolled (after `loot_table_load`'s changes), and for a mob what it wore, held or carried (a fox's mouth item, an allay's, piglin's or horse's inventory), but not an enderman's carried block or a copper golem's preserved equipment, which drop after. |
| `loot_function` | server | collect | A loot function of a mod's own kind: the entry named a `type` in a namespace outside `minecraft:`, and the roll kept it for this call. |
| `loot_table_load` | server | collect | A native mod's hook point (docs/modding/src/how-to/native-mods.md), declared in `hooks/*.rs` with the game's: the loot table `id` (`minecraft:chests/simple_dungeon`, `minecraft:blocks/stone`, `minecraft:entities/zombie`) as it loads, its JSON after the data packs', before anything rolls it (NeoForge's `LootTableLoadEvent`, Fabric's `LootTableEvents.MODIFY`). |
| `modify_custom_spawners` | server | listen | NeoForge's `ModifyCustomSpawnersEvent`: the level `dimension` is about to tick its custom spawners for the first time. |
| `modify_default_components` | both | collect | NeoForge 26.3's ModifyDefaultComponentsEvent: set or remove entries in `components`, the default component map for `item`, including vanilla items. |
| `recipe_book_categories` | both | listen | Supply categories and vanilla-shaped displays for custom recipe types. |
| `recipe_property_sets` | both | collect | Items a mod's own recipe type takes in a station slot, added to the recipe property sets (`RecipeManager.RECIPE_PROPERTY_SETS`): what a furnace, blast furnace, smoker, campfire, smithing table slot or brewing stand slot accepts, and what a client predicts it accepts. |
| `recipe_sources` | server | collect | Additional ingredient containers for server player `player` placing or checking a recipe in the mod crafting menu `menu` (`namespace:path`) at `pos`. |
| `recipes_received` | client | listen | The server's recipes of `recipe_types` arrived (NeoForge's `RecipesReceivedEvent`): from now on the recipe helpers (`native_recipes`, `native_recipes_of_type`) give the server's recipes of those types in place of the window's own, so rebuild what you made of them. |
| `register_block_state_models` | client | collect | The block-state definition types a mod adds (NeoForge's `RegisterBlockStateModels`, its `registerDefinition`): push each type id, `<mod id>:<path>`. |
| `register_block_tint_sources` | client | listen | The tint sources of blocks (NeoForge's `RegisterColorHandlersEvent.BlockTintSources`, vanilla's `BlockColors.register`): `register(layers, blocks)`, and a model face with `tintindex` i is multiplied by layer i of its block's list; a face past the list is not tinted. |
| `register_cauldron_fluid_content` | server | collect | Say which fluid, and how much, one of your cauldron blocks holds (NeoForge `RegisterCauldronFluidContentEvent`), so the fluid capability (`FLUIDS`) fills and drains it as it does vanilla's water and lava cauldrons. |
| `register_cauldron_interaction` | both | listen | Register cauldron item interactions, by exact item or by item tag, on both sides before snapshot publication. |
| `register_client_commands` | client | collect | The client's own commands (NeoForge's `RegisterClientCommandsEvent`), which `command` runs: push a `Command` per usage (`x <mode:word>`). |
| `register_color_resolvers` | client | listen | The colour resolvers block tints blend over the biomes around a block (NeoForge's `RegisterColorHandlersEvent.ColorResolvers`): `register(resolver)` adds the mod's own `BiomeColorResolver` and returns the `ColorResolver` its block tint sources pass to `BlockAndTintGetter::block_tint`. |
| `register_commands` | server | collect | The commands native mods add to the server's command tree: push yours. |
| `register_configs` | both | collect | The config files native mods keep (NeoForge's `registerConfig`): push yours, each of a kind, with the text written to `config/` when the file is missing. |
| `register_debug_entries` | client | collect | The F3 debug entries a mod adds (NeoForge's `RegisterDebugEntriesEvent`): push a `DebugEntry` per entry, its id `<mod id>:<path>` and the vanilla group its lines join. |
| `register_debug_renderers` | client | collect | The world-space debug renderers a mod has now (NeoForge's `RegisterDebugRenderersEvent`): push a `DebugRenderer` per renderer. |
| `register_dimension_transition_screen` | client | collect | Register loading screens once at client startup (NeoForge's `RegisterDimensionTransitionScreenEvent`). |
| `register_enchantment_damage_effects` | server | collect | Register namespaced enchantment damage value effects once before content loads. |
| `register_entity_layers` | client | listen | The layers a mod adds to living entity renderers (NeoForge's `EntityRenderersEvent.AddLayers`): `add(target, layer)`, and every living entity of the type, or every player of the player model, in view draws `EntityRenderLayer::submit` after its model and the layers added before. |
| `register_entity_renderers` | client | listen | The code-defined renderers of entity types (NeoForge's `EntityRenderersEvent.RegisterRenderers`, `registerEntityRenderer`): `register(type, renderer)`, and every entity of the type `type` (`mymod:bolt`) in view is drawn by `EntityRenderer::submit` in place of the type's own drawing. |
| `register_entity_spectator_shaders` | client | collect | Register the screen effect spectating an entity type puts on, once at client startup (NeoForge's `RegisterEntitySpectatorShadersEvent`), as vanilla's creeper, spider and enderman do. |
| `register_fluid_models` | client | collect | The models of your world fluids (NeoForge's `RegisterFluidModelsEvent`): push a `mod_api::FluidModel` per fluid whose look doesn't say all of it. |
| `register_game_tests` | server | listen | Register namespaced test functions before the world's test definitions load. |
| `register_gui_layers` | client | collect | The HUD layers a mod adds among vanilla's (NeoForge's `RegisterGuiLayersEvent`): push a `GuiLayer` per layer, its id `<mod id>:<path>` and its `GuiLayerPlacement`. |
| `register_item_models` | client | collect | The item model node types a mod adds (NeoForge's `RegisterItemModelsEvent`): push each type id, `<mod id>:<path>`. |
| `register_item_tint_sources` | client | collect | The item tint sources a mod adds (NeoForge's `RegisterColorHandlersEvent.ItemTintSources`): push each id, `<mod id>:<path>`. |
| `register_json_animation_types` | client | collect | The animation targets and interpolations a mod adds for keyframe animation files to name (NeoForge's `RegisterJsonAnimationTypesEvent`): push each id, `<namespace>:<path>`, as a `JsonAnimationType::Target` or `JsonAnimationType::Interpolation`. |
| `register_layer_definitions` | client | collect | Entity model layers built in code (NeoForge's `EntityRenderersEvent.RegisterLayerDefinitions`): push each one's id, `<mod id>:<model>#<layer>` as a `ModelLayerLocation` names it (`mymod:golem#main`), and its `LayerDefinition`, as the mod's `createBodyLayer()` builds it. |
| `register_load_conditions` | server | collect | Register named conditions once at mod load, before server data admission. |
| `register_model_loaders` | client | collect | The model loaders a mod adds (NeoForge's `ModelEvent.RegisterLoaders`): push each loader id, `<mod id>:<path>`. |
| `register_particle_groups` | client | collect | The render groups a mod's particles draw in (NeoForge's `RegisterParticleGroupsEvent`): push a `ParticleGroup` per group, its id `<mod id>:<path>`, how it blends and the particle types in it. |
| `register_particle_providers` | client | listen | The code-defined particle providers of a mod's own particle types (NeoForge's `RegisterParticleProvidersEvent`): `register(name, provider)`, and every particle of the type `name` (`ParticleOptions::named("mymod:spark")`) is moved each tick by `ParticleProvider::tick`. |
| `register_preset_editors` | client | collect | Register client screens for world-preset keys once at startup. |
| `register_range_select_item_model_property` | client | collect | The numeric item model properties a mod adds (NeoForge's `RegisterRangeSelectItemModelPropertyEvent`): push each id, `<mod id>:<path>`. |
| `register_render_pipelines` | client | collect | Register the mod's own render pipelines, once at client startup (NeoForge's `RegisterRenderPipelinesEvent`; 1.21.1 mods used `RegisterShadersEvent`): push a `RenderPipeline` for each into `event.pipelines`. |
| `register_runtime_sprites` | client | collect | Collect static, mod-generated PNGs for the block and GUI atlases. |
| `register_select_item_model_property` | client | collect | The categorical item model properties a mod adds (NeoForge's `RegisterSelectItemModelPropertyEvent`): push each id, `<mod id>:<path>`. |
| `register_special_model_renderer` | client | listen | The special item model types a mod draws (NeoForge's `RegisterSpecialModelRendererEvent`): `register(type, renderer)`, and every `minecraft:special` item model whose `model` object has the type `type` (`mymod:skillet`) is drawn by `SpecialModelRenderer::extract`. |
| `register_sprite_sources` | client | collect | The atlas sprite source types a mod adds (NeoForge's `RegisterSpriteSourcesEvent`): push each type id, `<mod id>:<path>`. |
| `register_texture_atlases` | client | collect | Declare atlases beside the game's (`RegisterTextureAtlasesEvent`), each with the atlas definition that lists its sprites. |
| `register_villager_professions` | server | collect | Declare villager professions once per mod load, before world admission. |
| `tags_updated` | both | listen | Tags changed, and what depends on them should be rebuilt (NeoForge's `TagsUpdatedEvent`); `cause` says how. |

#### Combat, damage and experience

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `apply_effect_tick` | server | listen | `MobEffect.applyEffectTick`: the content mod's resolved `EffectKey` `effect` acts on `entity` in `dimension` (`minecraft:overworld`). |
| `armor_hurt` | server | cancel | Armor is about to take durability damage from a hit (NeoForge's `ArmorHurtEvent`, vanilla's `LivingEntity.doHurtEquipment`): once per hit, for the pieces that would wear, and first for the helmet alone when a falling block hits a player (`hurtHelmet`). |
| `arrow_loose` | server | cancel | NeoForge's `ArrowLooseEvent`: the player releases a bow `stack` with `has_ammo` and `charge` ticks held. |
| `arrow_nock` | server | cancel | A player is about to nock `bow` with `hand` (`BowItem.use`, NeoForge's `ArrowNockEvent`): `has_ammo` is whether `getProjectile` found ammunition or the player is in creative. |
| `attack_entity` | server | cancel | A player attacks the entity `entity` (`Entity.getId`) of type `kind` (`minecraft:item_frame`, `minecraft:cow`, `minecraft:player`) standing at `pos` in the level `dimension` (a `&str`): `ServerGamePacketListenerImpl.handleInteract`'s attack, once in reach, where `Player.attack` starts. |
| `block_screen_effect_draw` | client | listen | Draw a block overlay a mod said it draws itself (`custom_draw` in `extract_block_screen_effect`), before vanilla's overlay of that kind, once a frame each. |
| `client_attack_target` | client | cancel | Before a client's ordinary melee click attacks entities (not a spear jab), after the interaction-key cancellation hook. |
| `consume_effect` | server | listen | A consumable's effect of a mod's own kind: the item's `on_consume_effects` entry is of a type in a namespace of its own, and the loader kept it for this call. |
| `creeper_ignite` | server | listen | A player ignited a creeper with an item. |
| `creeper_power` | server | listen | Lightning charged a creeper. |
| `critical_hit` | server | listen | NeoForge's `CriticalHitEvent`: a player's melee hit on a living `target_kind` (`minecraft:zombie`, `minecraft:armor_stand`) that deals damage, where `Player.attack` decides whether it is critical. |
| `debug_renderer_draw` | client | listen | Draw one active debug renderer (`register_debug_renderers`), asked for each view of a world, after `submit_custom_geometry`. |
| `dragon_egg_form` | server | cancel | The Ender Dragon fight is about to place its first dragon egg at `pos`. |
| `dragon_fight_won` | server | replace | The Ender Dragon fight is won (`EnderDragonFight.setDragonKilled`), on every win, not only the first. |
| `effect_particle_modification` | server | listen | NeoForge's `EffectParticleModificationEvent`, vanilla's `LivingEntity.updateSynchronizedMobEffectParticles`: asked for each active effect `effect` of the living entity `entity` (a `LivingRef`, a mob or a player), the ones vanilla hides included, as the server is about to send their swirls in its synched data (`DATA_EFFECT_PARTICLES`, `DATA_EFFECT_AMBIENCE_ID`). |
| `enchant_effect` | server | listen | A `minecraft:post_attack` effect of a mod's own kind, on a mob a player's weapon hit: the entry says `enchanted` attacker and `affected` victim, its `requirements` passed, and its `effect` names a `type` in a namespace outside `minecraft:`. |
| `ender_entity_teleport` | server | cancel | An enderman or a shulker is about to teleport on its own (`EntityTeleportEvent.EnderEntity`). |
| `ender_pearl_teleport` | server | cancel | A server player's or mob's ender pearl is about to teleport its `owner` (`EntityTeleportEvent.EnderPearl`, `ThrownEnderpearl.onHit`). |
| `enderman_anger` | server | cancel | `player` stares at `enderman` (NeoForge's `EndermanAngerEvent`, at the end of `Enderman.isBeingStaredBy`): the look is within the enderman's cone, in line of sight, and no carved pumpkin hides the face. |
| `exp_bottle` | server | listen | An experience bottle broke and its experience was awarded. |
| `experience_orb_can_merge` | server | listen | Whether the experience orb `into` takes the orb `from` in (vanilla's `ExperienceOrb.canMerge`): the same value, and ids a multiple of 40 apart. |
| `experience_orb_merge` | server | cancel | The experience orb `into` (`into_value` points, `into_count` orbs) is about to take `from` (`from_value` points, `from_count` orbs) in, after `experience_orb_can_merge` said yes (`from` is 0 and `from_count` 1 for a new orb that is spawning). |
| `explosion_detonate` | server | collect | An explosion at `center` in `dimension` (`minecraft:overworld`) is about to break `blocks` and reach `entities`: `ServerExplosion.explode` with its `toBlow`, before any block or entity is hit. |
| `explosion_knockback` | server | collect | An explosion at `center` in `dimension`, about to break `blocks`, pushes `entity` (None for a player, whom `host.players().acting_player()` names): `ServerExplosion.hurtEntities`, after the knockback is calculated. |
| `explosion_prime` | server | listen | A server explosion is about to begin at `center`, with `radius` (Bukkit's `ExplosionPrimeEvent`). |
| `explosion_start` | server | cancel | A server explosion is about to happen (`ExplosionEvent.Start`, in `ServerLevel.explode` before `ServerExplosion.explode`): every one, TNT, creepers, beds and anchors, fireballs, withers, end crystals and wind bursts. |
| `extract_block_screen_effect` | client | cancel | Before one of the first-person view's block overlays (`ScreenEffectRenderer`) is drawn, once a frame each is shown: `Block` with the view-blocking `state` at `pos` the camera is inside, `Water` with the `state` at the eye's block `pos` while the eye is under water, and `Fire` with fire's default `state` at the player's block `pos` while they burn. |
| `firework_explode` | server | listen | A firework rocket completed its explosion effects. |
| `gui_render_post` | client | listen | After the complete normal GUI batch, including screens, prompts, toasts, debug overlays, the pad cursor and IME. |
| `item_expire` | server | listen | A dropped item reaches its lifetime on the server (`ItemEntity.tick`, NeoForge's `ItemExpireEvent`). |
| `living_experience_drop` | server | cancel | A dying mob or player `entity` of `kind` is about to drop `original` points of experience; the vanilla conditions have been checked. |
| `mob_effect_added` | server | listen | NeoForge's `MobEffectEvent.Added`, vanilla's `MobEffect.onEffectStarted`: every `LivingEntity.addEffect` of the content mod's effect `effect`, new or not, changed or not, after `on_effect_added`. |
| `mob_effect_applicable` | server | replace | NeoForge's `MobEffectEvent.Applicable`, vanilla's `LivingEntity.canBeAffected`: asked wherever the game checks whether the resolved `EffectKey` `effect` (`minecraft:poison`) can land on the living entity `entity` (a `LivingRef`, the player's own included), before `addEffect` accepts it. |
| `mob_effect_expired` | server | cancel | NeoForge's `MobEffectEvent.Expired`, vanilla `LivingEntity.tickEffects`: the resolved `EffectKey` `effect` (`minecraft:speed`, amplifier `amplifier`) on the living entity `entity` (a `LivingRef`, the player's own included) counted down to nothing and is about to leave. |
| `mob_effect_remove` | server | cancel | NeoForge's `MobEffectEvent.Remove`, vanilla `LivingEntity.removeEffect`: the resolved `EffectKey` `effect` (`minecraft:speed`, amplifier `amplifier`) on the living entity `entity` (a `LivingRef`, the player's own included) is about to be removed by any path that walks the vanilla funnel: a command or milk for the player, a cure for a zombie villager, a mod's removal helper. |
| `on_effect_added` | server | listen | `MobEffect.onEffectAdded`: `LivingEntity.addEffect` gave `entity` (as in `native_apply_effect_tick`) the content mod's effect `effect`, which it didn't have. |
| `pickup_xp` | server | cancel | Before `player` picks up orb `orb` in `dimension`. |
| `player_attack_cooldown_reset` | server | listen | A server player reset their attack strength by swinging (`handlePunch`). |
| `player_brewed_potion` | server | listen | A server player took `stack` from a brewing stand bottle slot (`BrewingStandMenu.PotionSlot.onTake`): `player` is their UUID and `player_entity_id` their network id. |
| `player_change_beacon_effect` | server | listen | A server player changed a beacon's selected effects, after they were stored. |
| `player_exp_cooldown_change` | server | listen | A server player's XP orb pickup cooldown changed to `ticks`. |
| `player_pickup_arrow` | server | listen | A server player picked up an arrow after it was removed and its pickup was applied. |
| `player_ready_arrow` | server | listen | A server player released a drawn bow with `arrow` selected, by the use-item key or a mod's `LivingRef::release_using_item`. |
| `player_xp_change` | server | cancel | A server player's experience points are about to change through `Player.giveExperiencePoints` (NeoForge's `PlayerXpEvent.XpChange`): an orb pickup after mending, an advancement reward, or `/xp add ... points`. |
| `player_xp_level_change` | server | cancel | A server player's level is about to change through `Player.giveExperienceLevels`: once per level gained or lost by points, and for the anvil, `/experience add ... levels` and `PlayerRef::give_levels`. |
| `potion_brew_post` | server | listen | A brewing stand at `pos` brewed (`PotionBrewEvent.Post`, at the end of `BrewingStandBlockEntity.doBrew`), or `potion_brew_pre` was cancelled: `stacks` holds its five slots after the brew (or as the cancelling mods left them), and what the mods leave there is the stand's. |
| `potion_brew_pre` | server | cancel | A brewing stand at `pos` is about to brew (`PotionBrewEvent.Pre`, in `BrewingStandBlockEntity.doBrew`): `stacks` is a copy of its five slots (three bottles, the ingredient, the fuel). |
| `potion_splash` | server | listen | A thrown splash or lingering potion applied its hit effects. |
| `raid_spawn_wave` | server | listen | A raid wave has been spawned at `pos`; `wave` is its one-based wave number and `raiders` counts the raiders, not the mobs they ride. |
| `raid_state_change` | server | listen | A server raid at `pos` changed to `state` (`triggered`, `victory`, `loss` or `stopped`). |
| `render_arm` | client | cancel | Before this window's player's first-person `left` or right arm is drawn (`RenderArmEvent`, in `renderPlayerArm` and `renderMapHand`): the empty main hand's arm, and the arms under a held map, never while the player is invisible. |
| `render_fog` | client | listen | Final world fog, after the computed color hook, ordinary calculations and Horizon's fog option, before upload. |
| `render_frame_post` | client | listen | Once a frame, in a world or not, right after the frame was rendered (NeoForge's `RenderFrameEvent.Post`). |
| `render_frame_pre` | client | listen | Once a frame, in a world or not, after Horizon's extras and just before the frame is rendered (NeoForge's `RenderFrameEvent.Pre`). |
| `render_gui_post` | client | listen | The HUD (not drawn under F1), after the minimap (NeoForge's `RenderGuiEvent.Post`): place a box with `corners.place` (boxes stack away from their edge) and draw it with `native_fill` and `native_text`. |
| `render_gui_pre` | client | cancel | Before the in-world HUD is drawn (NeoForge's `RenderGuiEvent.Pre`): return `Flow::Cancel` and none of it is, as under F1: no vanilla layer (and so no `gui_layer_pre` or `gui_layer_post`), no `render_gui_post`, and no chat or toasts. |
| `render_hand` | client | cancel | Before one first-person hand of this window's player is drawn (`RenderHandEvent`, client): a hand that holds an item or a map, the empty main hand's arm, and the empty off hand, where vanilla draws nothing. |
| `render_item_in_frame` | client | cancel | Before a visible item frame's item or map is drawn. |
| `render_level_stage` | client | listen | Draw world geometry at one stage of a world view (NeoForge 26.3's `RenderLevelStageEvent`), asked for each `RenderLevelStage` in frame order. |
| `render_living_post` | client | listen | After a living entity is drawn (NeoForge's `RenderLivingEvent.Post`): once a frame for each mob and player in view whose `render_living_pre` wasn't canceled, after its model, its vanilla layers and the mods' layers. |
| `render_living_pre` | client | cancel | Before a living entity is drawn (NeoForge's `RenderLivingEvent.Pre`): once a frame for each mob and player in view (one the frame culls is not asked), before its pose, model and layers. |
| `render_name_tag` | client | listen | Name tag visibility and optional replacement text. |
| `render_needs` | client | collect | Asked as a frame is built and drawn: set what your mod needs in `RenderNeeds`, including entity runs and the world's depth without a shader pack. |
| `render_player_post` | client | listen | After a player is drawn (NeoForge's `RenderPlayerEvent.Post`): once a frame for each player whose `render_player_pre` wasn't canceled, after its `render_living_post`, or right after its `render_living_pre` when only that one was canceled. |
| `render_player_pre` | client | cancel | Before a player is drawn (NeoForge's `RenderPlayerEvent.Pre`): once a frame for each other player in view and for this window's player in third person (and F5 front view), before its `render_living_pre`. |
| `render_tooltip_gather_components` | client | cancel | `RenderTooltipEvent.GatherComponents`: the elements of `stack`'s tooltip (`ItemStack.getTooltipLines` as text lines, then the image component its item's `tooltip_component` part answered), once `item_tooltip` has run and before any wrapping or measuring. |
| `render_tooltip_pre` | client | cancel | `RenderTooltipEvent.Pre`: a tooltip, laid out and about to be placed and drawn. |
| `render_tooltip_texture` | client | listen | `RenderTooltipEvent.Texture`: the look of a tooltip box about to be drawn. |
| `screen_render_background` | client | listen | The open screen is about to draw its contents (`ScreenEvent.Render.Background`, after vanilla's `renderBackground` and before `render`): draw here for what must sit under the screen's widgets, items and tooltips. |
| `screen_render_foreground` | client | listen | The open screen has drawn its contents (`ScreenEvent.Render.Foreground`, vanilla's point before the deferred tooltips and the preedit overlay). |
| `screen_render_inventory_mob_effects` | client | cancel | Ahead of the effect panels beside the survival and creative inventories (`ScreenEvent.RenderInventoryMobEffects`, vanilla's `EffectsInInventory.extractRenderState`): `available_space` is the width in GUI pixels right of the container, at least 32 (with no effect to show, or less room, vanilla draws nothing and this does not fire). |
| `screen_render_post` | client | listen | The open screen has drawn itself and its tooltips (`Screen.extractRenderStateWithTooltipAndSubtitles`, the point of `ScreenEvent.Render.Post`): draw over it with `native_fill`, `native_sprite` and `native_text`, `screen.rect` being its container panel (or the whole window) and `screen.id` the screen it is. |
| `screen_render_pre` | client | cancel | Before the open screen draws itself (`ScreenEvent.Render.Pre`, vanilla's `Screen.renderWithTooltips`): draw to put something under it, or set `canceled` and the screen is not drawn, and neither is its `screen_render_background` or `screen_render_foreground`. |
| `should_apply_effect_tick_this_tick` | server | listen | `MobEffect.shouldApplyEffectTickThisTick` for a content mod's effect `effect` (an `EffectKey` for `ruby:bleeding`), each tick a player or a mob has it, in `MobEffectInstance.tickServer` before it counts down: `tick_count` is the ticks left (the entity's tick count when infinite). |
| `sweep_attack` | both | cancel | NeoForge's `SweepAttackEvent`: a player's damaging melee hit on a `target` (`minecraft:zombie`, `minecraft:oak_boat`) is about to sweep or not, after `critical_hit`. |
| `target_hit` | server | listen | A projectile hit the target block at `pos`, which now has `strength` output power. |
| `tnt_primed` | server | cancel | TNT at `pos` is about to be lit (primed): a flint and steel or a fire charge on it, fire burning it, a redstone signal, an arrow, or a dispenser's flint and steel or TNT item. |
| `village_siege` | server | cancel | A village siege found a spawn position on the server. |
| `wind_burst` | server | collect | Before a wind burst interacts with blocks, `wind_burst(dimension, center, blocks)` receives the shuffled candidate positions. |
| `xp_orb_targeting` | server | listen | An experience orb looks for a player to follow (NeoForge's `XpOrbTargetingEvent`, `ExperienceOrb.followNearbyPlayer`), on a server. |

#### Players

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `calculate_player_turn` | client | listen | The mouse sensitivity and cinematic camera option a grabbed player's mouse turn reads this frame (`MouseHandler.turnPlayer`). |
| `can_player_sleep` | server | listen | A server player tries to sleep in the bed whose head is at `pos` (NeoForge's `CanPlayerSleepEvent`, vanilla's `ServerPlayer.startSleepInBed`). |
| `client_player_change_game_type` | client | listen | NeoForge's `ClientPlayerChangeGameTypeEvent`: observe the local player info game type change. |
| `client_player_clone` | client | listen | After the client replaces its player on respawn or dimension change (NeoForge's `ClientPlayerNetworkEvent.Clone`). |
| `client_player_logging_in` | client | listen | The client accepted a play login and initialized its level and player identity (NeoForge's `ClientPlayerNetworkEvent.LoggingIn`). |
| `client_player_logging_out` | client | listen | Before client disconnect teardown (NeoForge's `ClientPlayerNetworkEvent.LoggingOut`). |
| `player_advancement_done` | server | listen | A server player completed an advancement, after progress was recorded. |
| `player_advancement_progress` | server | listen | A criterion of a server player's advancement was granted or revoked, after its progress changed and before any completion rewards. |
| `player_arm_swing` | server | listen | A guest's arm swing was handled by the server. |
| `player_bed_enter` | server | cancel | A server player is about to enter a bed after the sleep checks pass. |
| `player_bed_leave` | server | listen | `PlayerWakeUpEvent`: a server player is getting out of the bed at `pos` (`Player.stopSleepInBed`), heard first, while they still read as sleeping there. |
| `player_break_speed` | both | cancel | A player's destroy speed for the block at `pos` may be changed or canceled. |
| `player_bucket_entity` | server | listen | A server player caught a mob in a bucket, after it was removed and the filled bucket was given. |
| `player_bundle_item_select` | server | listen | A server player selected a bundle item in `slot` at `index`. |
| `player_changed_dimension` | server | listen | `player` (`name`) changed dimension (NeoForge's `PlayerChangedDimensionEvent`): `from` and `to` are dimension ids (`minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`). |
| `player_client_loaded_world` | server | listen | A server player finished loading the world around them after joining, respawning or changing dimension. |
| `player_clone` | server | listen | A player's data was copied to their new player (NeoForge's `PlayerEvent.Clone`, from `ServerPlayer.restoreFrom`): `was_death` is true after a death, false when they left the End after the credits (`respawn` with `keepAllPlayerData`). |
| `player_command` | server | cancel | A command a player typed (NeoForge's `CommandEvent`, before `Commands.performCommand` runs it), without its slash: any command, vanilla's and the mods' own, also one the player may not run. |
| `player_commands_sent` | server | collect | The command tree a server sends a player (NeoForge's and Paper's "commands sent to a player" stage): when they join and when their level changes (`/op`, `/deop`, the LAN settings). |
| `player_container_close` | server | listen | `player` (`name`) closed the menu `player_container_open` told of (NeoForge's `PlayerContainerEvent.Close`, posted in `ServerPlayer.doCloseContainer` after the menu's `removed`): they closed it, walked out of reach, died, changed level, or another menu opened. |
| `player_container_open` | server | listen | `player` (`name`) opened a menu (NeoForge's `PlayerContainerEvent.Open`, posted when `ServerPlayer.openMenu` sets the new `containerMenu`): any menu, vanilla or a mod's. |
| `player_deep_sleep` | server | listen | A server player has slept for 100 ticks in a bed and now counts as deeply asleep for the night-skip check. |
| `player_destroy_item` | server | listen | The item `item` of the server player `player` (`name`) broke in `slot` (`mainhand`, `offhand`, `head`, `chest`, `legs`, `feet`, `body` or `saddle`): `LivingEntity.onEquippedItemBroken`, Bukkit's `PlayerItemBreakEvent`. |
| `player_destroyed_block` | server | listen | A player's break removed the block `state` from `pos`: the `Block.destroy(level, pos, state)` call `ServerPlayerGameMode.destroyBlock` makes only after `Level.removeBlock` succeeded, before a creative player's return and `playerDestroy`'s drops. |
| `player_died` | server | listen | `player` died: where `ServerPlayer.die` starts, with the death message's key when the game knows it (`death.attack.lava`; none for a message it builds from the combat). |
| `player_edit_book` | server | cancel | A server player's book edit or signing packet is about to be applied. |
| `player_egg_throw` | server | listen | A server player threw an egg, after its projectile was launched and the use was applied. |
| `player_elytra_boost` | server | listen | A server player used a firework rocket to boost while gliding, after the rocket was launched and consumed. |
| `player_enchant_item` | server | listen | A server player enchanted `item` at an enchanting table (`EnchantmentMenu.clickMenuButton`): `enchantments` are the ones it was given, in order, as (id, level). |
| `player_fish` | server | cancel | A server player's fishing rod, with `state`: 0 the cast (`FishingRodItem.use` throwing the bobber), 1 a fish biting (`FishingHook.catchingFish`), 2 the reel-in of a bite, which gives the catch, 3 the reel-in with nothing on the line, 4 the reel-in of a bobber hooked to an entity or item (like Bukkit's `PlayerFishEvent`; NeoForge's `ItemFishedEvent` is later, in `FishingHook.retrieve`). |
| `player_flower_pot_manipulate` | server | listen | A server player put a plant into a flower pot, after the potted block state was stored and the item consumed. |
| `player_flyable_fall` | server | listen | NeoForge's `PlayerFlyableFallEvent`: server player `player` (`name`) landed after falling `distance` blocks at damage multiplier `multiplier` while allowed to fly (a creative or spectator player walking), so `Player.causeFallDamage` returned early and nothing was hurt. |
| `player_game_mode_changed` | server | cancel | A server player's game mode is about to change (NeoForge's `PlayerChangeGameModeEvent`): from `from` to `to`, both the shared `GameMode` used by commands and `PlayerRef`. |
| `player_handshake` | server | cancel | The handshake of a connection that is about to log in or be transferred in (Paper's `PlayerHandshakeEvent`), before the login starts and before the protocol version, the proxy's forwarding, the name, the whitelist or the bans are looked at. |
| `player_harvest_check` | both | collect | A player's ability to harvest the block at `pos` may be changed. |
| `player_heart_type` | client | listen | Which hearts the local player's health bar draws this frame (NeoForge's `PlayerHeartTypeEvent`, from `Gui.HeartType.forPlayer`): `kind` comes in as vanilla's pick (poisoned, else withered, else frozen when fully frozen, else normal); set it to draw another kind. |
| `player_input` | server | listen | A guest sent the server's current `PlayerInput` flags, after they were applied to the hub. |
| `player_insert_lectern_book` | server | listen | A server player inserted a book into a lectern, after the block entity and inventory changed. |
| `player_inventory_changed` | server | listen | `player_inventory_changed(player, stacks)`: a server player's inventory changed, at the moment vanilla's `InventoryChangeTrigger` is checked (the per-tick check found another item or count in a slot), heard before any criterion and whether or not the tree has `inventory_changed` advancements. |
| `player_item_cooldown` | server | cancel | An item cooldown is about to start for a server player. |
| `player_item_frame_change` | server | listen | A server player changed an item frame after its contents or rotation changed. |
| `player_item_mend` | server | listen | Mending repaired a server player's item with experience from an orb. |
| `player_item_smithed` | server | listen | `PlayerEvent.ItemSmithedEvent`: a server player took `result` from a smithing table (`SmithingMenu.onTake`, or `onQuickCraft` for a shift-click): `player` is their UUID and `player_entity_id` their network id, `inputs` the template, base and addition as they were before the take used them (`None` for an empty slot). |
| `player_jump` | server | listen | A guest's accepted movement made the server jump them from the ground. |
| `player_kicked` | server | listen | A network or fake guest was kicked, after the hub recorded the rendered `reason`. |
| `player_launch_projectile` | server | listen | A server player launched a projectile item, after it was created. |
| `player_lectern_page_change` | server | listen | A server player changed the page of a lectern, after its page and redstone pulse changed. |
| `player_level_change` | server | listen | A server player's experience level changed, after the new level was applied. |
| `player_load_from_file` | server | listen | A player was loaded from the world's save (NeoForge's `PlayerEvent.LoadFromFile`, `PlayerList.loadPlayerData`): their `.dat` has been read and applied, and they are not placed in a level yet. |
| `player_logged_in` | server | listen | `player` (`name`) joined the game (NeoForge's `PlayerLoggedInEvent`): placed in the world and listed in `host.players().server_players()`, so `PlayerRef::send_payload` reaches them. |
| `player_logged_out` | server | listen | `player` (`name`) left the game (NeoForge's `PlayerLoggedOutEvent`), by any path: a lost connection, a kick, the server stopping or failing. |
| `player_login` | server | cancel | A player whose login the server has accepted (NeoForge's `PlayerNegotiationEvent` stage): the name, whitelist, ban and server-full checks have passed and the player holds a place, but their saved state is not loaded and they are not placed in a level yet. |
| `player_login_profile` | server | listen | A player's game profile at login, before the server sends the login success: the UUID and name the login authenticated and the properties it would send. |
| `player_loom_pattern_select` | server | listen | A server player selected a loom pattern, after the loom menu accepted the button. |
| `player_map_filled` | server | listen | A server player filled an empty map, after the filled map and its saved data were created. |
| `player_move` | server | cancel | A player's movement report is about to be applied: they would move from `from` to `to`, or turn from `from_rotation` to `to_rotation` (`[yaw, pitch]`). |
| `player_name_entity` | server | listen | A server player named a mob with a name tag, after its name and persistence changed and the tag was consumed. |
| `player_name_format` | both | collect | A player's display name was formatted. |
| `player_open_sign` | server | listen | A server player opened a sign for editing, after the editor state was made. |
| `player_passage_cross` | server | cancel | `player`'s client crossed `passage` (arismoko/horizon#3775), one of the player's level's, from `from` (its feet before), and the server is about to follow: the move to `from` passed the move checks and both ends are tracked for the player, and the server has not yet worked out where the player comes out. |
| `player_pick_item` | server | cancel | A server player picked an item from a block or entity, before it enters the hotbar. |
| `player_pong` | server | listen | A guest's client answered a ping (`ServerboundPongPacket`, vanilla's `ServerGamePacketListenerImpl.handlePong`): `id` is the number the ping carried. |
| `player_recipe_book_click` | server | listen | A server player's recipe book requested a known recipe be placed in their crafting grid. |
| `player_recipe_book_settings_change` | server | listen | A server player's authoritative recipe book pair changed. |
| `player_recipe_discovered` | server | cancel | A recipe is about to be unlocked for a server player. |
| `player_respawn_position` | server | listen | `PlayerRespawnPositionEvent`: where `player` respawns, once the server has found it: the bed or anchor they stand up at (facing it), or the world spawn's free column. |
| `player_respawned` | server | listen | `player` (`name`) respawned (NeoForge's `PlayerRespawnEvent`): at the end of `PlayerList.respawn`, once they are placed and their health, food and experience reset, with their state swapped in. |
| `player_riptide` | server | listen | A server player launched with a riptide trident, after the velocity, spin attack and grounded lift have been applied. |
| `player_save_to_file` | server | listen | A player was saved to the world's store (NeoForge's `PlayerEvent.SaveToFile`, `PlayerList.save`): when they leave or the world closes, when a dedicated or integrated server saves its guests, and for an integrated server's owner when the world saves them. |
| `player_set_spawn` | server | cancel | NeoForge's `PlayerSetSpawnEvent`: a server player's respawn point is about to be set (`ServerPlayer.setRespawnPosition`), once for each call vanilla makes: every sleep in a bed (the same bed too, which refreshes its angle), an anchor that is a new point, `/spawnpoint`, and the copy to the new player when a player respawns and their respawn block was found (`pos` is none for a player with no point). |
| `player_shear_entity` | server | cancel | A player shearing a mob is about to do so. |
| `player_shield_disable` | server | listen | A server player's shield was disabled by a blocked melee hit. |
| `player_spawn_phantoms` | server | replace | `PlayerSpawnPhantomsEvent`: the phantom spawner is about to decide whether to try a spawn near a non-spectator server player, before its difficulty and rest rolls. |
| `player_sprint` | server | listen | A server player's sprint started or stopped: their client sent `START_SPRINTING` or `STOP_SPRINTING` (the sprint key, a double tap forward, or letting go) and the server's player is now `sprinting` (vanilla's `setSprinting` in `handlePlayerCommand`). |
| `player_start_spectating_entity` | server | listen | A server player started spectating `entity_id`, after their camera moved to it (`ServerPlayer.setCamera` with a new target: a spectator's click or `/spectate`) and their client was told. |
| `player_start_tracking` | server | listen | A client started tracking an entity (NeoForge's `PlayerEvent.StartTracking`, posted at the end of `ServerEntity.addPairing`): `player` (`name`) is now sent `entity` (its ID, `uuid` its UUID, `kind` its type, `minecraft:zombie`) and its changes. |
| `player_stat_award` | server | cancel | A statistic is about to be awarded to a server player, before its count and scoreboard criteria change. |
| `player_stays_on_ground_surface` | both | listen | The player's crouch edge backoff (`Player.maybeBackOffFromEdge`) asks whether the player is staying on the ground surface (`Player.isStayingOnGroundSurface`, vanilla: the shift key is down). |
| `player_stonecutter_recipe_select` | server | listen | A server player selected a stonecutter recipe, after the menu accepted its button. |
| `player_stop_spectating_entity` | server | listen | A server player stopped spectating `entity_id`, after their camera returned to the player. |
| `player_stop_tracking` | server | listen | A client stopped tracking an entity (NeoForge's `PlayerEvent.StopTracking`, posted at the end of `ServerEntity.removePairing`): it went out of range, left the level or died, or `player` (`name`) left, changed level or respawned, which stops everything they tracked. |
| `player_stop_using_item` | server | listen | A server player released the use-item key, or a mod's `LivingRef::release_using_item` did, before the item finished being used. |
| `player_swap_hands` | server | cancel | Heard before a non-spectator player's swap-hands action. |
| `player_swap_with_equipment_slot` | server | listen | A server player swapped their main and off hands, after both inventory slots changed. |
| `player_switch_hotbar_slot_post` | server | listen | `PlayerSwitchHotbarSlotEvent.Post`: a server player switched from hotbar slot `old_slot` to `new_slot` (0 to 8), after the selected slot changed and a main-hand item use stopped. |
| `player_switch_hotbar_slot_pre` | server | cancel | `PlayerSwitchHotbarSlotEvent.Pre`: a server player is switching from hotbar slot `old_slot` to `new_slot` (0 to 8), after the slot range check and before the selected slot changes. |
| `player_tab_list_name_format` | server | collect | ServerPlayer.getTabListDisplayName: the name the Tab list shows for `player`, None to let the client choose it. |
| `player_take_lectern_book` | server | listen | A server player took the book from a lectern, after the menu and block entity changed. |
| `player_teleport_end_gateway` | server | cancel | A server player is about to travel through an End gateway at `gateway`. |
| `player_teleported` | server | listen | A server player a command teleported (NeoForge's `EntityTeleportEvent`, for a player): `/tp` and `/teleport`, inside their level or into another one. |
| `player_tick_post` | both | listen | After a player's tick on both logical sides (NeoForge 26.3 `PlayerTickEvent.Post`). |
| `player_tick_pre` | both | listen | Before a player's tick on both logical sides (NeoForge 26.3 `PlayerTickEvent.Pre`). |
| `player_toggle_entity_age_lock` | server | listen | A server player toggled an ageable mob's age lock with a golden dandelion, after the lock and item use changed. |
| `player_toggle_flight` | server | cancel | A player's client started (`flying`) or stopped flying (vanilla's `handlePlayerAbilities`, which honours it only for a player who may fly: creative or spectator), heard once for each change, on a server, with the player's state swapped in, so `PlayerRef::tell` and the other helpers reach them. |
| `player_toggle_sneak` | server | listen | A guest's client changed sneaking in its player input packet, after the hub applied the new state. |
| `player_totem` | server | listen | A totem of undying saved the server player `player` (`name`): their hurt would have killed them and the totem held in either hand fired (`LivingEntity.checkTotemDeathProtection`; Bukkit's `EntityResurrectEvent`), heard just before it is used up and its effects apply. |
| `player_trade` | server | listen | A server player completed a villager or wandering trader trade. |
| `player_unleash_entity` | server | listen | A server player removed an entity's leash, after it was cleared. |
| `player_velocity` | server | listen | A server applied `velocity` to a player, when syncing changed motion to that player's client. |
| `spread_players_teleport` | server | cancel | A server entity is about to be placed by `/spreadplayers` (NeoForge's `EntityTeleportEvent.SpreadPlayersCommand`, vanilla's `SpreadPlayersCommand.setPlayerPositions`): once per entity, after the spread worked out its spot. |

#### Entities and mobs

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `animal_tame` | server | cancel | The player `tamer` (UUID) and `tamer_entity_id` is about to tame the mob `entity` of `kind` (`AnimalTameEvent`): its taming roll succeeded (a wolf's bone, a cat's fish, a parrot's seeds or a nautilus's food; a horse, donkey, mule or llama the player rides). |
| `baby_entity_spawn` | server | cancel | Two animals bred and a baby is about to be made (NeoForge's `BabyEntitySpawnEvent`, vanilla's `Animal.spawnChildFromBreeding`): `parent` and `partner` are entity ids, `player` and `player_entity_id` the UUID and entity id of the player who fed them into love (`getCausedByPlayer`, None when none), `kind` the baby's type (a horse and a donkey have a mule). |
| `block_entity_type_add_blocks` | both | collect | Extend existing types' valid blocks (NeoForge `BlockEntityTypeAddBlocksEvent`). |
| `block_shear_entity` | server | listen | A dispenser sheared `entity` of kind `kind` at the block in front of its own block at `pos`, after the shearing effects. |
| `custom_spawner_tick` | server | listen | `CustomSpawner.tick` for a spawner a mod added with `modify_custom_spawners`: the level `dimension` ticks the spawner `key`, once per tick in the list's order, with `spawn_enemies` as `ServerLevel.isSpawningMonsters` (the `spawn_monsters` rule, and not peaceful). |
| `enchanted_entity_loot` | server | listen | An entity's loot table reads an enchantment's level (NeoForge's `EnchantedEntityLootEvent`): asked in the middle of the roll, each time an `enchanted_count_increase` function or a `random_chance_with_enchanted_bonus` condition is evaluated, for a death and for `/loot ... kill`. |
| `entity_break_door` | server | listen | A mob broke the door at `pos`. |
| `entity_breed` | server | listen | Two mobs completed a breeding action. |
| `entity_change_block` | server | listen | A mob changed a block at `pos`, after the new state was applied. |
| `entity_change_block_pre` | server | cancel | A mob or a player is about to change the block at `pos`: a landing's trampling (farmland to dirt, a turtle egg breaking), before its neighbour updates and events. |
| `entity_constructing` | both | listen | A new entity is being made (NeoForge's `EntityEvent.EntityConstructing`, vanilla's `Entity.<init>`): `kind` is its type (`minecraft:zombie`, `minecraft:player` for a player) and `holder` holds its attachments. |
| `entity_drop_item` | server | listen | A mob is about to drop `count` of `item` at `pos` (Bukkit's `EntityDropItemEvent`). |
| `entity_enter_block` | server | listen | A bee entered the hive at `pos` and was stored there. |
| `entity_enter_love_mode` | server | listen | A mob entered love mode after player interaction. |
| `entity_entering_section` | both | listen | `entity` moved from the 16x16x16 section `old_section` into `new_section` (NeoForge's `EntityEvent.EnteringSection`, vanilla's `EntitySectionStorage` callbacks on `Entity.setPosRaw`). |
| `entity_interact` | both | replace | A player right-clicks, in the level `dimension` (a `&str`), the entity `entity` (`Entity.getId`) of type `kind` (`minecraft:cow`) with `hand` (`Hand::Main` or `Hand::Off`), holding `stack` (none in an empty hand): where `Player.interactOn` starts, before the entity's own `interact` and the item's `interactLivingEntity`. |
| `entity_interact_specific` | server | cancel | a right-click on an entity with the point clicked: `entity_interact_specific` hears the entity, its type, the held stack and the point clicked, measured from the entity's position, before `native_interact_entity` (NeoForge's `PlayerInteractEvent.EntityInteractSpecific` counterpart); a claim swings the hand and ends the click. |
| `entity_invulnerability_check` | both | listen | Decide whether an entity is invulnerable to this specific damage source (`Entity.isInvulnerableTo`). |
| `entity_join_level` | server | cancel | An entity joins the server level `dimension` (NeoForge's `EntityJoinLevelEvent`): vanilla's `PersistentEntitySectionManager.addEntity`. |
| `entity_leave_level` | server | listen | An entity leaves the server level `dimension` (NeoForge's `EntityLeaveLevelEvent`): vanilla's `onTrackingEnd`, when it is removed for any reason (picked up, merged, despawned, burnt, blown up, killed, taken by a hopper or a mob, gone through a portal) or its chunk unloads, a level closing with nobody in it included. |
| `entity_mob_griefing` | server | listen | May `entity` change the world? |
| `entity_mount` | server | cancel | An entity mounts or dismounts a vehicle (NeoForge's `EntityMountEvent`, posted from `Entity.startRiding` and `removeVehicle`): `entity` of type `kind` (`minecraft:player`, `minecraft:zombie`) in the server level `dimension`, getting on (`mounting`) or off the `vehicle` of type `vehicle_kind` (`minecraft:oak_boat`, `minecraft:minecart`, `minecraft:horse`, `minecraft:cushion`). |
| `entity_move` | server | cancel | One physical `Entity.move` invocation (arismoko/horizon#2414, design arismoko/horizon#2308, the maintainer ruling of the same issue): the entity `entity` of `kind` (`minecraft:zombie`) is about to move by `delta` (blocks this tick) for `cause`'s sake - itself, the player behind a position packet, a piston, a shulker box lid, a shulker mob's opening lid. |
| `entity_outline` | client | listen | Whether an entity the client draws has the glowing outline, and in which colour (`Entity.isCurrentlyGlowing` and `getTeamColor`, what Fabric's and NeoForge's glowing and team colour overrides change). |
| `entity_place` | server | cancel | `BlockEvent.EntityPlaceEvent` (and `EntityMultiPlaceEvent`, which extends it): `placer` set the block `state` at `pos` against `placed_against`, before it is written. |
| `entity_portal` | server | listen | A server entity finished its portal wait and is beginning to leave `dimension` through `pos`. |
| `entity_pose_change` | server | listen | A guest player's synchronized pose changed from `from` to `to`. |
| `entity_shoot_bow` | server | listen | A player bow or crossbow launched an arrow with `force`, after the projectile entered the level. |
| `entity_size` | both | collect | An entity's size was recalculated (NeoForge's `EntityEvent.Size`, vanilla's `Entity.refreshDimensions`): its pose changed to `pose`, it grew up, its `minecraft:scale` changed, or it is new. |
| `entity_struck_by_lightning` | server | cancel | A lightning bolt is about to hit `entity` (NeoForge's `EntityStruckByLightningEvent`, before `Entity.thunderHit`): once per bolt strike for each entity whose box meets the strike box (3 blocks around, 9 up), players included. |
| `entity_tame` | server | listen | A mob became tamed after its owner and persistent state were set. |
| `entity_teleport` | server | listen | A server entity was moved by a same-level teleport command, after its position changed. |
| `entity_tick_listened` | server | collect | Whether a mod listens to `entity_tick_pre` and `entity_tick_post`: a mod that hooks them hooks this one too and sets `listened` (HEAD, `*listened = true`). |
| `entity_tick_post` | server | listen | The entity `entity` of type `kind` ticked in the server level `dimension` (NeoForge's `EntityTickEvent.Post`): for each entity `entity_tick_pre` let through, after it ticked, also one that died or was removed in its tick. |
| `entity_tick_pre` | server | cancel | An entity of type `kind` (`minecraft:zombie`, `minecraft:item`) with ID `entity` is about to tick in the server level `dimension` (NeoForge's `EntityTickEvent.Pre`, posted around `Entity.commonTick` and `tick` in `ServerLevel.tickNonPassenger`). |
| `entity_toggle_glide` | server | listen | A guest's fall flying state changed on the server. |
| `entity_travel_to_dimension` | server | cancel | `entity` is about to travel to the level `dimension` (NeoForge's `EntityTravelToDimensionEvent`), before anything of it moves: the entity is still where it is, in its own level. |
| `entity_unleash` | server | listen | A leashed mob's leash was removed, before any lead drop is handled (Bukkit's `EntityUnleashEvent`). |
| `finalize_spawn` | server | cancel | NeoForge's `FinalizeSpawnEvent`: the new mob of type `kind` is at `pos` and `Mob.finalizeSpawn` (equipment, attributes, variants, the class's own setup) is about to run for `reason`. |
| `hears_projectile_impacts` | server | collect | Whether a mod hears projectiles' impacts (`native_projectile_impact`): set `hears`. |
| `item_despawn` | server | listen | A dropped item entity reached its 6000-tick lifetime and is about to be removed (Bukkit's `ItemDespawnEvent`). |
| `item_spawn` | server | listen | A dropped item entity was successfully added to the server level. |
| `living_breathe` | server | listen | An entity's breathing this tick (`LivingBreatheEvent`, posted from `LivingEntity.baseTick` every tick for a living entity with the generic air logic): `entity` is its ID and `kind` its type (`minecraft:player`, `minecraft:zombie`). |
| `living_change_target` | server | cancel | A mob is about to change its target. |
| `living_conversion_post` | server | listen | A mob converted into another type (NeoForge's `LivingConversionEvent.Post`): `converted` replaces `entity`. |
| `living_conversion_pre` | server | cancel | A mob is about to convert into another type (NeoForge's `LivingConversionEvent.Pre`, vanilla's `Mob.convertTo`): a zombie drowning, a piglin or hoglin in the Overworld, a tadpole growing, a mooshroom sheared by a dispenser, a pig or villager struck by lightning. |
| `living_damage_post` | server | listen | NeoForge's `LivingDamageEvent.Post`: entity `entity` of type `kind` has been hurt by the hit `container` describes, with its final values (health damage, the reductions, the post-attack invulnerability ticks). |
| `living_damage_pre` | server | listen | NeoForge's `LivingDamageEvent.Pre`: `LivingEntity.actuallyHurt` of entity `entity` of type `kind` (`minecraft:player`, `minecraft:zombie`) by `source` (named as `native_living_incoming_damage` names it) has taken armor and magic, and the absorption hearts are next: `container` holds the damage it started with, what it is now (`new_damage`, which may be 0) and each reduction so far (armor, enchantments, mob effects). |
| `living_death` | server | cancel | A mob or server player is about to die (NeoForge's `LivingDeathEvent`, vanilla's `LivingEntity.die`): its health has run out, and the loot, experience and death message have not run. |
| `living_death_message` | server | collect | `living_death_message` is asked for the death message a tamed mob sends its owner (`TamableAnimal.die`), once the message is built and only when it would be sent (owner online, `show_death_messages` on). |
| `living_death_post` | server | listen | A mob or server player's death has finished (Fabric's `ServerLivingEntityEvents.AFTER_DEATH`, vanilla's `LivingEntity.die` at its `broadcastEntityEvent`): the loot, equipment and experience dropped and the death message went out. |
| `living_destroy_block` | server | cancel | A mob is about to destroy a block (NeoForge's `LivingDestroyBlockEvent`): a wither breaking blocks around it, a ravager breaking leaves, the ender dragon's wall check, an enderman picking a block up, a silverfish waking its friends (the infested blocks break), a zombie breaking a door or a turtle egg. |
| `living_drops` | server | cancel | Not `loot_drops`: that is the loot-modifier step, which edits the stacks a table rolled (blocks, shearing, a mob's table). |
| `living_drown` | server | cancel | An entity that can't breathe and has no air (`LivingDrownEvent`, posted from `LivingEntity.baseTick`), with vanilla's answers as the defaults: it drowns at -20 air, for 2 damage and 8 bubbles. |
| `living_entity_use_item_finish` | both | listen | NeoForge's `LivingEntityUseItemEvent.Finish`: `entity`'s use of `stack` in `hand` ran its time (`LivingEntity.completeUsingItem`), `duration` the ticks left, always 0. |
| `living_entity_use_item_start` | both | cancel | NeoForge's `LivingEntityUseItemEvent.Start`: `entity` starts using `stack` in `hand` (`LivingEntity.startUsingItem`), for `duration` ticks (`use_duration`'s answer). |
| `living_entity_use_item_stop` | both | cancel | NeoForge's `LivingEntityUseItemEvent.Stop`: `entity` lets go of `stack` in `hand` before its use ran its time (`LivingEntity.releaseUsingItem`: the use button let go, a screen opened), `duration` the ticks left. |
| `living_entity_use_item_tick` | both | cancel | NeoForge's `LivingEntityUseItemEvent.Tick`: each tick `entity` uses `stack` in `hand`, where `LivingEntity.updatingUsingItem` starts, `duration` the ticks left. |
| `living_equipment_change` | server | listen | A mob's, an armor stand's or a player's equipment slot took a different stack (NeoForge's `LivingEquipmentChangeEvent`, posted in `LivingEntity.collectEquipmentChanges`): `entity` is its ID and `kind` its type (`minecraft:zombie`, `minecraft:player`), `slot` the `EquipmentSlot` (`mainhand`, `offhand`, `feet`, `legs`, `chest`, `head`, `body`, `saddle`), `from` what the slot held when this was last heard (none for empty) and `to` what it holds now. |
| `living_equipment_listened` | server | collect | No longer needed: the game reads every mob's and connected player's equipment each tick, and tells `living_equipment_change`, whenever an enabled mod subscribes to that hook. |
| `living_fall` | both | cancel | A living entity is about to handle a fall (`LivingFallEvent`). |
| `living_flying_speed` | both | listen | A living entity's airborne acceleration (`LivingEntity.getFlyingSpeed`) is about to be used by its travel. |
| `living_get_projectile` | both | collect | `LivingEntity.getProjectile(weapon)` (NeoForge's `LivingGetProjectileEvent`) for a player (`player` and `name`) or a mob (`entity`, `kind`): a skeleton or illusioner's shot, or the crossbow load of a pillager or piglin. |
| `living_heal` | both | cancel | A living entity is about to heal. |
| `living_incoming_damage` | server | cancel | NeoForge's `LivingIncomingDamageEvent`: `LivingEntity.hurtServer`, past its guards (an invulnerable or dead entity, fire on a fire-resistant one), is about to hurt entity `entity` of type `kind` (`minecraft:zombie`, `minecraft:player`) by `source`: its damage type (`minecraft:fall`), causing entity (`DamageSource.getEntity`: the shooter of an arrow), direct entity (`getDirectEntity`: the projectile's entity id when the damage type names one, on a player, mob or ender dragon; 0 for `magic()` and any other source with no direct entity, even during that projectile's impact), and where it comes from (`getSourcePosition`). |
| `living_jump` | both | listen | A living entity jumped (`LivingEvent.LivingJumpEvent`). |
| `living_jump_power` | both | listen | A living entity's jump power (`LivingEntity.getJumpPower`: jump strength attribute, block jump factor and Jump Boost) is about to be used by `jumpFromGround`. |
| `living_knockback` | server | cancel | NeoForge's `LivingKnockBackEvent`: `LivingEntity.knockback` is about to push entity `entity` of type `kind` (`minecraft:zombie`, `minecraft:player`) `strength` away from the direction (`x`, `z`), on the raw strength, before the entity's knockback resistance takes its share and its motion changes. |
| `living_shield_block` | server | listen | NeoForge's `LivingShieldBlockEvent`: entity `entity` of type `kind` (`minecraft:player`, `minecraft:zombie`) is hit for `damage` from `source` (its type and entities, as `native_living_incoming_damage` names them), and `LivingEntity.applyItemBlocking` is about to resolve what a raised `blocks_attacks` item stops. |
| `living_speed` | both | listen | A living entity's movement speed (`LivingEntity.getSpeed`) is about to drive its travel on the ground and through fluids. |
| `living_use_totem` | server | cancel | NeoForge's `LivingUseTotemEvent`: a killing blow on entity `entity` of type `kind` from `source` (named as `native_living_incoming_damage` names it) would be taken by the `death_protection` item `item` in its `hand` (main or off; `LivingEntity.checkTotemDeathProtection`). |
| `living_visibility` | server | listen | How visible `entity` is to `looking` (NeoForge's `LivingVisibilityEvent`, last in `LivingEntity.getVisibilityPercent`): the range a mob's targeting check sees it from is scaled by `visibility`. |
| `mob_despawn` | server | listen | NeoForge's `MobDespawnEvent`: the mob `mob` (of type `kind`, at `pos`) is about to run `Mob.checkDespawn`, which fires once per tick for every mob that is ticked or only checked for despawning, persistent ones and peaceful-mode ones included. |
| `mob_killed` | server | listen | `player`'s melee blow killed the mob `entity` of `kind` (other deaths aren't told yet). |
| `mob_spawn_placement_check` | server | listen | NeoForge's `MobSpawnEvent.SpawnPlacementCheck`: may the entity type `kind` spawn at `pos` in `level` for `reason`? |
| `mob_spawn_position_check` | server | listen | NeoForge's `MobSpawnEvent.PositionCheck`: may the freshly created mob of type `kind` stand at `pos` (its position, a mob's centre) in `level` for `reason`? |
| `mob_spawned` | server | listen | A new mob of `kind` joined `dimension` at `pos`: `natural` when the world's spawning made it, false when something else did (a breed, a raid, a spawner, a conversion, `/summon`, and the second mob of a jockey the world spawned). |
| `mob_split` | server | cancel | A slime, magma cube or sulfur cube is dying and about to split into smaller ones (NeoForge's `MobSplitEvent`, vanilla's `AbstractCubeMob.remove`): `parent` is its entity id, `kind` its type, `size` its size, `child_size` the size of the children (half of it) and `count` how many vanilla made (2 to 4 for a slime or magma cube, 2 for a sulfur cube; a primed one makes none and isn't asked). |
| `play_level_sound_at_entity` | both | cancel | A sound bound to `entity` is about to play (NeoForge's `PlayLevelSoundEvent.AtEntity`, in `Level.playSeededSound`'s entity overload, `playSound(except, sourceEntity, ...)`): a goat horn, a frog's tongue, eating and spawn, an animal eating (frog, goat, sniffer), a long jump, a goat's ram, a breeze's charge and deflection, shearing (sheep, mooshroom, snow golem, bogged, copper golem), dye on a sheep, an axe on a copper golem, an allay's item and duplication, a ghastling hatching, a trident thrown (the trident) or riptide (the player), and a mod's sound bound to an entity. |
| `portal_spawn` | server | cancel | A fire placed at `pos` (`state`) found an empty obsidian frame, `shape`, and is about to fill it with a nether portal (`BlockEvent.PortalSpawnEvent`, in `BaseFireBlock.onPlace`), in the Overworld or the Nether. |
| `potential_spawns` | server | cancel | Possible natural spawns at a server position (NeoForge's `LevelEvent.PotentialSpawns`, `NaturalSpawner.mobsAt`). |
| `projectile_impact` | server | cancel | A projectile hit something (NeoForge's `ProjectileImpactEvent`, posted where vanilla's projectile has found what it meets, before the hit acts): `projectile` (its entity id) of type `kind` (`minecraft:arrow`), shot by `owner` (a mob's or a player's entity id, 0 for none) in the level `dimension`. |
| `projectile_launch` | server | listen | A server projectile was added to the level. |
| `spawn_change` | server | listen | The world's spawn point changed to `pos` and `angle`. |
| `trade_with_villager` | server | collect | A trade of a villager or a wandering trader was made (`MerchantResultSlot.onTake` once the offer is taken): `merchant` is the trader's entity id and `kind` its type (`minecraft:villager`, `minecraft:wandering_trader`), `offer` the live one in the trading session (not a copy) as the trade left it: `uses` counted, `result` what the player got. |

#### Blocks

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `alter_ground` | server | collect | A tree's `alter_ground` decorator (`AlterGroundDecorator.place`, the podzol under a big spruce) is about to turn the ground around `positions`: the lowest of its trunk or roots (`logs` if it has no roots, `logs` and `roots` when they start level, else `roots`); only those at the first one's Y are placed around. |
| `beacon_state_change` | server | listen | A beacon at `pos` activated or deactivated after its beam was checked. |
| `bell_ring` | server | cancel | On an authoritative server, a bell at `pos` is about to ring. |
| `block_break_progress_update` | server | listen | A server guest's block-breaking animation changed to `progress` at `pos` (0 through 9). |
| `block_burn` | server | cancel | On an authoritative server, fire is about to burn the block at `pos` away. |
| `block_changed` | server | listen | An authoritative server committed a changed state through `Level.setBlock`. |
| `block_cook` | server | listen | A block finished cooking `item` into `result` at `pos`. |
| `block_damage_abort` | server | listen | A server player stopped digging `pos` before the block broke, after the server cleared the dig and sent the block back to the client. |
| `block_dispense_armor` | server | listen | A dispenser equipped `item` on the first eligible `target` in front of its block at `pos`. |
| `block_drops` | server | cancel | NeoForge's `BlockDropsEvent`: a block `state` broke at `pos` and `Block.dropResources` has built the item entities `drops` it will spawn, but nothing is in the world yet. |
| `block_fade` | server | cancel | A block at `pos` is about to fade from `from` to `to` because of melting or coral death. |
| `block_failed_dispense` | server | listen | A dispenser at `pos` failed to dispense `item`. |
| `block_form` | server | cancel | Precipitation is about to form ice or place or thicken snow at `pos`. |
| `block_grow_feature` | server | cancel | A sapling, azalea, mushroom or fungus at `pos` is about to grow into the configured `feature`, `None` when vanilla has none to place (a lone dark oak sapling). |
| `block_highlight` | client | cancel | Before an eligible targeted block's selection outline is built. |
| `block_ignite` | server | cancel | On an authoritative server, fire is about to be placed at `pos` by flint and steel, a fire charge, lava, lightning or fire spread. |
| `block_lock_check` | server | cancel | A guest's block inventory lock check at `pos`. |
| `block_receive_game` | server | cancel | A sculk sensor or shrieker received a vibration for `event` at `pos`. |
| `block_spread` | server | cancel | On an authoritative server, a fluid flows or fire spreads from `from` into `to` (`kind` is `fluid` or `fire`): `FlowingFluid.spreadTo`, and `FireBlock.tick` setting fire to `to` or burning it (`checkBurnOut`). |
| `block_step_on` | server | listen | `Block.stepOn` for a mod block, on its authoritative server level. |
| `block_tool_modification` | both | cancel | A tool is about to transform a block. |
| `blocks` | both | collect | Blocks this mod adds, declared in code rather than one file each (NeoForge's `DeferredRegister.Blocks` drained by `RegisterEvent`): push a `mod_api::Block` per block, the same declaration a content mod's `data/<id>/horizon/block/<path>.json` holds (a vanilla `template`, `properties`, `state_properties`, `shapes`...), with its id in a namespace owned by your mod's manifest. |
| `bonemeal` | both | cancel | `BonemealEvent`: bone meal from `stack` used on the block `state` at `pos` (`BoneMealItem.growCrop`), by `player` or, when it is `None`, by a dispenser or a villager. |
| `break_block` | server | cancel | A player breaks the block `state` at `pos`, once the game allowed it (the held item can break blocks, the debug stick didn't take the click): where `ServerPlayerGameMode.destroyBlock` starts. |
| `bucket_empty` | server | cancel | A server player empties a bucket at `pos` (`fluid`: `minecraft:water`, `minecraft:lava`, `minecraft:powder_snow`; a fish or axolotl bucket empties `minecraft:water`): where `BucketItem.use` has found the block the fluid goes into and the player may build there, right before it is placed. |
| `campfire_start` | server | listen | An item was placed on a campfire at `pos` and began cooking. |
| `crop_grow_post` | server | listen | A crop-like block grew (NeoForge's `CropGrowEvent.Post`). |
| `crop_grow_pre` | server | listen | A crop-like block at `pos` is about to grow on a random tick (NeoForge's `CropGrowEvent.Pre`): crops, stems, nether wart, cocoa, sweet berries and bamboo at their own position; cactus and chorus flowers at the block above; sugar cane at its own position; kelp, weeping and twisting vines and cave vines at the block they grow into. |
| `dispenser_acts` | server | replace | A dispenser at `pos` in `dimension`, facing `facing`, is about to act on `stack` (`DispenserBlock.getDispenseMethod`), once per dispenser action on the authoritative server (not droppers). |
| `enchanted_block_loot` | server | listen | NeoForge's `EnchantedBlockLootEvent`: a block's loot table is reading the level of `enchantment` on `tool` to decide what `state` at `pos` drops, in `ApplyBonusCount.run` or `BonusLevelTableCondition.test`. |
| `farmland_trample` | server | cancel | Called on the server before a landing converts farmland to its base block. |
| `fill_bucket` | server | cancel | A player fills (`stack` an empty bucket) or empties a bucket at `pos`: `BucketItem.use`, where it asks `Level.mayInteract`. |
| `fluid_flow` | both | listen | The current of the fluid block at `pos` (`FlowingFluid.getFlow`, the vector that pushes an entity standing in it) is about to push a player, mob, item, experience orb, armor stand or arrow. |
| `fluid_level_change` | server | cancel | A flowing fluid at `pos` is about to change level. |
| `fluid_place_block` | server | cancel | Liquid at `liquid_pos` is about to place `state` at `pos`. |
| `fluid_tick` | server | cancel | The fluid at `pos` is about to run its scheduled tick (`FlowingFluid.tick`). |
| `fluid_tooltip` | client | listen | `FluidTooltipEvent`: the tooltip of `stack`, a fluid the game draws a slot of (the recipe viewer's fluid wells), once its name, its amount in millibuckets and, with F3+H, its id are in `lines`, each with a colour as `rgba` packs it (`u32::MAX` keeps the text's own). |
| `grindstone_place` | server | cancel | A grindstone at `pos` made its result after its inputs changed, for the server player `player` (UUID) and `player_entity_id` (`GrindstoneEvent.OnPlaceItem`, in `GrindstoneMenu.createResult`): `place` holds the inputs, the vanilla result and an initially empty override. |
| `grindstone_take` | server | cancel | The server player `player` (UUID) and `player_entity_id` took a grindstone's result at `pos` (`GrindstoneEvent.OnTakeItem`, in its result slot's `onTake`): `take` holds the inputs, the experience (-1 for vanilla's, or what `grindstone_place` set) and what the inputs hold afterwards (empty). |
| `hanging_break` | server | cancel | An item frame (`minecraft:item_frame`, `minecraft:glow_item_frame`) or a painting (`minecraft:painting`), `kind` its entity type and `pos` the block it hangs in of the level `dimension` (a `&str`), is about to break: a server player's hit (`player`, their state swapped in, so `PlayerRef::tell` answers them), a blast, or the wall it hangs on gone (`player` None for both). |
| `hanging_place` | server | cancel | A server player places a hanging entity: an item frame (`minecraft:item_frame`, `minecraft:glow_item_frame`), a painting (`minecraft:painting`) or a leash knot (`minecraft:leash_knot`, when a lead is tied to a fence that has none yet), `kind` its entity type and `pos` the block it takes (the fence's for a knot). |
| `hopper_inventory_search` | server | listen | A hopper selected a container while searching for items to push or pull. |
| `hopper_pickup_item` | server | cancel | A hopper at `pos` is about to pick up `count` of `item`. |
| `instantiate_block_state_definition` | client | collect | A blockstate file whose `neoforge:definition_type` a mod registered (`register_block_state_models`) is read: push a `BlockStateModelPick` for each state of `states` that draws a model, or one `Refuse` if the file doesn't read, for a type your mod owns. |
| `inventory_block_start` | server | cancel | A server player is about to open an inventory supplied by a block at `pos`. |
| `leaves_decay` | server | cancel | A non-persistent leaf block at distance 7 is about to decay at `pos`. |
| `left_click_block` | server | cancel | A player left-clicks the block `state` at `pos`, `face` the face hit (`[0, 1, 0]` is the top): where `ServerPlayerGameMode.handleBlockBreakAction` takes a dig's start, after its reach, spawn protection and world border checks, before `Block.attack` (a note block plays) or a creative break. |
| `menu_background` | client | listen | The normal menu blur request, before finite/range validation. |
| `moisture_change` | server | cancel | Farmland at `pos` is about to change moisture from `from` to `to`. |
| `neighbor_notify` | server | cancel | The block at `pos` (`state`) is about to tell its neighbours it changed (NeoForge's `BlockEvent.NeighborNotifyEvent`): at `ServerLevel.updateNeighborsAt`, which notifies all six sides (`skipped_side` is `None`), and `updateNeighborsAtExceptFromFacing`, which notifies all but `skipped_side`. |
| `note_block_change` | server | cancel | A player is tuning a note block on the authoritative server, not in client prediction. |
| `note_block_play` | server | cancel | The note block at `pos` is about to play (NeoForge's `NoteBlockEvent.Play`, in `NoteBlock.playNote`), once a block above no longer mutes it. |
| `piston_post` | server | listen | On an authoritative server, the piston at `piston` pushed the blocks at `push` one step `direction` (`[0, 1, 0]` is up; pulling when it retracts, `extending` false) and destroyed those at `destroy` (NeoForge's `PistonEvent.Post`, at the end of `PistonBaseBlock.moveBlocks`, after the neighbour updates). |
| `piston_pre` | server | cancel | On an authoritative server, the piston at `piston` is about to push the blocks at `push` one step `direction` (`[0, 1, 0]` is up; pulling when it retracts, `extending` false) and destroy those at `destroy` (NeoForge's `PistonEvent.Pre`, in `PistonBaseBlock.moveBlocks`, once its structure resolved). |
| `portal_create` | server | listen | A Nether portal was built for a dimension travel, or an End platform was made for an arrival. |
| `prepare_hopper_transfer` | server | listen | A hopper at `pos` began an item transfer attempt. |
| `right_click_block` | both | cancel | A player right clicks a block (NeoForge's `PlayerInteractEvent.RightClickBlock`): once per hand, where `useItemOn` starts, on the clicking client and on the server (`click` says which, with the block, the hit, the hand and its stack, and the player), for a spectator too (the server then only lets one open a menu). |
| `screen_transparent_background` | client | cancel | `Screen.extractTransparentBackground`: draws the translucent dark background behind an in-game screen (inventory, container, creative, book, sign, editor). |
| `sculk_bloom` | server | listen | A sculk catalyst at `pos` has begun blooming after taking experience. |
| `sign_changed` | server | cancel | A player finished editing a sign: `ServerGamePacketListenerImpl.updateSignText` has taken their update packet and every check has passed (they were editing that sign, within reach, the sign is not waxed, the chat filter let the text through), right before the lines go on the sign. |
| `sponge_absorb` | server | listen | A sponge at `pos` absorbed water and became wet. |
| `use_item_on_block` | both | cancel | `UseItemOnBlockEvent`: after a phase was chosen, before its Part or vanilla behaviour. |
| `vault_change_state` | server | listen | A vault at `pos` changed to `state`. |
| `vault_loot` | server | listen | A vault at `pos` is about to eject a loot item. |

#### Items, inventory and crafting

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `anvil_craft_post` | server | listen | A player took an anvil's result at `pos` and the levels, inputs and anvil wear are done (`AnvilCraftEvent.Post`); a broken anvil is already air. |
| `anvil_craft_pre` | server | cancel | A player took an anvil's result at `pos` (`AnvilCraftEvent.Pre`), before the levels are charged, the inputs used up and the anvil worn. |
| `anvil_damaged` | server | listen | An anvil at `pos` took damage, changing its block from `from` to `to`. |
| `anvil_update` | both | cancel | An anvil at `pos` made vanilla's result with a left input, for `player` (UUID) and `player_entity_id` (`AnvilUpdateEvent`, after `AnvilMenu.createResult`), whenever an input or the name changed: `update` holds the inputs (the right one may be empty, for a rename), the rename text and vanilla's result, and its `output`, `xp_cost` and `material_cost` start at vanilla's for the mods to change; set `cancel` for no result. |
| `armor_stand_manipulate` | server | cancel | A player takes an item from an armor stand or puts one on it (`ArmorStand.interact`, after the stand's disabled-slot checks and `swapItem`'s take/put locks, before anything moves): `stand_id` is the stand's `Entity.getId` and `slot` the one the click reached (`mainhand`, `offhand`, `feet`, `legs`, `chest` or `head`). |
| `brewing_complete` | server | listen | A brewing stand finished brewing at `pos`, after its bottle contents were changed. |
| `brewing_fuel` | server | listen | A brewing stand at `pos` consumed `fuel` and gained brewing fuel. |
| `brewing_start` | server | listen | A brewing stand at `pos` began brewing `ingredient`, after fuel and the timer were set. |
| `can_eat` | both | replace | `Player.canEat` for `stack`, from `Consumable.canConsume` when eating starts and block interaction eligibility for cake and candle cake. |
| `compost_item` | server | listen | A player added a compostable item to the composter at `pos`. |
| `container_loot_filled` | server | collect | Horizon addition: no NeoForge counterpart. |
| `container_opened` | server | cancel | A player opens one of the game's own menus (`ServerPlayer.openMenu`; the job of NeoForge's `PlayerContainerEvent.Open`, which comes after the menu is open and cannot stop it): `kind` is the menu type (`minecraft:generic_9x3`, `minecraft:furnace`, `minecraft:crafting`, `minecraft:merchant`, `minecraft:anvil`, `minecraft:lectern`...; a horse's inventory is `minecraft:horse_inventory`) and `pos` the block it belongs to, none for a villager, a mount or a vehicle's chest. |
| `crafter_craft` | server | listen | A crafter at `pos` completed recipe `recipe`. |
| `creative_slot` | server | cancel | A creative-mode player sets a slot of their inventory from the creative screen (`ServerGamePacketListenerImpl.handleSetCreativeModeSlot`): `slot` is the inventory menu's slot (5 to 8 armour, 9 to 35 the inventory, 36 to 44 the hotbar, 45 the off hand; below 0 is a stack thrown out of the screen, which is dropped) and `item` (`minecraft:air` for an emptied slot) and `count` what is put there. |
| `enchant_item` | server | listen | A server player applied an enchantment at `pos`; `item` is the finished item and `levels` the levels charged. |
| `enchantment_level_set` | server | listen | An enchanting table at `pos` drew the level of one offer (`EnchantmentMenu.slotsChanged`, after `getEnchantmentCost`): `offer` holds the server player whose table it is, the row (0 to 2), the power (bookshelves counted), the item and the level drawn. |
| `furnace_burn` | server | listen | A furnace at `pos` consumed fuel and began or resumed burning it. |
| `furnace_extract` | server | listen | A server player took an output from a furnace at `pos`; `item` is the output item id and `count` is how many were taken. |
| `furnace_smelt` | server | listen | A furnace finished smelting `item` into `result` at `pos`. |
| `furnace_start_smelt` | server | listen | A furnace at `pos` began cooking `item` (its cooking timer advanced from zero). |
| `get_enchantment_level` | both | listen | `GetEnchantmentLevelEvent`: the enchantments gameplay sees on `stack`, starting from its own `minecraft:enchantments`. |
| `get_enchantment_level_for` | both | listen | `GetEnchantmentLevelForEvent`: the same enchantment-level read as `get_enchantment_level`, but only asked when the caller knows who wears or holds the stack. |
| `get_equipment_drop_chance` | server | listen | A dying mob `entity` of `kind` is about to roll its drop chance for the item `stack` in `slot` (`Mob.dropCustomDeathLoot`, NeoForge's `GetEquipmentDropChanceEvent`). |
| `ingredient` | both | collect | A recipe ingredient of a mod's own kind: the entry named a type in a namespace outside `minecraft:`, and the loader kept it for this call. |
| `ingredient_test` | both | listen | A mod's own ingredient kind, matching a concrete stack at craft time: the ingredient's `test` calls here with the kind, its NBT compound and the stack being matched, `accept` already true (a nonempty load-time list named the item; an empty custom list does not filter it). |
| `inventory_move_item` | server | cancel | A hopper is about to move one item between containers, a composter's bone meal included. |
| `item_attribute_modifier` | both | listen | `ItemAttributeModifierEvent`: the attribute modifiers `stack` grants (`ItemStack.getAttributeModifiers`), starting from its own `minecraft:attribute_modifiers` or its item's. |
| `item_consumption_teleport` | server | cancel | A server player or fox that ate chorus fruit (or another item with the `teleport_randomly` consume effect) is about to try a random spot (NeoForge's `EntityTeleportEvent.ItemConsumption`, vanilla's `TeleportRandomlyConsumeEffect.apply`): once per attempt (up to 16), after they stopped riding. |
| `item_crafted` | server | listen | A player took a crafting grid's result (`ResultSlot.onTake`, NeoForge's `PlayerEvent.ItemCraftedEvent`): `player` is the player's uuid, `recipe` the recipe the grid matched (`minecraft:oak_planks`) and `stack` the result it took, with its components. |
| `item_fished` | server | collect | A server player `player` (UUID) and `player_entity_id` reels in a catch: `ItemFishedEvent` in `FishingHook.retrieve`, after fishing loot and its modifiers roll. |
| `item_model_property` | client | listen | Client-side evaluation of a mod's namespaced conditional item model property. |
| `item_model_range_property` | client | listen | Client-side value of a numeric item model property a mod registered (`register_range_select_item_model_property`), asked each time a `minecraft:range_dispatch` on it picks a branch. |
| `item_model_select_property` | client | listen | Client-side value of a categorical item model property a mod registered (`register_select_item_model_property`), asked each time a `minecraft:select` on it picks a case. |
| `item_model_update` | client | listen | A node of a type a mod registered (`register_item_models`, vanilla's `ItemModel.update`) is resolved for a stack, in hand, in a screen or on the ground: push the names of the node's `models` to draw, in order, only for a type your mod owns. |
| `item_picked_up` | server | listen | `player` touches the ground item entity `item` (`ItemEntity.playerTouch`), before its pickup delay and target are checked: it is asked on every tick the two touch, even while the item can't be taken. |
| `item_picked_up_partial` | server | listen | `player` picked up part of a ground stack (`ItemEntity.playerTouch`, right after `Inventory.add` took some of it and left the rest). |
| `item_picked_up_post` | server | listen | `player` picked up the whole stack of the ground item entity `item` (`ItemEntity.playerTouch`, right after `Inventory.add` took all of it), the entity `item_picked_up` named. |
| `item_smelted` | server | listen | `FurnaceResultSlot.checkTakeAchievements` ran for a server player at a furnace, blast furnace or smoker's result slot: `player` is their UUID and `player_entity_id` their network id; `stack` and `amount_removed` are what vanilla passes there (`getSmelting`, `getAmountRemoved`). |
| `item_stacked_on_other` | both | cancel | NeoForge's `ItemStackedOnOtherEvent`, before either item override. |
| `item_tint_source` | client | listen | Client-side colour of an item tint source a mod registered (`register_item_tint_sources`, vanilla's `ItemTintSource.calculate`), asked for each entry on it each time the item is resolved, in hand, in a screen or on the ground. |
| `item_tooltip` | client | listen | `ItemTooltipEvent`: the whole tooltip of `stack` (`ItemStack.getTooltipLines`), once its name, the item's own lines, the components' lines, the advanced lines and the operator warning are in `lines`, each with a colour as `rgba` packs it (`u32::MAX` keeps the text's own). |
| `item_toss` | server | cancel | `ItemTossEvent`: `player` tosses the item entity `item` (`Player.drop`: the drop key, a menu's throw, what no longer fits in the inventory), not what scatters when they die. |
| `items` | both | collect | Items this mod adds, declared in code (NeoForge's `DeferredRegister.Items`), as `blocks` declares blocks: a `mod_api::Item` per item, a content mod's `data/<id>/horizon/item/<path>.json` (a vanilla `template`, `max_stack_size`, `rarity`, `components`, `crafting_remainder`). |
| `menu_clicked` | server | listen | A click in a menu a mod opened (`native_open_menu`), which moved nothing: the menu's `slot` (its own first, then the player's inventory, `native_menu_inventory_slot`; -999 outside), the `button` and the click's `mode` (`ClickType`'s id) of the player `uuid`. |
| `menu_closed` | server | listen | A menu a mod opened closed: the player closed it, the server or a mod opened another, or the player left. |
| `menu_extra_data` | server | replace | Block `block` at `pos` opens its mod menu type `menu` for server player `player`: set `data` to hand the menu's screen bytes of your own (a machine's tier, mode or recipes), up to 32 KiB, which the client reads with `screen_extra_data` while the screen is open (NeoForge's `openMenu` with an extra-data writer). |
| `menu_links` | server | collect | Containers exposed by linked slots of server player `player`'s open mod menu `menu` (`namespace:path`) at `pos`. |
| `mod_container_viewers` | server | listen | The number of players with the mod container at `pos` of `dimension` open went from `previous` to `viewers` (vanilla's `ContainerOpenersCounter`, as a barrel keeps it): 0 to more than 0 is `onOpen`, back to 0 is `onClose`, any change `openerCountChanged`, so a block can show itself open while `viewers` is not 0. |
| `on_equip` | server | listen | A player's equipment slot `slot` (`head`, `chest`, `legs`, `feet`, `offhand`, and `mainhand` on a swap of the hands or `/item`) took a different item, `new` (none when it was emptied): `LivingEntity.onEquipItem`, before the equip sound. |
| `prepare_item_craft` | server | replace | A crafting-grid result was prepared for a server player; `pos` is the crafting table, or None for the inventory grid. |
| `prepare_item_enchant` | server | listen | Enchanting offers were calculated for a server player at `pos`. |
| `prepare_smithing` | server | replace | A smithing result was prepared for a server player at `pos`. |
| `right_click_item` | both | cancel | `PlayerInteractEvent.RightClickItem`, immediately before `Item.use`. |
| `station_closing` | server | collect | A workstation `PlayerRef::open_station` opened is closing for `player` (UUID): the player closed it, it was no longer valid or another menu replaced it. |
| `trade_select` | server | listen | A server player selected trade `index` from `villager` in the trading menu. |

#### World, chunks and ticks

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `biome_colors` | client | listen | Change the colours of `biome` (`minecraft:plains`) on this client: the grass, foliage (leaves, vines), dry foliage (leaf litter) and water tints, 0xRRGGBB, as its biome file and the colormaps set them. |
| `can_continue_sleeping` | server | replace | `CanContinueSleepingEvent`: a sleeping living entity's check that it may go on sleeping, run each tick for a server player the dead and health checks would not already wake, and for a sleeping villager or wandering trader. |
| `cauldron_level_changed` | server | cancel | On an authoritative server, a cauldron at `pos` is about to change fill level from `from_level` to `to_level` for `reason`. |
| `chunk_load` | server | listen | Chunk `chunk` (`[x, z]`, in chunks) of `dimension` came into the world (NeoForge's `ChunkEvent.Load`): `new` when it was just generated, false when read from the save. |
| `chunk_sent` | server | listen | `chunk`'s packet went to `player` (NeoForge's `ChunkWatchEvent.Sent`, at the end of `PlayerChunkSender.sendChunk`), after `chunk_watch`, so a mod can send its own data for the chunk now. |
| `chunk_ticket_level_updated` | server | listen | `chunk`'s ticket level went from `old` to `new` (NeoForge's `ChunkTicketLevelUpdatedEvent`, from `ChunkMap.updateChunkScheduling`): a player, a forced chunk, a portal or the dragon fight's arena began or ceased to keep it alive or ticking. |
| `chunk_unload` | server | listen | Chunk `chunk` of `dimension` left the world (NeoForge's `ChunkEvent.Unload`), with a level that closes for being empty too. |
| `chunk_unwatch` | server | listen | `player` stopped watching `chunk` (NeoForge's `ChunkWatchEvent.UnWatch`, `ChunkMap.dropChunk`): it left their view, or they leave the level (through a portal or a teleport, or respawning) or the server. |
| `chunk_watch` | server | listen | `player` started watching `chunk` (NeoForge's `ChunkWatchEvent.Watch`): the chunk came into their view and goes to their client, once until `chunk_unwatch`. |
| `client_chunk_load` | client | listen | Chunk `chunk` (`[x, z]`, in chunks) of `dimension` came into this client's level from the server (NeoForge's `ChunkEvent.Load` on the client, whose `isNewChunk` is always false there): once per chunk packet the client adopts, after the column and its block entities are in the level, so a listener reads its blocks through `ClientHost`. |
| `client_chunk_unload` | client | listen | Chunk `chunk` (`[x, z]`, in chunks) of `dimension` is leaving this client's level (NeoForge's `ChunkEvent.Unload` on the client): once per resident column, with its blocks still there, just before they are freed. |
| `client_level_load` | client | listen | The client made a new level for `dimension` (NeoForge's `LevelEvent.Load` from the `ClientLevel` constructor): on joining a server, single player's own included, and on changing dimension. |
| `client_level_unload` | client | listen | The client dropped its level for `dimension` (`LevelEvent.Unload` from `Minecraft.setLevel` and `disconnect`): replaced by another dimension's, or on leaving the server. |
| `client_tick_post` | client | listen | Once every client tick, paused or not and on the title screen too, after the window's tick ran (NeoForge's `ClientTickEvent.Post`): the window's `Game` only, so not on a server's own tick (`server_tick` is that moment). |
| `client_tick_pre` | client | listen | Once every client tick, paused or not and on the title screen too, before the window's tick runs (NeoForge's `ClientTickEvent.Pre`): the window's `Game` only, so not on a server's own tick (`server_tick` is that moment). |
| `entities_load` | server | listen | Entities from `chunk` finished loading into the server world. |
| `entities_unload` | server | listen | Entities in `chunk` left the active server world and were queued for saving. |
| `food_level_changed` | server | cancel | The food level of the server player `player` (`name`) is about to change from `from` to `to` (Bukkit's `FoodLevelChangeEvent`): eating, hunger and saturation effects, the exhaustion that costs a point, starving. |
| `level_load` | server | listen | `dimension`'s level opened on the server (NeoForge's `LevelEvent.Load`, posted in `MinecraftServer.createLevels`): before any of its chunks (`chunk_load`), so a mod sets up what it keeps for the level here. |
| `level_save` | server | listen | `dimension`'s level was saved (NeoForge's `LevelEvent.Save`, in `ServerLevel.save`): its level data, chunks and entities, by the autosave, `/save-all`, a pause, a closing level or the server stopping; each level of a world save in turn. |
| `level_tick_post` | both | listen | After a successful level tick pass on either logical side; never after a failed tick. |
| `level_tick_pre` | both | listen | Before a level's tick pass on either logical side. |
| `level_unload` | server | listen | `dimension`'s level closed (NeoForge's `LevelEvent.Unload`, posted after `level.close()` in `MinecraftServer.stopServer`), after its last save: drop what the mod kept for it. |
| `play_level_sound_at_position` | both | cancel | A sound is about to play at `position` in `level` (NeoForge's `PlayLevelSoundEvent.AtPosition`, in `Level.playSeededSound`): every server sound not bound to an entity, mobs' own sounds included (`Entity.playSound` plays at the entity's position). |
| `server_tick` | server | listen | The authoritative server tick's phases: `ServerTickStart` before any level is ticked (NeoForge's `ServerTickEvent.Pre`), `ServerTickEnd` after the last (`.Post`). |
| `server_tick_rate_changed` | server | listen | `/tick rate` set the server's rate, or a `/tick sprint` ended (`MinecraftServer.onTickRateChanged`, before its autosave wait is adjusted). |
| `sleep_finished_time` | server | cancel | Enough players have slept through the night in level `dimension` and the clock is about to move to the morning (NeoForge's `SleepFinishedTimeEvent`, vanilla's `ServerLevel.tick` when it moves the clock to `wake_up_from_sleep`). |
| `thunder_change` | server | cancel | The server changed its thunder state through `/weather`. |
| `tick_target_millis` | client | listen | How many milliseconds a client tick lasts (`Minecraft.getTickTargetMillis`, the target the window's `DeltaTracker.Timer` divides the frame's time by): `millis` comes in as vanilla's (50 at 20 ticks a second, longer when the server's `/tick rate` is slower); set it to speed up or slow down the client's fixed-step clock, a "timer". |
| `time_skip` | server | cancel | The overworld clock is about to jump, from sleep or `/time`. |
| `validate_tickets` | server | listen | `dimension`'s level has loaded the saved chunk tickets of `controller` (NeoForge's `LoadingValidationCallback.validateTickets`), before its first tick and after `level_load`, once for each open and each controller registered with a validator that has tickets in the level. |
| `vanilla_game_event` | server | cancel | A vanilla game event is about to reach its listeners (NeoForge's `VanillaGameEvent`, vanilla's `Level.gameEvent`): every event, in `dimension`. |
| `vehicle_create` | server | listen | A boat or minecart was created at `pos`, after it joined the level. |
| `vehicle_damage` | server | cancel | A vehicle is about to take damage from a player's attack. |
| `vehicle_destroy` | server | listen | A boat or minecart is about to be destroyed at `pos`. |
| `vehicle_moved` | server | listen | A boat or minecart (`kind`, `minecraft:oak_boat`) moved since the last tick (`VehicleMoveEvent`): `from` and `to` are its positions, heard after the tick moved it. |
| `viewed_level_sound` | client | listen | A sound of a level the server opened for viewing (arismoko/horizon#3716) reached this client, and the client asks where in the player's own level to play it. |
| `weather_change` | server | cancel | The server changed its weather parameters. |
| `world_border_bounds_change_finish` | server | listen | A timed world-border size change finished on the server, after its final size was applied. |
| `world_border_bounds_changed` | server | listen | The server world's border bounds changed, either immediately or when its timed size change was requested. |
| `world_border_center_changed` | server | listen | The server world's border center changed. |
| `world_drawn` | client | listen | The world is drawn (see `WorldDrawn`). |
| `world_lifecycle` | server | listen | The world's phases, read-only; `error_text` carries the engine's own open-failure text, empty otherwise. |
| `world_saved` | server | listen | The server completed a world save cycle. |
| `world_view_drawn` | client | listen | A world view is drawn (see `WorldViewDrawn`): once per view taken this frame, after the frame's world and before `world_drawn`, the views seen in a view (its `parent`, one `nesting_depth` deeper) before it. |
| `world_views` | client | collect | Asked once as each frame is built (not on the title screen's panorama): push the world views (see `WorldView`) your mod wants drawn this frame, each parent before the views seen in it. |

#### Commands, chat and network

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `broadcast_message` | server | cancel | A server broadcast line is about to be logged and sent. |
| `chat_recipient` | server | cancel | A public chat line from `sender` (`sender_name`) is about to reach the placed player `recipient`: a player's chat after `server_chat` let it through, or a `/say` or `/me` (signed or not). |
| `chat_suggestion_applied` | client | listen | `chat_suggestion_applied(original, start, end, suggestion, input)` reports a chat completion after Tab or a click. |
| `client_chat` | client | cancel | NeoForge's `ClientChatEvent`: a client is about to send `message`. |
| `client_chat_received` | client | cancel | NeoForge's `ClientChatReceivedEvent`: a chat line from the server is about to show, once Chat Delay lets it through. |
| `client_disconnecting` | client | listen | Before an actual connection is discarded, not on a dimension change. |
| `client_information_updated` | server | listen | NeoForge's `ClientInformationUpdatedEvent`: a server player's client sent new client information (language, view distance, chat, skin parts, main hand, filtering, listing, particles) in play. |
| `client_packet` | client | listen | A play packet from the server reached the window's game, decoded, just before the game handles it (`Connection.genericsFtw` handing it to `ClientPacketListener`): every packet of the play phase, custom payloads included, in the order the server sent them. |
| `client_packet_send` | client | cancel | A play packet the window's game is about to send to the server (`Connection.send`): every packet of the play phase it sends, in the order it sends them, the selected slot that goes out before an attack or a use included. |
| `command` | client | cancel | A command the local player typed or clicked in chat, without its slash, before the server and Horizon's commands see it: set `ran` when the mod ran it. |
| `custom_click_action` | server | cancel | NeoForge's `CustomClickActionEvent`: a server player's client sent a `custom` click action (a dialog button or a text component's click event): its id and optional payload. |
| `customize_chat_overlay` | client | listen | The in-game chat is about to be drawn (NeoForge's `CustomizeGuiOverlayEvent.Chat`, in `Gui.renderChat`): `x` and `y` come in as the chat's bottom-left corner in GUI pixels (0 and the GUI height minus 40, vanilla's place); move them to draw the chat elsewhere. |
| `resource_pack_status` | server | listen | A server player's client answered a resource pack in play (vanilla's `ServerboundResourcePackPacket`): `status` is `Action`'s number, 0 successfully loaded, 1 declined, 2 failed download, 3 accepted, 4 downloaded, 5 invalid URL, 6 failed reload, 7 discarded. |
| `run_mod_command` | server | cancel | Execute a parsed mod command after permission checks and argument resolution. |
| `server_about_to_start` | server | listen | The server is about to open its worlds: nothing of the world is loaded yet (NeoForge's `ServerAboutToStartEvent`, "called before the server begins loading anything"). |
| `server_chat` | server | cancel | NeoForge's `ServerChatEvent`: a player's chat message, through their chain and the chat filter, is about to be broadcast. |
| `server_list_ping` | server | cancel | Paper's `PaperServerListPingEvent`: a client asked a server for its status (the server list's ping, the status request in the status protocol state), and the answer is about to be sent. |
| `server_started` | server | listen | The server has opened its worlds and is about to run its tick loop (NeoForge's `ServerStartedEvent`): every level is loaded, no player has joined, and the first tick has not run. |
| `server_starting` | server | listen | Fired once immediately after `server_about_to_start`, before the server accepts play, on both integrated and dedicated servers. |
| `server_stopped` | server | listen | Fired once at the end of server teardown, after `server_stopping` and the final save attempt and storage release. |
| `server_stopping` | server | listen | The server begins teardown (NeoForge's `ServerStoppingEvent`), including after startup or the tick loop failed: the start of vanilla's `MinecraftServer.stopServer`, with the worlds still loaded and the players still connected and not yet saved, so a mod can save its own data or tell them. |
| `tab_complete` | server | collect | A guest asked to complete `command`; owned API `Suggestion` records may be added to or filtered from `suggestions`. |
| `teleport_command` | server | cancel | A server entity is about to be teleported by `/tp` or `/teleport` (NeoForge's `EntityTeleportEvent.TeleportCommand`, in `TeleportCommand.performTeleport`, once per teleported entity): `dimension` is the level it arrives in, `entity` its network id, `from` its feet now and `to` where the command sends it. |

#### Client: screens, interface and input

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `client_pause_change_post` | client | listen | NeoForge's `ClientPauseChangeEvent.Post`: the pause state changed. |
| `client_pause_change_pre` | client | cancel | NeoForge's `ClientPauseChangeEvent.Pre`: cancel a proposed pause change. |
| `component_hover` | client | cancel | `GuiGraphicsExtractor.componentHoverEffect`: the hover of the text component under the cursor, about to be drawn, in the chat, a book or a dialog. |
| `customize_boss_event_progress` | client | cancel | One boss bar is about to be drawn (NeoForge's `CustomizeGuiOverlayEvent.BossEventProgress`, in `BossHealthOverlay`): `bar` is the bar, `x` and `y` come in as its top-left corner in GUI pixels (91 left of the screen's centre; 12 for the first bar), and `increment` as the spacing to the next bar's `y` (19). |
| `debug_entry_lines` | client | collect | The lines of one enabled F3 debug entry a mod registered (`register_debug_entries`), asked each frame the overlay draws it. |
| `dimension_transition_screen_draw` | client | listen | Draw the complete selected loading screen with its GUI drawing context. |
| `framerate_limit` | client | listen | The frame rate this window is about to be held to: Max Framerate after the inactivity throttle and Battery Mode (260 is unlimited). |
| `gui_layer_draw` | client | listen | Draw one HUD layer a mod registered (`register_gui_layers`), each frame the HUD shows, between that layer's `gui_layer_pre` and `gui_layer_post`; not asked when a mod's `gui_layer_pre` hid it. |
| `gui_layer_post` | client | listen | After one of vanilla's HUD layers (or after the layer a mod hid), with the ids of `gui_layer_pre`: draw here to put something above the layer (`RenderGuiLayerEvent.Post`). |
| `gui_layer_pre` | client | cancel | Before one of vanilla's HUD layers is drawn (`RenderGuiLayerEvent.Pre`): draw here to put something below the layer, or set `canceled` to hide it. |
| `health_hud` | client | replace | The player-health HUD rows, once a frame the HUD draws them (`HealthHud::layout`, not under F1): `inputs` carries the health, food, air and vehicle state the layout reads, `wi`/`hi` the screen's GUI size. |
| `input_key` | client | listen | A key event after handling, the window's or the control socket's `key` (one press, then one release for a tap), including repeats and consumed shortcuts (`InputEvent.Key`). |
| `interaction_key_mapping_triggered` | client | cancel | An interaction mapping is about to act on the tick's crosshair target. |
| `key_mapping_pre` | client | cancel | A physical key or mouse button bound to the gameplay mapping `mapping` (`key.jump`, `key.attack`...) changes state while playing, before the game acts on it. |
| `left_click_empty` | client | listen | The local player left-clicks empty space, with any main-hand stack. |
| `minimap` | client | replace | The minimap a native mod shows: set `minimap` when it is none yet. |
| `mouse_button_post` | client | listen | An accepted raw button event finished dispatch, including a screen handler's early return. |
| `mouse_button_pre` | client | cancel | Raw mouse button input on any screen, before dispatch; cancel skips dispatch and Post. |
| `mouse_scrolling` | client | cancel | Gameplay scrolling after accumulation, before hotbar/spectator handling. |
| `movement_input` | client | listen | Local player movement input, after reading it and before applying it. |
| `panel_event` | client | listen | The player did `event` on mod `mod_id`'s panel (`native_panel_items`): act on it when it is yours. |
| `right_click_empty` | client | listen | The local player right-clicks empty space with an empty `hand`. |
| `screen_char_typed_post` | client | cancel | A typed character that no text box took (`ScreenEvent.CharacterTyped.Post`, vanilla's `Screen.charTyped` returning false), once per printable character that `screen_char_typed_pre` let through: `c` and `modifiers` as there. |
| `screen_char_typed_pre` | client | cancel | A character was typed at the open screen (`ScreenEvent.CharacterTyped.Pre`, vanilla's `Screen.charTyped`): `screen` is the screen it is, `c` the character, `modifiers` GLFW's bits (shift 1, control 2, alt 4, super 8), and `field` a copy of the focused text box. |
| `screen_closing` | client | listen | The `screen` that was open is being replaced (`ScreenEvent.Closing`), after `screen_opening` let it. |
| `screen_exclusion_areas` | client | collect | The hook `native_screen_exclusion` asks: a mod adds the rectangles it draws over `screen` (a side panel, a button column) to `areas`, in GUI pixels, `[x, y, width, height]`. |
| `screen_init_post` | client | listen | A screen was observed opening or changing rectangle, at most once per frame (`ScreenEvent.Init.Post`). |
| `screen_key_pressed_post` | client | cancel | A key press that no screen took (`ScreenEvent.KeyPressed.Post`, vanilla's `Screen.keyPressed` returning false): `key`, `repeat` and `modifiers` as in `screen_key_pressed_pre`. |
| `screen_key_pressed_pre` | client | cancel | A key went down over the open screen (`ScreenEvent.KeyPressed.Pre`, vanilla's `Screen.keyPressed`): `screen` is the screen it is, `key` vanilla's name for the key (`key.keyboard.e`) — a key with no name fires nothing — `repeat` whether it is the keyboard's auto-repeat of a held press (Horizon's extra; NeoForge folds repeats into the event), and `modifiers` GLFW's bits (shift 1, control 2, alt 4, super 8). |
| `screen_key_released_post` | client | cancel | A key release that no screen took (`ScreenEvent.KeyReleased.Post`, vanilla's `Screen.keyReleased` returning false): `key` and `modifiers` as in `screen_key_released_pre`. |
| `screen_key_released_pre` | client | cancel | Before an open screen handles a named key's release (same screen gate as key presses). |
| `screen_mouse_button_pressed_post` | client | listen | A mouse button went down over the open screen and the screen has handled it (`ScreenEvent.MouseButtonPressed.Post`, vanilla's `Screen.mouseClicked` returning): `screen`, `button` and `modifiers` as in `screen_mouse_button_pressed_pre`. |
| `screen_mouse_button_pressed_pre` | client | cancel | A mouse button went down over the open screen (`ScreenEvent.MouseButtonPressed.Pre`, vanilla's `Screen.mouseClicked`): `screen` is the screen it is, `screen.mouse` where the pointer is (so there is no position of its own), `button` the button's GLFW index (0 left, 1 right, 2 middle, 3 back, 4 forward; a button past those is the platform's own number, which differs between systems) and `modifiers` GLFW's bits (shift 1, control 2, alt 4, super 8). |
| `screen_mouse_button_released_post` | client | listen | A mouse button went up over the open screen and the screen has handled it (`ScreenEvent.MouseButtonReleased.Post`, vanilla's `Screen.mouseReleased` returning), with the fields of `screen_mouse_button_released_pre`. |
| `screen_mouse_button_released_pre` | client | cancel | A mouse button went up over the open screen (`ScreenEvent.MouseButtonReleased.Pre`, vanilla's `Screen.mouseReleased`), with the same `screen`, `button` and `modifiers` as `screen_mouse_button_pressed_pre` and fired at the same point. |
| `screen_mouse_dragged_post` | client | listen | A held-button pointer movement has gone through the open screen's drag handlers (`ScreenEvent.MouseDragged.Post`, vanilla's `Screen.mouseDragged`): `button`, `delta` (GUI pixels, `screen.mouse` the new position) and `modifiers` as in `screen_mouse_dragged_pre`. |
| `screen_mouse_dragged_pre` | client | cancel | Before a real held-button movement reaches screen drag handlers. |
| `screen_mouse_scrolled_post` | client | listen | The wheel turned over the open screen and the screen's scroll handlers have run (`ScreenEvent.MouseScrolled.Post`, vanilla's `Screen.mouseScrolled`): `scroll` and `modifiers` as in `screen_mouse_scrolled_pre`. |
| `screen_mouse_scrolled_pre` | client | cancel | The wheel turned over the open screen (`ScreenEvent.MouseScrolled.Pre`, vanilla's `Screen.mouseScrolled`): `screen` is the screen it is, `screen.mouse` where the pointer is (so there is no position of its own), `scroll` the wheel's movement in x and y as the game's own scroll handler scales it (a line times the wheel sensitivity: with Discrete Scrolling on, a notch is ±1, its sign only; a touchpad's pixels are divided by 30), and `modifiers` GLFW's bits (shift 1, control 2, alt 4, super 8). |
| `screen_opening` | client | replace | `Gui.setScreen` is about to replace the `current` screen with `new` (`ScreenEvent.Opening`; the screen `playing` is none: vanilla's null). |
| `screen_preedit_post` | client | listen | After a screen's preedit handler (`ScreenEvent.Preedit.Post`) and only when no text field took the composition (the handler returned false: vanilla's focused field takes it, so this is a preedit no field wanted) and `screen_preedit_pre` did not cancel. |
| `screen_preedit_pre` | client | cancel | Before a screen's preedit handler (`ScreenEvent.Preedit.Pre`, vanilla's `GuiEventListener.preeditUpdated`): the input method is composing `full_text` (empty when the composition ended), with the caret at byte `caret_position`, split into `blocks` of which `focused_block` is the one being converted. |
| `screen_room` | client | collect | Room for a mod's own panel right of the open container screen's window, in GUI pixels: `right` starts at 0; raise it to the width your panel takes beside the window, its gap included, and leave it be otherwise. |
| `screenshot` | client | cancel | An F2 screenshot has been read back, before either PNG is written (`ScreenshotEvent`). |
| `toast_add` | client | cancel | A client toast is about to enter its queue (`ToastAddEvent`). |
| `window_resize` | client | listen | After a window resize or applying a changed GUI scale, fullscreen, or Unicode font setting. |

#### Client: rendering, sound and particles

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `arm_pose_transform` | client | listen | Apply a registered pose (`HumanoidModel.ArmPose`'s transformer). |
| `bake_model_loader` | client | collect | A model whose `loader` a mod registered (`register_model_loaders`) is baked: push its quads to `quads` for a loader your mod owns. |
| `camera_distance` | client | listen | Final scaled third-person camera distance, before visual-shape collision clipping. |
| `camera_pose` | client | listen | The camera this frame is drawn from, `partial` ticks past the last tick, after vanilla placed it (`Camera.update`): its position, and its yaw and pitch in degrees. |
| `camera_shake` | client | listen | The camera shake native mods ask for in this frame, `partial` ticks past the last tick (0 to 1): add your pitch, yaw and roll in degrees and your offset in blocks, along the camera's right, up and back, to `shake`, so several mods' shakes add up. |
| `camera_submersion` | client | listen | Camera submersion fog type (`Camera.getFluidInCamera`), checked before fog and camera fluid effects are set up. |
| `camera_type_cycle` | client | listen | `key.togglePerspective` (F5) is cycling the window's camera type (`Minecraft.handleKeybinds` calling `Options.setCameraType(getCameraType().cycle())`): `previous` is the type it leaves, and `next` comes in as vanilla's next one (first person, third person behind, third person in front, then first person again); set `next` to another, to skip a type or keep one. |
| `client_resources_reloaded` | client | listen | Render-thread notification after a resource-pack reload has applied successfully. |
| `compute_fog_color` | client | listen | Computed world fog RGB, after ordinary effects and before upload. |
| `compute_fov` | client | listen | World field of view, after normal calculation and zoom and before projection/Nausea. |
| `compute_fov_modifier` | client | listen | Player FOV scale, once per client tick with a player, after vanilla effects and before zoom and easing. |
| `emissive` | client | collect | At most 32 pairs of already-loaded base and transparent overlay sprites. |
| `flip_frame` | client | listen | Once for each frame presented, in a world or not, after that frame's `render_frame_post` (NeoForge's `FlipFrameEvent`): the place to rotate per-frame GPU data. |
| `get_arm_pose` | client | replace | NeoForge's `IClientItemExtensions.getArmPose`: the pose of the arm that holds `stack` in `hand`, as a humanoid model draws it (local and remote players and humanoid mobs). |
| `hand_drawn` | client | listen | The first-person hand is drawn (see `HandDrawn`), on every frame `world_drawn` runs, with no hand too. |
| `lightning_strike` | server | cancel | A server lightning bolt is about to be created. |
| `map_decorations` | client | collect | Icons a mod puts on map `map` each time it is drawn: held, in an item frame or on the cartography table's screen (a map's pixels are its own; nothing is saved in the map). |
| `model_baking_completed` | client | listen | Every model of a resource load is baked and published (`ModelEvent.BakingCompleted`): the block and item models and the standalone ones mods asked for (`standalone_models`), in the store `model` now reads. |
| `music` | client | listen | Choose this client's music, once a client tick: `music` and `volume` hold what vanilla chose (`Minecraft.getSituationalMusic` and `getMusicVolume`: the biome's track, the creative or underwater one, a boss's, the menu's; None for silence). |
| `music_layers` | client | collect | The tracks to play over (or instead of) the music on this client, asked once a client tick: push a `MusicLayer` (a sound event, looped, and its volume, 0 to 1) for each. |
| `play_sound` | client | listen | `starting` is true for starts and loop replays, false for a live channel refresh. |
| `play_sound_source` | client | listen | A sound that is not streamed has started on `channel` (`PlaySoundSourceEvent`), after the engine set its pitch, volume, attenuation, looping and position: `sound` is what is playing, after `play_sound`'s edits. |
| `play_streaming_source` | client | listen | A streamed sound (music, a record) has started on `channel` (`PlayStreamingSourceEvent`): `sound` is what is playing, after `play_sound`'s edits. |
| `shader_params` | client | listen | The values native mods hand their shader packs in this frame, `partial` ticks past the last tick (0 to 1): a pack's WGSL reads them as `frame.mods` (eight `vec4<f32>`). |
| `sound_engine_load` | client | listen | The sound engine opened its output (`SoundEngineLoadEvent`): once at window start, after each resource reload (with the `context` of the new resource generation) and when the output device changes. |
| `sprite_source` | client | collect | An entry of an atlas definition whose `type` a mod registered (`register_sprite_sources`, vanilla's `SpriteSource.run`), asked at the entry's place in the definition's `sources`, each time the atlas is loaded or reloaded, on the resource loader. |
| `standalone_models` | client | collect | Models no blockstate names, registered once on the client before resources load. |
| `submit_custom_geometry` | client | listen | Submit unlit geometry after particle submission, before opaque rendering (NeoForge 26.3 `SubmitCustomGeometryEvent`, vanilla `LevelRenderer.submitFeatures`). |
| `texture_atlas_stitched` | client | listen | An atlas was stitched and published (`TextureAtlasStitchedEvent`): `atlas` names it. |
| `visuals` | client | listen | The world frame's local, world-space rim and colored-light sources. |
| `zoom` | client | replace | The zoom a native mod asks for, applied while playing (the spyglass's own view wins): set `zoom` when it is none yet. |

#### Other

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `client_mining_predict` | client | cancel | Veto a predicted client dig after the interaction mapping and vanilla eligibility checks. |

### Block parts (`BlockPart`)

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `adjacent_block_path_type` | server | replace | The optional contribution of the neighbour `event.state` at `event.pos` (`IBlockExtension.getAdjacentBlockPathType`). |
| `affect_neighbors_after_removal` | server | collect | `Block.affectNeighborsAfterRemoval` for a mod's block: an authoritative server removed the block `state` from `pos` (`moved`: a piston pushed it) and has run its template class's own updates. |
| `analog_output` | server | collect | `BlockState.getAnalogOutputSignal`: the signal a comparator reads from the block `state` at `pos`, `face` the side that touches the comparator (or the conductor in front of it). |
| `beacon_color_multiplier` | both | collect | The colour the block `state` at `pos` gives the beam of the beacon at `beacon_pos` (`IBlockExtension.getBeaconColorMultiplier`), asked as the beacon builds its beam sections one block at a time. |
| `block_jump_factor` | both | listen | The jump factor of a block state this part is attached to (`Block.getJumpFactor`, which `Entity.getBlockJumpFactor` reads for the player, the cube, the panda, the rabbit and every other living jumper) is being read; vanilla's values are 0.5 for honey and 1.0 for the rest. |
| `block_path_type` | server | replace | The optional classification of `event.state` at `event.pos` (`IBlockExtension.getBlockPathType`). |
| `block_render_visibility` | client | collect | Hides a block state from the mesher: set `event.hidden` and the state draws nothing, as if its blocks were air. |
| `block_signal` | server | listen | The redstone signal the block `state` at `pos` gives on its `face`, the side that touches the block reading it: `BlockState.getDirectSignal` when `direct` (what passes through a block it touches, as a lever's does), else `getSignal`. |
| `block_speed_factor` | both | listen | The speed factor of a block state this part is attached to (`Block.getSpeedFactor`, which `Entity.getBlockSpeedFactor` reads for the player, mobs, boats and minecarts where vanilla reads it) is being read; vanilla's values are 0.4 for honey and soul sand and 1.0 for the rest. |
| `bounce_on` | server | collect | An entity bounced off a block state this part is attached to (`Block.bounceOn`), after falling `event.fall_distance` blocks. |
| `bounce_restitution` | both | collect | An entity landed on a block state this part is attached to and asks its restitution (`IBlockExtension.getBounceRestitution`, which `Entity.getBlockBounciness` asks of the block an entity lands on): the factor its vertical speed is multiplied by when it hits the block from above. |
| `can_be_connected_to` | server | replace | On an authoritative server, whether the block `state` at `pos` lets the fence, wall, or iron bars or pane `connector` beside it connect to it (`IBlockExtension.canBeConnectedTo`): the block being connected to answers, not the connecting one. |
| `can_be_hydrated` | server | replace | Whether `fluid` at `fluid_pos` waters the farmland this part is attached to, the block `state` at `pos` (`IBlockExtension.canBeHydrated`): set `hydrates` to `Some(true)` or `Some(false)`, or leave it `None` for vanilla's rule, a fluid in `#minecraft:water`. |
| `can_drop_from_explosion` | server | replace | On an authoritative server, whether the block `state` at `pos`, destroyed by `explosion`, drops (`IBlockExtension.canDropFromExplosion`), asked where `BlockBehaviour.onExplosionHit` asks `Block.dropFromExplosion`. |
| `can_entity_destroy` | server | replace | On an authoritative server, whether `entity` may destroy the block `state` at `pos` (`IBlockExtension.canEntityDestroy`), asked where vanilla asks its per-block rule: the wither's block break (`WitherBoss.canDestroy`: not `#wither_immune`), a dangerous wither skull's blast (the entity is the skull, readable during the blast; only a block it may destroy has its resistance capped at 0.8), the dragon's `checkWalls` (a `#dragon_transparent` block is skipped before the part is asked, and a `#dragon_immune` block stays even on a yes), a zombie's door breaking (asked as the goal starts) and the turtle-egg `RemoveBlockGoal`'s target choice. |
| `can_harvest_block` | both | collect | Whether the player `player`'s tool harvests the block `state` at `pos` (`IBlockExtension.canHarvestBlock`, what `Player.hasCorrectToolForDrops` asks of the block). |
| `can_stick_to` | server | collect | Whether the block state this part is attached to sticks to `other`, the state beside it (`IBlockExtension.canStickTo`): set `sticks` to decide. |
| `can_survive` | server | replace | On an authoritative server, `BlockState.canSurvive` for `state` at `pos` (`BlockBehaviour.canSurvive`), before the block answers it: as it is placed (`BlockItem.canPlace`), from a shape update of a block whose template checks it (a plant, a torch, a sign: false breaks the block with its drops), and wherever else vanilla asks. |
| `can_survive_v2` | server | replace | `can_survive` again, for the world generation of the server: while a feature places `state` at `pos`, its parts answer `survives` here, as they answer `survives` in the live game. |
| `can_sustain_plant` | server | replace | On an authoritative server, `IBlockExtension.canSustainPlant` for the soil `ground` at `ground_pos` and the plant `state` above it, asked before the plant answers its own `mayPlaceOn` (`may_place_on`) wherever vanilla's plant rule reads its ground: when the plant is placed, from its shape update and wherever else vanilla asks. |
| `can_sustain_plant_v2` | server | replace | `can_sustain_plant` again, for the world generation of the server: while a feature places the plant `state` on the soil `ground` at `ground_pos`, the soil's parts answer `sustains` here. |
| `clone_item_stack` | server | replace | `BlockBehaviour.getCloneItemStack`, asked by `ServerGamePacketListenerImpl.handlePickItemFromBlock`. |
| `connects_to` | server | collect | Whether a fence, wall, or iron bars or pane of yours connects a side: set `connected` to say whether the block `state` at `pos` connects toward `neighbour`, the state beside it, `direction` pointing from the block to that neighbour and `face_solid` whether the neighbour's face toward the block is sturdy. |
| `contents_changed` | server | listen | A server-only, non-cancellable backing-block notice at Container.setChanged / BlockEntity.setChanged, after the write and before dirty/comparator updates. |
| `dispense_method` | server | replace | `DispenserBlock.getDispenseMethod`, before the item's `dispense_item`: the block the dispenser faces chooses what the dispense does, for any item, including vanilla items, on the authoritative server (not droppers). |
| `enchant_power_bonus` | both | collect | The enchanting power the block `state` at `pos` gives the enchanting table at `table_pos` (`IBlockExtension.getEnchantPowerBonus`), asked for each of the table's bookshelf offsets (`EnchantingTableBlock.BOOKSHELF_OFFSETS`) whose block between it and the table is in `#minecraft:enchantment_power_transmitter`. |
| `entity_inside` | server | collect | An entity's block sweep crossed a block state this part is attached to (`Block.entityInside`), and the part is told mid-sweep, where vanilla runs the block's own `entityInside`. |
| `exp_drop` | server | replace | Experience before enchantments (`IBlockExtension.getExpDrop`). |
| `explosion_resistance` | server | replace | On an authoritative server, the explosion resistance of the block `state` at `pos` against `explosion` (`IBlockExtension.getExplosionResistance`), asked as a ray of the blast enters the block (`ExplosionDamageCalculator.getBlockExplosionResistance`). |
| `fall_on` | server | collect | An entity landed on a block state this part is attached to (`Block.fallOn`), after falling `event.distance` blocks. |
| `falling_tick` | server | listen | A falling block of `state` ticked and is still in the air (`IFallableExtension.fallingTick`): `entity` is the falling block, and `pos` the block it is in (`Entity.blockPosition`) after its move. |
| `fire_odds` | server | replace | On an authoritative server, how fire treats the block `state` at `pos` (`FireBlock.setFlammable(block, igniteOdds, burnOdds)`), asked from its side `face`, the one that faces the fire or the air fire may take: put `(ignite, burn)` in `odds` for your own blocks, and vanilla's table isn't read. |
| `friction` | both | listen | The friction of a block state this part is attached to (`IBlockExtension.getFriction`) is being read for the entity on it; vanilla's values are 0.6 for most blocks, 0.98 for ice, packed ice and frosted ice, 0.989 for blue ice and 0.8 for slime. |
| `has_analog_output` | server | collect | `BlockState.hasAnalogOutputSignal`: set `output` for a block of yours that comparators read (how full or far along it is, as a composter's level), so they ask `analog_output`. |
| `ignited_by_lava` | server | replace | On an authoritative server, whether lava sets fire beside the block `state` at `pos` (`LavaFluid.isFlammable`, the only reader of `BlockStateBase.ignitedByLava`), asked from its side `face`, the one that faces the air the fire would take, as lava's random tick looks for something to light. |
| `is_bonemeal_success` | server | collect | `BonemealableBlock.isBonemealSuccess` for a block that answered `is_valid_bonemeal_target`, after it did: set `success` to grow it this time. |
| `is_fertile` | server | collect | Whether the soil this part is attached to, the block `state` at `pos` under a growing crop, counts as moist (`IBlockExtension.isFertile`): set `fertile` to decide. |
| `is_fire_source` | server | replace | On an authoritative server, whether the block `state` at `pos`, under a fire, keeps that fire burning (`FireBlock.tick`'s check of the dimension's `infiniburn` tag), asked on every tick of the fire above it: `Some(true)` makes the fire endless, as netherrack does in the overworld (rain doesn't put it out and it never burns out), and `Some(false)` refuses a block the tag lists. |
| `is_ladder` | both | listen | Whether a living entity climbs the block state this part is attached to (`IBlockExtension.isLadder`, which `LivingEntity.onClimbable` asks of the block the entity stands in). |
| `is_pathfindable` | server | collect | Changes whether a path of a `PathComputationType` (`Land`, `Water` or `Air`) may go through a block state. |
| `is_signal_source` | server | listen | `BlockState.isSignalSource`: set `source` for a block of yours that gives a signal, so redstone wire beside it connects to it. |
| `is_slime_block` | server | collect | Whether the block state this part is attached to counts as a slime block to a piston (`IBlockExtension.isSlimeBlock`, which `PistonStructureResolver` asks wherever it tells slime from honey): set `slime` to decide. |
| `is_sticky_block` | server | collect | Whether the block state this part is attached to is sticky to a piston (`IBlockExtension.isStickyBlock`): set `sticky` to decide. |
| `is_valid_bonemeal_target` | both | replace | A block nothing in Horizon knows a `BonemealableBlock` for is about to take bone meal: vanilla's `BonemealableBlock.isValidBonemealTarget`, `state` the block's own state at `pos`. |
| `item_after_vertical_collision` | both | cancel | `native_item_after_vertical_collision` lends a dropped item's active stack after vertical collision response, before block effects, merging and pickup. |
| `light_emission` | both | replace | The light the block `state` emits at `pos` in `dimension` (`IBlockExtension.getLightEmission`), for a block that declares `dynamic_light: true`. |
| `makes_open_trapdoor_above_climbable` | both | listen | Whether an open trapdoor above the block state this part is attached to climbs like a ladder (`IBlockExtension.makesOpenTrapdoorAboveClimbable`, which `LivingEntity.onClimbable` asks of the block under the open trapdoor a living entity stands in). |
| `may_place_on` | server | replace | On an authoritative server, `VegetationBlock.mayPlaceOn` for the plant `state` on `ground` at `ground_pos`, before the plant answers it: the blocks a mod plant (one whose template is a vanilla plant, such as a crop) can stand on. |
| `may_place_on_v2` | server | replace | `may_place_on` again, for the world generation of the server: while a feature places the plant `state` on `ground` at `ground_pos`, its parts answer `may` here, and the soil's parts answer `can_sustain_plant_v2` before this. |
| `minecart_pass` | server | listen | A minecart passed over a rail block state this part is attached to (the moment `AbstractMinecart` calls `activateMinecart` on an activator rail, for every rail; Forge's `IBaseRailExtension.onMinecartPass`, which NeoForge 26.3 no longer has, so this hook has no counterpart). |
| `neighbor_changed` | server | listen | An authoritative server is about to deliver `neighborChanged`. |
| `on_block_exploded` | server | listen | An explosion destroys this block, before removal and while its live block entity is readable. |
| `on_block_state_change` | both | listen | A write through `Level.setBlock` held and the block at `pos` changed from `old` to `new` (`IBlockExtension.onBlockStateChange`), heard at the end of the write, where vanilla runs `updatePOIOnBlockStateChange`: after the block-entity removal, `on_place`, the client update, the neighbour updates and the neighbours' shape updates. |
| `on_caught_fire` | server | listen | On an authoritative server, a neighbour of fire passed its burn roll. |
| `on_destroyed_by_player` | server | replace | A player removes the block `state` at `pos` (`IBlockExtension.onDestroyedByPlayer`, where `ServerPlayerGameMode.destroyBlock` runs `playerWillDestroy` and removes the block), with `tool`, the stack in the player's hand (`None` for a bare hand), and `will_harvest`, whether the break will drop loot as far as the tool goes (never in creative, and false for a tool the block refuses). |
| `on_destroyed_by_push_reaction` | server | listen | On an authoritative server, a piston's move destroys the block `state` at `pos` (`IBlockExtension.onDestroyedByPushReaction`), `direction` the way the blocks move and `fluid` the fluid state the cell holds. |
| `on_neighbor_change` | server | listen | A neighbouring block entity changed, was created or removed. |
| `on_tree_grow` | server | replace | A trunk placer is about to place the block under its trunk at `pos`, where the soil `state` this part is attached to stands (`IBlockExtension.onTreeGrow`, asked from `TrunkPlacer.placeBelowTrunkBlock`). |
| `perform_bonemeal` | server | listen | `BonemealableBlock.performBonemeal` for a block that answered `is_valid_bonemeal_target` and whose `is_bonemeal_success` said yes: grow it here, `state` its state at `pos`. |
| `piston_carries_block_entity` | both | replace | Whether a piston carries your block `state` at `pos` with its block entity: asked at the last step of `PistonBaseBlock.isPushable`, after the block's push reaction, for a block that is otherwise pushable, whenever a piston's structure takes the block in. |
| `piston_push_reaction` | both | replace | Resolve a state's piston reaction at load and reload on both sides. |
| `play_hit_sound` | client | cancel | Asked before a mining hit sound, including the initial hit, every fourth breaking tick, and the level-event route. |
| `rail_can_make_slopes` | server | collect | Whether the rail `state` at `pos` may climb toward a raised rail beside it (`IBaseRailBlockExtension.canMakeSlopes`). |
| `rail_direction` | both | collect | The track the rail `state` at `pos` lays for `cart` (`IBaseRailBlockExtension.getRailDirection`), so a switch track or a one-way rail can turn a cart differently from its stored shape. |
| `rail_flexible` | server | collect | Whether the rail `state` at `pos` may make corners (`IBaseRailBlockExtension.isFlexibleRail`). |
| `rail_valid_shape` | server | collect | Whether the rail `state` may take `shape` (`IBaseRailBlockExtension.isValidRailShape`), asked as it connects to its neighbours (`RailState`) before the shape is written. |
| `random_tick` | server | cancel | An authoritative server's `ServerLevel.tickChunk` is about to random-tick the block `state` at `pos` (`BlockState.randomTick`): only blocks that tick randomly, a mod's as its vanilla template block does. |
| `remove_block` | server | replace | Choose the state left by removing your block. |
| `respawn_position` | server | collect | Where a player whose respawn point is the block `state` at `pos` stands up when they respawn there (`IBlockExtension.getRespawnPosition`), or that they can't respawn there. |
| `rotate` | server | collect | A structure template is about to place the block `state` at `pos` turned by `rotation` (`IBlockExtension.rotate`, where vanilla runs `state.mirror(mirror).rotate(rotation)`): `state` is already mirrored, and `rotated` starts at the game's own answer (vanilla's table, or a mod block's template's). |
| `should_check_weak_power` | server | collect | `IBlockExtension.shouldCheckWeakPower`: set `check` to decide whether indirect power is read through this block. |
| `should_display_fluid_overlay` | client | collect | Whether a fluid's side beside a block state uses the fluid's overlay sprite (water's `water_overlay`) instead of its flowing sprite (`IBlockExtension.shouldDisplayFluidOverlay`): set `event.overlay`. |
| `should_hide_adjacent_fluid_face` | client | collect | Whether a block state hides the face a fluid beside it draws toward it (`IBlockExtension.shouldHideAdjacentFluidFace`): set `event.hidden` and the fluid's face toward the block is not drawn. |
| `should_redstone_wire_connect_to` | server | collect | `BlockBehaviour.shouldRedstoneWireConnectTo`: set `connected` to say whether redstone wire connects to the block `state` at `pos`. |
| `smoke_source` | server | replace | On an authoritative server, `CampfireBlock.isSmokeSource` for the block `state` under a campfire, as the campfire is placed or the block under it changes: `Some(true)` for your own blocks makes the campfire a signal fire whose smoke rises higher, as over a hay bale. |
| `sound_type` | both | listen | The sound type of a block state this part is attached to (`IBlockExtension.getSoundType`) is being read, to play one of its sounds. |
| `state_at_viewpoint` | client | collect | The block state the camera at `viewpoint` sees of the block `state` at `pos` (`IBlockExtension.getStateAtViewpoint`), so a partial fluid block, a tank or a waterlogged mod block can show underwater fog only while the eye is really inside the liquid. |
| `state_for_placement` | server | replace | A block is about to take the state it is placed in (`Block.getStateForPlacement(BlockPlaceContext)`), `state` the block's own default state. |
| `tick_block` | server | cancel | An authoritative server's `ServerLevel.tickBlock` is about to run a due scheduled tick of the block `state` at `pos` (`BlockState.tick`): the block is still the one the tick was scheduled for. |
| `tool_modified_state` | both | cancel | The state the block `state` becomes under a tool action (`IBlockExtension.getToolModifiedState`: an axe's strip, scrape or wax-off, a shovel's flatten or douse, a hoe's till), asked of the block a tool is used on (`tool.pos`). |
| `update_shape` | server | replace | An authoritative server is about to compute the state the block `state` at `pos` takes now that its neighbour in `direction` is `neighbour` (`BlockBehaviour.updateShape`). |
| `use_item_on` | server | replace | `BlockBehaviour.useItemOn`: the block's own behaviour, with the held item, on the server for each player. |
| `use_without_item` | server | replace | `BlockBehaviour.useWithoutItem`: the captured state's empty-hand method, after useItemOn asks for it, on the server and main hand only. |
| `weak_changes` | server | collect | Whether this block receives block entity changes through an adjacent conductor. |

### Block entity parts (`BlockEntityPart`)

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `block_entity` | both | replace | Initial saved data for a declared block entity (`EntityBlock.newBlockEntity`), fired inside the write that creates it (`LevelChunk.setBlockState`), on a server and on a client of one. |
| `block_entity_client_tick` | client | listen | A client tick of this client's copy of the mod block entity at `pos` (`ClientLevel.tickBlockEntities`), when its `id` is yours and its block's state ticks on the client (`block_entity_client_ticks`): once per client tick while the level runs, in vanilla's ticker order among the others, whether or not the copy is drawn. |
| `block_entity_client_ticks` | client | listen | Whether block `block`'s block entity ticks on the client (`EntityBlock.getTicker` asked with the client level): set `ticks` when it is yours, and `block_entity_client_tick` runs for it once per client tick. |
| `block_entity_collect_components` | server | replace | The item components the block entity at `pos` holds in its own data when its `id` is yours (`BlockEntity.collectImplicitComponents`): set `components` to them, read from `entity` alone (a pot's meal, a cabinet's contents). |
| `block_entity_data_received` | client | listen | The synchronized `tag` has been merged into this client's block entity. |
| `block_entity_on_load` | both | listen | The mod block entity `entity` has been added to its level, with saved data already applied, before its first tick (`IBlockEntityExtension.onLoad`). |
| `block_entity_render_bounding_box` | client | collect | Where the mod block entity at `pos` draws, for culling (NeoForge's `getRenderBoundingBox`): `bounds` is min x, y, z then max x, y, z, in blocks from the block's lower north-west corner. |
| `block_entity_submit` | client | listen | Draw the mod block entity at `pos`, of block `block` (`mymod:machine`), on this client: once a frame while it is in view (vanilla's `BlockEntityRenderer.submit`: within its view distance, 64 blocks unless `block_entity_view_distance` says otherwise, its chunk faded in, and its bounds, `block_entity_render_bounding_box`, in the frustum), when its `id` is yours. |
| `block_entity_tick` | server | listen | A server tick of the block entity at `pos` in `level`, in vanilla's order among the others (`Level.tickBlockEntities`), when its `id` is yours. |
| `block_entity_ticks` | server | listen | Whether block `block`'s block entity ticks (`EntityBlock.getTicker`): set `ticks` when it is yours. |
| `block_entity_update_tag` | server | replace | What clients get of the block entity at `pos` when its `id` is yours (`BlockEntity.getUpdateTag`): set `tag` to it, read from `entity` alone. |
| `block_entity_view_distance` | client | collect | How far from the camera the mod block entity of block `block` draws, in blocks (vanilla's `BlockEntityRenderer.getViewDistance`): 64 unless a part answers. |
| `menu_button` | server | listen | A player pressed a mod container's button (`clickMenuButton`), on the server. |
| `menu_quick_move` | server | cancel | A player shift-clicked slot `slot` of the menu of the mod block entity at `pos` (`AbstractContainerMenu.quickMoveStack`, the method a Java menu overrides), on the server, before the game's own routing: move the stack where your block wants it. |
| `on_chunk_unloaded` | both | listen | The chunk holding this mod block entity is unloading (`IBlockEntityExtension.onChunkUnloaded`): told once for each of its mod block entities, on each side that holds a copy of it, before the copy is dropped. |
| `pre_remove_side_effects` | server | collect | `BlockEntity.preRemoveSideEffects` of a mod's block entity: an authoritative server is removing `entity` from `pos`, where the block `state` stood, and its updates may have side effects (not under flag 256, `UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS`). |
| `screen` | both | replace | A menu opens on this client for block `block`, a mod's: set `screen` when the block is yours, to lay its screen out (`screen_draw` draws it). |
| `screen_click` | client | replace | A left or right (`right`) press at `at` (GUI pixels from the screen's top left, so outside it too) on block `block`'s screen, a mod's, over no slot: set `button` to a menu button id when it is on one of yours. |
| `screen_draw` | client | listen | Draw block `block`'s screen at `origin` (its top left, GUI pixels), over a plain panel with its slots' frames and under the items and labels, from the menu's data slots `data` (the block entity's `data` on the server): data slots are 16-bit on the wire as in vanilla (a bigger value wraps), and `data` is empty until the server's first values arrive, so read it with `get`. |
| `screen_tooltip` | client | collect | The tooltip at `at` (as in `screen_click`) on block `block`'s screen, a mod's, over no item: push its lines when it is over something of yours. |

### Item parts (`ItemPart`)

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `apply_enchantments` | server | listen | NeoForge's `IItemExtension.applyEnchantments`: the enchanting table applies `enchantments`, the list it chose for the clicked row as (id, level), to `stack`, the item in its input slot (`EnchantmentMenu.clickMenuButton`). |
| `can_equip` | both | listen | NeoForge's `IItemExtension.canEquip`: may `stack` go into the equipment slot `slot` of the living entity `entity`, of type `kind` (`minecraft:player`)? |
| `can_fit_inside_container_items` | both | listen | NeoForge's `IItemExtension.canFitInsideContainerItems`: may `stack` go inside an item that holds items? |
| `can_grindstone_repair` | both | listen | NeoForge's `IItemExtension.canGrindstoneRepair`: does the grindstone repair `stack` as it repairs a damageable item? |
| `can_perform_action` | both | listen | NeoForge's `IItemExtension.canPerformAction`: can `stack` perform the item ability `ability`, a plain, case-sensitive name such as `shield_block`, `shears_harvest` or `shears_carve` (a mod's own abilities also use plain names, such as `my_mod_chop`)? |
| `contained_fluid` | client | listen | The fluid a stack of a mod's own container item holds, as its `neoforge:fluid_container` item model draws it (`FluidUtil.getFirstStackContained`): `fluid` arrives `None` (holds none); set it to the fluid's key, taken once at registration with `r.fluid_key`. |
| `crafting_remainder` | server | listen | NeoForge's `IItemExtension.getCraftingRemainder(ItemStack)`: what `stack` leaves when consumed by crafting, furnace fuel or brewing. |
| `creator_mod_id` | both | collect | `IItemExtension.getCreatorModId`: the mod that made `stack`, which a recipe viewer or tooltip names, instead of the mod that registered its item. |
| `damage_item` | server | cancel | `IItemExtension.damageItem`: the wear `stack` is about to take (`ItemStack.hurtAndBreak`: a tool that mines or hits, armour that takes a hit, a shield, a rod, an elytra, a tool a dispenser uses, a mob's armour), after Unbreaking has had its say, as NeoForge passes it. |
| `dispense_item` | server | replace | `DispenserBlock.getDispenseMethod`: choose a behaviour for any item, including vanilla items, on the authoritative server (not droppers). |
| `empty_contents` | server | replace | `IDispensibleContainerItemExtension.emptyContents`: `stack` (a bucket of a mod fluid, a jar, a canister) is about to be emptied at `pos`, by `player` (the acting player) who clicked `hit` (the block and face), or by a dispenser (`player` and `hit` are `None`). |
| `get_equip_sound` | server | listen | NeoForge's `IItemExtension.getEquipSound`: the sound event `stack` plays when it is equipped in equipment slot `slot` (`head`, `chest`, `legs`, `feet`, `offhand`, `mainhand`, or a mod's own place name such as `curios:ring`). |
| `highlight_tip` | client | listen | NeoForge's `IItemExtension.getHighlightTip`: the name drawn above the hotbar when `stack` is selected. |
| `hurt_enemy` | server | listen | A player's hit landed on the mob `entity` of type `kind` (`minecraft:zombie`), holding `stack`: where `Item.hurtEnemy` is called, before the post-attack enchantments and the weapon's wear. |
| `inventory_tick` | server | listen | A server tick of `stack`, the stack a player carries in `slot`: where `Inventory.tick` calls `Item.inventoryTick`, for every stack, each tick, `/tick freeze` included (it stops the world, not the player). |
| `is_combine_repairable` | both | listen | NeoForge's `IItemExtension.isCombineRepairable`: may two of `stack`'s item be combined in a crafting grid to repair it (`RepairItemRecipe.canCombine`)? |
| `is_not_replaceable_by_pick_action` | server | listen | NeoForge's `IItemExtension.isNotReplaceableByPickAction`: must a pick (the pick-block key) leave `stack`, in hotbar `slot` (0 to 8), where it is? |
| `is_piglin_currency` | both | listen | NeoForge's `IItemExtension.isPiglinCurrency`: is `stack` barter currency to a piglin (`PiglinAi.isBarterCurrency`)? |
| `is_primary_item_for` | server | listen | NeoForge's `IItemExtension.isPrimaryItemFor`: is `stack`'s item primary for `enchantment` (an id such as `minecraft:sharpness`), so that enchantment selection may offer it? |
| `item_bar` | client | listen | The small bar `GuiGraphicsExtractor.itemDecorations` draws under an item, wherever the game shows one: `bar` arrives holding vanilla's value (the damage bar, or a bundle's fullness already applied) and may be replaced (width `0..=13`, colour `rgb`) or set to `None` to hide it. |
| `item_decorations` | client | listen | Append an item's custom decorations after its default bar, cooldown and count. |
| `item_interact_mob` | both | replace | `ItemStack.interactLivingEntity`: after ordinary entity interactions pass, for an alive native Mob and a nonempty stack in `hand`. |
| `makes_piglins_neutral` | server | listen | NeoForge's `IItemExtension.makesPiglinsNeutral`: does `stack`, worn by `wearer`, keep calm piglins from targeting it, as gold armour does (`PiglinAi.isWearingSafeArmor`)? |
| `on_block_placed_by` | server | listen | `BlockItemExtension.onBlockPlacedBy`: this item's block has been placed, its final block state and stack block-entity data applied, and `Block.setPlacedBy` completed (including companion blocks). |
| `on_dropped_by_player` | server | cancel | `IItemExtension.onDroppedByPlayer`: `player` (the acting player's handle) pressed the drop key on `stack`, the whole selected stack before anything leaves it; `all` is true when the whole stack goes, false for one item (`ServerPlayer.drop(boolean)`). |
| `on_entity_item_update` | server | cancel | `IItemExtension.onEntityItemUpdate`: each tick of a dropped item entity holding `stack`, before the entity's own tick (`ItemEntity.tick`: its fluids, movement, pickup delay, merging and lifetime), and after `entity_tick_pre`. |
| `on_entity_swing` | both | cancel | `IItemExtension.onEntitySwing`: the player in place (`entity`, its `Entity.getId` when known) is about to swing `stack` in `hand`: where `LivingEntity.swing` starts, for an attack, a use or a swing the server asked for. |
| `on_item_use_first` | server | replace | `IItemExtension.onItemUseFirst`: the held item's first say on a click on a block, on the server for each player, after `use_item_on_block` (`ItemBeforeBlock`) let it through and before the block's own `useItemOn`. |
| `on_left_click_entity` | server | cancel | `IItemExtension.onLeftClickEntity`: `player` (the acting player's UUID), holding `stack` in the main hand, is about to attack the entity `entity` (`Entity.getId`) of type `kind` (`minecraft:cow`, `minecraft:player`, `minecraft:item_frame`): where `Player.attack` starts, once the global `attack_entity` let the blow through and before any damage, knockback, enchantment effect, `hurt_enemy` or the weapon's wear. |
| `on_use_tick` | both | listen | `Item.onUseTick` of `stack`, used by `entity` with `remaining` ticks left, always above 0: what a mod's own item does each tick it is used (cook, charge, drain). |
| `release_using` | both | listen | `Item.releaseUsing` of `stack`: `entity` let it go with `time_left` ticks of its `use_duration` left, so it was held for `duration - time_left` (a bow-like item shoots by that charge). |
| `should_cause_block_break_reset` | client | listen | NeoForge's `IItemExtension.shouldCauseBlockBreakReset`: does a break restart because the main hand changed? |
| `should_cause_reequip_animation` | client | listen | NeoForge's `IItemExtension.shouldCauseReequipAnimation`: does the first-person hand lower and raise again to show `stack`, the stack the hand holds now? |
| `tooltip` | client | collect | `Item.appendHoverText`: an item's own lines in its tooltip, after its name and before its components' (enchantments, lore, attributes). |
| `tooltip_component` | client | replace | Client-side image component for this stack's tooltip. |
| `tooltip_component_draw` | client | listen | Render the measured component at origin, inside the tooltip background. |
| `use_animation` | client | listen | `Item.getUseAnimation` of `stack` in use, for first- and third-person rendering. |
| `use_duration` | both | listen | `Item.getUseDuration` of `stack`, in ticks: `duration` arrives with vanilla's (a consumable's time, 72000 for a shield, a spear, a bow, 0 for an item with no use over time). |
| `use_item` | both | replace | A player uses `stack` (right click) with `self.hand`: in the air, or on a block once `native_use_on` let it through, where `Item.use` starts. |
| `use_on` | both | replace | A player uses `stack` (right click) with `self.hand` on the block at `pos`, `face` the face clicked (`[0, 1, 0]` is the top), once the block itself let the click through (a chest still opens): where `ItemStack.useOn` starts. |
| `xp_repair_ratio` | server | listen | `IItemExtension.getXpRepairRatio`: the durability points Mending restores to `stack` per point of experience, where `ExperienceOrb.repairPlayerItems` repairs the damaged Mending item it picked, once per item it repairs. |

### Entity parts (`EntityPart`)

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `general_entity_tick` | server | listen | The authoritative tick of one general entity row of type `kind` (`mymod:robot`), with ID `entity`, in the server level's entity phase: `ServerLevel.tickNonPassenger`'s `entity.tick()` for a row the type registry owns (arismoko/horizon#792 stage 4). |

### Menu parts (`MenuPart`)

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `menu_slot_rules` | both | listen | Slot placement and stack limits for registered vanilla and mod menus; mounts use `horizon:mount_inventory`, while player inventory and the creative picker have no registered type. |
| `menu_still_valid` | server | listen | Whether a player's block menu of a mod menu type stays open (`AbstractContainerMenu.stillValid`). |
| `menu_transfer` | server | replace | A recipe viewer's "+" on a mod menu (JEI's `IRecipeTransferHandler`): move one set of `recipe`'s ingredients (as many as the items allow with `max`) from `player`'s inventory into the open menu of a mod menu type, or with `dry_run` only say what that would do. |

### Fluid parts (`FluidPart`)

| Hook | Side | Contract | What it does |
| --- | --- | --- | --- |
| `fluid_rules` | server | listen | How a mod's world fluid flows at `pos`, starting from what it declared (NeoForge's `BaseFlowingFluid` and `FluidType` getters: `getTickDelay`, `getDropOff`, `getSlopeFindDistance`, `canConvertToSource`). |

## Host methods

`Host` is what a hook may ask of the game or do in it. `ServerHost` methods exist on the
server only, `ClientHost` methods on the client only, `Host` methods on both sides.

### `advancements`

| Method | On | What it does |
| --- | --- | --- |
| `advancement` | `ServerHost` | `ServerAdvancementManager.get(id)`: the loaded advancement `id`, from the set the last data-pack load or `/reload` built (loaded on first use). |

### `block_entities`

| Method | On | What it does |
| --- | --- | --- |
| `block_entity` | `Host` | Find a block entity in a loaded chunk of an open level, or on a client in the level it shows. |
| `block_entity_identity` | `Host` | The handle `BlockEntitiesHost::block_entity` gives, and also for a block entity out of the world for its own tick, whose identity and attachments stay in place meanwhile: what its Part state (`BlockEntityPartState`) checks a handle against. |
| `block_entity_sync` | `ServerHost` | Sends clients the block at `pos` and its block entity's update tag (`block_entity_update_tag`) again, as `Level.sendBlockUpdated` does: with the level's other block changes, to the players who see its chunk, once however often it is called before then. |

### `block_states`

| Method | On | What it does |
| --- | --- | --- |
| `lit_block_name` | `Host` | Read a loaded, light-emitting block. |
| `fluid_key` | `Host` | Resolve a registered fluid identifier. |
| `block_state_id` | `Host` | The state `name` names, read as `/setblock` reads a block (`BlockStateParser.parseForBlock`): `minecraft:oak_stairs[facing=east]`, `stone` for `minecraft:stone`, the properties left out at their default. |
| `block_state_string` | `Host` | The state `state` as `/setblock` takes it back (`BlockStateParser.serialize`): its block, then each property in name order, `minecraft:oak_stairs[facing=east,half=bottom,...]`, or the block alone for one with no properties. |
| `block_state_default` | `Host` | The default state of the resolved `block` key: `Block.defaultBlockState`, the one a block takes when nothing else says. |
| `item_crafting_remainder` | `Host` | `Item.getCraftingRemainder`: Horizon currently stores the remainder's item id, so the result is a single plain stack. |
| `block_item` | `Host` | `Block.asItem`: the block's item identity, including air for a block with no item; None if either resolved identity is unavailable. |
| `item_block` | `Host` | The block `item` places (`BlockItem.getBlock`, as `Block.byItem` reads it): `minecraft:wheat_seeds` places `minecraft:wheat`, and a standing/wall item places its standing block, not the wall variant. |
| `set_block_property` | `ServerHost` | Sets property `key` of the block at `pos` to `value`, as a furnace's tick sets its `lit` (`Level.setBlock` with `UPDATE_ALL`: neighbours and clients are told); true when the block changed, false when the block has no such property or value, or the property already has `value` (as `Level.setBlock` for an unchanged state). |
| `set_block_property_with` | `ServerHost` | `native_set_block_property` with `Level.setBlock`'s `flags`, as a brewing stand's tick sets its bottles with `flags::UPDATE_CLIENTS`. |

### `boss_bars`

| Method | On | What it does |
| --- | --- | --- |
| `boss_bar` | `ServerHost` | An existing custom boss bar by id (`minecraft:` when no namespace), or None. |
| `boss_bars` | `ServerHost` | Every custom boss bar, in the order `/bossbar list` shows them. |
| `create_boss_bar` | `ServerHost` | `/bossbar add`: a new bar, visible, white, `progress`, at 0 of 100, with nobody added. |

### `client_input`

| Method | On | What it does |
| --- | --- | --- |
| `claim_items` | `ClientHost` | Right-clicks with these items (at most 64 ids) are mod `mod_id`'s; a later call replaces the list. |
| `poll` | `ClientHost` | Mod `mod_id`'s events since its last poll (at most 256 wait). |
| `cursor` | `ClientHost` | Where the mouse is, in GUI pixels. |
| `mod_key` | `ClientHost` | The saved key and modifiers for an owner-bound registration, or its declared default. |
| `is_down` | `ClientHost` | Whether mod `mod_id`'s registered mapping `id` is held (`KeyMapping.isDown`): its key or mouse button is down and its conflict context and modifiers are active. |
| `consume_click` | `ClientHost` | Takes one click off mod `mod_id`'s registered mapping `id` (`KeyMapping.consumeClick`): true when it had one. |
| `playing` | `ClientHost` | Playing, with nothing over the world: keys and zoom apply. |
| `mapping` | `ClientHost` | A vanilla mapping (`key.forward`, `key.jump`, `key.attack`…): whether it is held (`KeyMapping.isDown`), and its key's name as Controls shows it. |
| `set_mapping_down` | `ClientHost` | Holds the vanilla gameplay mapping `key` down or lets it go (`KeyMapping.setDown`): the held state its key gives it, without the click a press adds. |
| `click_mapping` | `ClientHost` | Adds one click to the vanilla gameplay mapping `key` (`KeyMapping.click`), as a press of its key does: the attack, use or pick a click starts on a subsequent client tick, through the checks of `startAttack` and `startUseItem` (the miss cooldown, busy hands, the item, the target under the crosshair) and the packets the player's own click sends; the hotbar slot, drop, inventory or hand swap of the inventory keys. |
| `millis` | `ClientHost` | Milliseconds on the game's own clock (`Util.getMillis`), counting from the game's start: what a text field's cursor blinks by and what tells a double click from two clicks. |
| `modifiers` | `ClientHost` | The current window modifier keys in the same GLFW encoding as input hooks. |
| `clipboard_text` | `ClientHost` | The clipboard's text (`KeyboardHandler.getClipboard`), empty when it holds none, while the game handles a key, mouse or typed-text event: a mod's key or screen hook pasting into its own field. |
| `set_clipboard_text` | `ClientHost` | Puts `text` on the clipboard (`KeyboardHandler.setClipboard`) on the same terms as `native_clipboard_text` reads it. |

### `client_narrator`

| Method | On | What it does |
| --- | --- | --- |
| `narrate` | `ClientHost` | Has the narrator say `text` as a system message, through the same `GameNarrator` the game's own narration uses: `Narration::Now` is `saySystemNow` (cuts off what is being said), `Narration::Queued` is `saySystemQueued`. |

### `client_options`

| Method | On | What it does |
| --- | --- | --- |
| `input_options` | `ClientHost` | A snapshot of the current input preferences. |
| `graphics_options` | `ClientHost` | A snapshot of the current graphics preferences. |
| `gui_options` | `ClientHost` | A snapshot of the current GUI and accessibility preferences. |
| `gui_scale` | `ClientHost` | The GUI scale in effect (`Window.getGuiScale`), at least 1; none on a server or when no window exists. |
| `chat_options` | `ClientHost` | A snapshot of the current chat preferences. |
| `audio_options` | `ClientHost` | A snapshot of the current audio preferences. |

### `client_render`

| Method | On | What it does |
| --- | --- | --- |
| `instance_group_write` | `ClientHost` | Atomically overwrite retained model copies; use `InstanceGroupRef::write`. |
| `instance_group_set_len` | `ClientHost` | Set the drawn length; use `InstanceGroupRef::set_len`. |
| `instance_group_set_transform` | `ClientHost` | Replace the group pose; use `InstanceGroupRef::set_group_transform`. |
| `instance_group_destroy` | `ClientHost` | Retire retained model copies; use `InstanceGroupRef::destroy`. |
| `refresh_render_visibility` | `ClientHost` | Asks `block_render_visibility` again for every state and, when the answers differ from those in use, remeshes every loaded section with them. |
| `refresh_biome_colors` | `ClientHost` | Asks `biome_colors` again for every biome and, when the colours differ from those in use, rebuilds every loaded section and the far terrain with them (`LevelRenderer.allChanged`, as changing Biome Blend does). |
| `mod_image` | `ClientHost` | A mod's own RGBA image, uploaded into the HUD atlas slot `MOD_IMAGE_SPRITE` and drawn with `native_sprite`: its own source rectangle crops and stretches it. |
| `block_tint_color` | `ClientHost` | For a client mod drawing a block's faces itself (a copycat drawing its material's, from `block_sprites`): the 0xRRGGBB colour a face of `state` with tint index `tint_index` (`FaceSprite::tint_index`) is multiplied by at `pos`, as the chunk mesher tints it there (`BlockColors.getColor`, biome blend included: grass, foliage and water follow the biome around `pos`). |
| `block_sprites` | `ClientHost` | Published asset/data read: available in Setup, Reloading and World. |
| `texture_sprites` | `ClientHost` | Published asset/data read: available in Setup, Reloading and World. |
| `gui_sprite` | `ClientHost` | Published asset/data read: available in Setup, Reloading and World. |
| `block_sprites_version` | `ClientHost` | Published asset/data read: available in Setup, Reloading and World. |

### `client_text`

| Method | On | What it does |
| --- | --- | --- |
| `text_string` | `ClientHost` | `Component.getString()`: resolve a component to its plain characters, including current-language translations, arguments, key names and siblings. |
| `text_visual_order` | `ClientHost` | `Component.getVisualOrderText()`: current-language, styled characters in visual order, including bidi shaping and mirroring. |
| `text_width` | `ClientHost` | `Font.width(FormattedText)`: the width of a component in unscaled GUI pixels, using the current language, styles and resource-pack fonts. |
| `text_split` | `ClientHost` | `Font.split(FormattedText, width)`: wrap a component at `width` unscaled GUI pixels. |

### `client_view`

| Method | On | What it does |
| --- | --- | --- |
| `block_collision` | `Host` | Collision boxes in block-local coordinates with an empty collision context. |
| `hud_mode` | `ClientHost` | Read effective HUD visibility independently of first-person hands. |
| `native_corner_shown` | `ClientHost` | Whether F3's debug screen leaves `corner` free this frame for a box `render_gui_post` places: false for a corner it takes (`DebugScreenOverlay`), both top corners while it is open (its left and right columns) and a bottom corner while one of its charts is drawn there. |
| `set_hud_mode` | `ClientHost` | Select HUD and hand visibility, including after vanilla F1 input handling. |
| `camera_type` | `ClientHost` | The window's camera type (`Options.getCameraType`), including its latest pending request. |
| `set_camera_type` | `ClientHost` | Request the window's camera type without cycling, also from shared-host callbacks. |
| `is_paused` | `ClientHost` | The applied client pause state, not whether a pause screen is open. |
| `clock` | `ClientHost` | The clock of the world this window shows, for client mods: its game time and time of day as the server last sent them (`ClientboundSetTimePacket`), advanced each tick between packets; none on the title screen. |
| `raycast` | `ClientHost` | The first block along the view within `distance` (at most 1024), past the loaded chunks over the distant terrain. |
| `view_hit` | `ClientHost` | The first block along the view within `RAYCAST_MAX`, and its distance. |
| `camera` | `ClientHost` | The most recently extracted world frame's camera, if one exists. |
| `project` | `ClientHost` | World point `at` projected on this frame's screen (`GameRenderer.projectPointToScreen`): normalised device x and y, -1 to 1 across the screen (x to the right, y up), and depth z (reversed: nearer is larger). |
| `local_item_use` | `ClientHost` | The local player's item use as the client sees it: the hand, ticks used and ticks left (`isUsingItem`, `getUsedItemHand`, `getTicksUsingItem`, `getUseItemRemainingTicks`). |
| `set_local_rotation` | `ClientHost` | Turns the local player's view to `yaw` and `pitch` (degrees), as `setYRot` and `setXRot` store them: yaw as given (not wrapped; the server wraps it), pitch `% 360` then clamped to -90..90. |
| `camera_entity` | `ClientHost` | The entity the view follows (`Minecraft.getCameraEntity`): the entity a server attached the camera to, else the local player. |
| `client_surface_heights` | `ClientHost` | Each column's height by `heightmap` in the world this window shows (`LevelReader.getHeight(type, x, z)` on a client level): one above the highest block that counts on that map, from the blocks the server has sent, none for a column whose chunk this client does not hold. |
| `client_far_surface` | `ClientHost` | For a client mod drawing a map past the loaded chunks: each column's surface as this window's far terrain (the LOD Distance setting) holds it, whether or not the chunk is loaded. |
| `set_camera_entity` | `ClientHost` | Points the view at `entity` (`Minecraft.setCameraEntity`), as a server's camera packet does; the local player's own ref puts it back. |
| `crosshair_target` | `ClientHost` | The ordinary block-or-entity pick (Minecraft.hitResult), using the game's targeting ray, reach and occlusion. |
| `targeting` | `ClientHost` | Current targeting controls. |
| `set_targeting` | `ClientHost` | Set the window's targeting controls for subsequent picks. |

### `config`

| Method | On | What it does |
| --- | --- | --- |
| `dist` | `Host` | Physical process, not the logical side of this hook. |
| `game_version` | `Host` | Exact Minecraft version supported by this process. |
| `server_data_version` | `Host` | The engine's `DataVersion` written into level data and chunks. |
| `horizon_version` | `Host` | Horizon version printed in crash reports. |
| `on` | `Host` | Whether the player left mod `mod_id` on in Options > Mods. |
| `setting` | `Host` | A mod's setting `key` (one file per key in `native_data_dir`). |
| `set_setting` | `Host` | Keeps a mod's registered setting `key`. |
| `config_files` | `Host` | The calling mod's own files in the game's `config/` folder, beside `options.txt` (beside `server.properties` on a dedicated server): its folder `config/<mod id>/` and the files beside it named `<mod id>.<ext>` or `<mod id>-<rest>`, as Fabric and NeoForge mods keep theirs. |
| `data_files` | `Host` | `horizon-mod-data/<mod id>/` in the game folder, never scanned as settings or config. |
| `dimension` | `Host` | The level in place (`minecraft:overworld`), for a hook that isn't told which it runs in. |
| `server_has_time` | `Host` | Whether the server's current tick still has time before the next one is due (`MinecraftServer.haveTime`, NeoForge's `ServerTickEvent.hasTime` and `LevelTickEvent.hasTime`): a mod doing optional work in `server_tick` or `level_tick_pre`/`level_tick_post` skips it when this is false. |
| `instance_dir` | `Host` | The instance (game) folder, which its players share: `saves/`, `resourcepacks/`, `shaderpacks/` and `horizon-mods/` are in it, and the title's screens create worlds and read packs there. |
| `game_tests_enabled` | `ServerHost` | Startup opt-in for test-only content. |
| `tick_stats` | `ServerHost` | The server's tick rate (20 unless `/tick rate` changed it) and the milliseconds its ticks took on average lately, as `/tick query` reports them. |
| `tick_times` | `ServerHost` | The last 100 server ticks' durations in nanoseconds, as `MinecraftServer.getTickTimesNanos` lays them out: the tick numbered `n` at index `n % 100`, 0 where none was recorded yet. |
| `mob_cost_sampling` | `ServerHost` | Times every mob's own tick (its riders inside it) from now on, or no longer, summed by level and mob type for `take_mob_costs`, as a profiler samples them: while on, two clock reads and a map update a mob; off, one thread-local read. |
| `take_mob_costs` | `ServerHost` | The mob tick costs sampled since the last call (`mob_cost_sampling`), costliest first; clears them. |
| `count_packets` | `ServerHost` | Counts every player's play packets by id and direction from now on, or no longer, for `player_take_packet_counts`. |
| `server_view_distance` | `ServerHost` | The server's view distance in chunks (`PlayerList.getViewDistance`, `view-distance`): how far players are sent chunks at most. |
| `server_simulation_distance` | `ServerHost` | The server's simulation distance in chunks (`PlayerList.getSimulationDistance`, `simulation-distance`): how far around each player chunks tick. |
| `server_set_view_distance` | `ServerHost` | Sets the server's view distance live, as `PlayerList.setViewDistance` (and the management protocol's `serversettings/view_distance/set`) do: every player is sent `ClientboundSetChunkCacheRadiusPacket` and their chunks follow, each up to what their client asked for. |
| `server_set_simulation_distance` | `ServerHost` | Sets the server's simulation distance live, as `PlayerList.setSimulationDistance` does: every player is sent `ClientboundSetSimulationDistancePacket`, and the chunks around the players tick to the new distance from the next tick. |
| `set_tick_rate` | `ServerHost` | Sets the server's tick rate, as `/tick rate` does (`ServerTickRateManager.setTickRate`): the game ticks `rate` times a second from the next tick on, an autosave wait already running is shortened when the new rate asks for less, and `server_tick_rate_changed` fires. |
| `world_dir` | `ServerHost` | The open world's folder (`world/`, beside its `level.dat`): a server mod keeps its data in a folder of its own there, so the world's backups carry it. |
| `world_data_files` | `ServerHost` | `horizon-mod-data/<mod id>/`, copied, backed up and deleted with that save. |
| `game_rules` | `ServerHost` | Every current rule in the one server-wide store, by namespaced id. |
| `game_rule` | `ServerHost` | Read the server-wide rule from the first server hook onward. |
| `reload_resources` | `ServerHost` | The retained server reload listeners of the last successful reload. |
| `translate` | `ClientHost` | The selected language's text for a translation key (`Component.translatable`'s lookup), `None` when no language layer defines it. |
| `setting_bool` | `ClientHost` | The calling mod's toggle `key` (`Registrar::register_setting`): the player's choice, else its default. |
| `setting_choice` | `ClientHost` | The index of the option the player picked for the calling mod's choice `key`, else its default. |
| `setting_f64` | `ClientHost` | The calling mod's range `key`: the player's value, else its default; a kept value outside the range reads as the default. |
| `setting_color` | `ClientHost` | The calling mod's colour `key`, `0xRRGGBB`: the player's choice, else its default. |
| `official_extra_settings` | `ClientHost` | The calling mod's old settings from `horizon-extras.txt`, when it is the official mod that took over a built-in extra (`fps_overlay`, `keystrokes`, `minimap`, `zoom`): each of that extra's keys the file has a line for, with the value the extra used, as it wrote it (`("fpsCorner", "top_left")`). |

### `displays`

| Method | On | What it does |
| --- | --- | --- |
| `display` | `ServerHost` | Narrows an entity to a display, or None for another kind, a stale handle or a client. |

### `draw`

| Method | On | What it does |
| --- | --- | --- |
| `fluid_name` | `ClientHost` | Fluid `fluid`'s name as the lang names it (`fluid_type.<mod>.<name>`, vanilla's by its block), else its path in words. |
| `bake_layer` | `ClientHost` | Entity model layer `layer` baked into its `ModelPart` tree, each part at its pose, as `EntityModelSet.bakeLayer` bakes it: one a native mod registered (`register_layer_definitions`), or vanilla's, by the name `entity-models.json` gives it (`minecraft:zombie#main`, `minecraft:player#main`). |
| `size` | `Host` | The GUI's size in GUI pixels, width then height. |
| `item_model` | `Host` | Item `item`'s icon drawn with model `model` (`minecraft:paper` as `mymod:wheel_icon`, an `item_model` of a content mod's), as `native_item` draws one; false for an unknown item. |
| `quad` | `Host` | A quad of corners `points` (in order round it) in GUI pixels, `argb`: a wedge, a slanted bar, what `native_fill`'s rectangles can't draw. |
| `fill` | `Host` | A rectangle in GUI pixels, `argb` (0x78000000 is the extras' box). |
| `fill_gradient` | `Host` | `GuiGraphics.fillGradient`: rectangle `rect` (`[x, y, w, h]` in GUI pixels, as `fill`(Self::fill) takes it) blending from ARGB `top` at `y` to `bottom` at `y + h`. |
| `sprite` | `Host` | Part `src` (`[x, y, w, h]` in the sprite's own pixels) of sprite `name` (vanilla's `gui/...`, or a content mod's GUI texture `mymod:gui/...`), drawn stretched into `dst` (`[x, y, w, h]` in GUI pixels), every texel multiplied by `tint` (`0xAARRGGBB`, a fluid's colour) when there is one. |
| `item` | `Host` | Item `item`'s icon (`mymod:machine`) with its top left at `at`, `size` GUI pixels square (16 in a slot); false for an unknown item. |
| `text` | `Host` | A line of text at `(x, y)` in GUI pixels, in `argb` as vanilla's `GuiGraphics` takes it (`0xFFFFFFFF` is opaque white). |
| `item_stack` | `Host` | Stack drawing does not enforce resident count or damage maxima. |
| `gui_paint` | `Host` | Paint GUI extraction calls through this window's atlas, including player faces. |
| `gui_nine_slice` | `Host` | Vanilla's nine-slice widget rendering. |
| `gui_tooltip` | `Host` | A vanilla tooltip at the cursor, in GUI pixels. |
| `text_width` | `Host` | The width of `text` in GUI pixels, as `native_text` draws it on `draw` (`Font.width`). |
| `fluid` | `Host` | Fluid `fluid` (`minecraft:water`, `mymod:oil`) drawn as a square with its top left at `at`, `size` GUI pixels wide (16 in a slot): the sprite its look names (`data/<mod>/horizon/fluid/<name>.json`) tinted with its colour, else a flat square of the colour, which vanilla water and lava have. |
| `draw_scene` | `Host` | Draws `blocks`, a scene of block states at positions of its own (not the world's), into the box `rect` (x0, y0, x1, y1, GUI pixels) of a screen's `batch`, seen as `view` says: Create's Ponder scenes. |
| `push_clip` | `Host` | `GuiGraphics.enableScissor`: what this hook call draws from now on, until the matching `pop_clip`(Self::pop_clip), is cut to `rect` (`[x, y, w, h]` in GUI pixels, as `fill`(Self::fill) takes it), the visible part of a sprite, glyph or item kept. |
| `pop_clip` | `Host` | `GuiGraphics.disableScissor`: closes the clip `push_clip`(Self::push_clip) opened last, so what is drawn next is cut only by the clips around it. |
| `entity` | `Host` | Draws living entity `entity` standing in `view.rect`, `view.size` GUI pixels a block, turned as `view.look` says: vanilla's `InventoryScreen.renderEntityInInventoryFollowsMouse` (`GuiEntityLook::At` the cursor) and `renderEntityInInventory` (`GuiEntityLook::Turned`), as a mob page, an entity icon or a data model's mob shows it. |
| `draw_menu_background` | `Host` | `Screen.extractMenuBackground`: draws the tiled menu background (32x32) across `rect` (`[x, y, w, h]` in GUI pixels). |
| `tiled_sprite` | `Host` | Repeats sprite `sprite` across `rect` (`[x, y, w, h]` in GUI pixels) with each tile sized `size` GUI pixels square, tinted by `argb`. |
| `gui_draw_item_with` | `Host` | Draw an item stack with custom count/decoration text (or None) at `at` in GUI units. |
| `widget_button` | `Host` | A button as the game's screens draw one (`AbstractButton`): `label` centred on the `widget/button` sprite (`button_highlighted` when `highlighted`, `button_disabled` and grey when not `active`), scrolling when it doesn't fit, and recorded for the narrator and a controller's focus. |
| `widget_text_field` | `Host` | A text field as the game's screens draw one (`EditBox`): its text from the first character `field.scroll` shows, scrolled on to keep the cursor in view, the selection highlighted, the caret, the suggestion, the `widget/text_field` sprite when bordered, and a record for the narrator and a controller's focus. |
| `push` | `Host` | `PoseStack.pushPose`: `pop` comes back to the pose as it is now. |
| `pop` | `Host` | `PoseStack.popPose`; the block's own pose is never popped. |
| `translate` | `Host` | Moves what is drawn next by `by`, in blocks. |
| `rotate_about` | `Host` | Turns what is drawn next by `degrees` about `axis` (counterclockwise seen from the axis' positive end, as `Axis.rotationDegrees`) through `pivot`, in blocks from the pose's origin: `[0.5; 3]` is the block's centre. |
| `scale` | `Host` | Scales what is drawn next by `by` about the pose's origin. |
| `keep` | `Host` | Keeps what this call drew, with neither `block_entity_submit` nor `block_entity_render_bounding_box` called until its state, block entity, light or resource packs change. |
| `partial` | `Host` | The frame's partial tick, 0 to 1 (`BlockEntityRenderState`). |
| `game_time` | `Host` | The level's game time, for animations. |
| `animation_time` | `Host` | The frame on this client's animation clock, in ticks with the partial tick: what turns `spin` parts. |
| `light` | `Host` | The light the helpers draw with: the block's own (`LevelRenderer.getLightCoords`), 16 times sky light in bits 0..8 and 16 times block light in bits 8..16. |
| `set_light` | `Host` | Lights what is drawn next with `light` (as `native_draw_light` reads it). |
| `quad` | `Host` | Draws a quad with `corners` in blocks under `draw`'s pose, in a model face's order (top left, bottom left, bottom right, top right as seen from its front), textured with the `uv` rectangle (u0, v0, u1, v1 in texels of 16 per side) of `sprite` (`mymod:entity/screen`, `mymod:block/casing`, `block/stone`) times the 0xRRGGBB `tint` (0xFFFFFF for none). |
| `cube` | `Host` | Draws a box from `from` to `to` (texels of 16 per block, as a block model's element) under `draw`'s pose, with the faces given, by `Direction` (down, up, north, south, west, east), baked as a model element's are (`FaceBakery`), times the 0xRRGGBB `tint`. |
| `block` | `Host` | Draws block `block`'s model (`mymod:cog`, any block with a model) in the state with `properties` (`[("axis", "x")]`, the rest default) under `draw`'s pose, as vanilla's `submitBlockModel` draws a block in an entity's place: a block's baked model, with its default tints. |
| `item` | `Host` | Draws `stack` under `draw`'s pose as an item is drawn in the world (`ItemModelResolver.updateForTopItem` and `ItemStackRenderState.submit`), centred on the pose's origin, in its model's `display` transform: `Fixed` as a campfire's or an item frame's, `Ground` as a dropped item's. |
| `spin` | `Host` | What is drawn next turns by itself: `degrees_per_tick` degrees each tick of the animation clock (`animation_time`; a shaft at `rpm` turns `rpm * 0.3`), about `axis` through `pivot`, both in blocks from the block's corner whatever the pose, counterclockwise seen from the axis' positive end. |
| `still` | `Host` | What is drawn next stands still again, after `spin`. |
| `model` | `Host` | Draws the resolved standalone model under `draw`'s pose: unturned, from the block's lower north-west corner, lit by `draw.light`. |
| `terrain_model` | `Host` | Draws the resolved standalone model under `draw`'s pose, as `Self::model` places it, with the terrain's own materials: the block sprites (animated, mipmapped as the options set), back faces culled, colour and depth written as opaque terrain, lit by `draw.light` raised by each quad's own emission. |
| `text` | `Host` | Draws `text` under `draw`'s pose as vanilla's `Font.drawInBatch` draws a component in the world (a sign's line, a text display's): in the pose's x-y plane, one unit a font pixel, y down, its left end at `at`, facing +z. |
| `text_width` | `Host` | The width of `text` in font pixels, as `text` draws it, styles included (`Font.width`), to lay a line out before drawing it. |
| `model_part` | `Host` | Draws model part `part` and the parts under it under `draw`'s pose, as vanilla's `ModelPart.render` does: each part moved to its place (in texels, 16 to a block), turned about z, then y, then x, and scaled, its faces textured from `sprite` (`mymod:entity/golem`, a content mod's `assets/mymod/textures/entity/golem.png`, or vanilla's `entity/zombie/zombie`) times the 0xRRGGBB `tint`. |
| `fluid_box` | `Host` | Draws `stack` as a still fluid filling `bounds` (`[x0, y0, z0, x1, y1, z1]` in blocks under `draw`'s pose), as a tank's contents: `minecraft:water` or `minecraft:lava` with no components. |
| `quad` | `Host` | A quad of corners `corners` in solid `argb`. |
| `sprite` | `Host` | A quad of corners `corners` showing sprite `name` of the game's 2D atlas (`block/fire_1`, `misc/underwater`), with `uv` the texture coordinates of each corner in sprite units (0 to 1 is the sprite once; beyond that it repeats), every texel multiplied by `argb`. |
| `draw_instance_group` | `Host` | Draw a retained group once per view, with per-view culling and the batch route rules. |
| `lines` | `Host` | Submit unlit ARGB world lines atomically. |
| `quads` | `Host` | Submit unlit, alpha-blended world quads atomically, using the white atlas texel. |
| `pipeline_quads` | `Host` | Submit world quads drawn by the mod pipeline `pipeline`, atomically: its shader gets each quad's texture coordinate and colour, and its blend and depth modes apply. |
| `usage` | `Host` | Queued counts and fixed capacities, shared by both depth modes and all listeners. |
| `draw_model_instances` | `Host` | Draw every copy in `instances` of the resolved `model`. |

### `effects`

| Method | On | What it does |
| --- | --- | --- |
| `muffle_sounds` | `ClientHost` | Muffles the game's sounds for a while, as ears ringing after a close blast hear them: `amount` (0 to 1) of every sound's volume is taken away and `lowpass` (0 to 1) dulls it, from untouched to a 300 Hz low-pass. |
| `flame_particle` | `ClientHost` | A small local vanilla particle; caller should rate-limit emission. |
| `play_local_sound` | `ClientHost` | Plays `sound` for this player only (`SoundPlay`'s event: `minecraft:ui.button.click`, or a mod's own from its `assets/<id>/sounds.json`): nothing is sent to the server, and other players don't hear it. |
| `particle_count` | `ClientHost` | The particles alive in the client's level (`ParticleEngine.countParticles`): 0 once a clear is asked, and on a server. |
| `clear_particles` | `ClientHost` | `ParticleEngine.clearParticles`: every particle and emitter goes before the next frame is drawn or the next particle tick runs, whichever comes first. |
| `add_particle` | `ClientHost` | ClientLevel.addParticle: submits one local particle through the ordinary provider, camera-distance, setting and population limits. |
| `create_fireworks` | `ClientHost` | `ClientLevel.createFireworks`: draws a firework's explosions in this client's world as a bursting rocket draws them (`FireworkParticles.Starter`): each explosion's shape, colours, fade colours, trail and twinkle, one every other tick. |
| `sound_events` | `ClientHost` | Every sound event the client's resources have (`SoundManager.getAvailableSounds`). |
| `audio_devices` | `ClientHost` | The output devices sound can play on (`SoundEngine.getAvailableSoundDevices`). |
| `audio_device` | `ClientHost` | The output device the options ask for (`Options.soundDevice`). |

### `entities`

| Method | On | What it does |
| --- | --- | --- |
| `entity` | `Host` | The entity `id` (the id events carry) as a handle for the `entity_*` methods: None with no world open, or for no such entity in the level in place or among the server's players. |
| `entity_residency` | `Host` | Where the entity `entity` names is now, as its Part state (`EntityPartState`) checks it: resident in the level this host is in, out of its lists for its own tick, resident in another level, or gone (removed or its removal pending, unloaded, moved under a new id, or a handle of another world open). |
| `level` | `Host` | The level `dimension` (`minecraft:the_nether`) as a handle: on a client, the level it shows, whatever the server calls it, and each level the server opened for viewing (`horizon:levels`) while that view is open. |
| `entity_by_uuid` | `ServerHost` | Finds a loaded entity by UUID in the server level in place for this hook. |
| `summon` | `ServerHost` | Spawns an entity of `kind` (`minecraft:item_display`) at `pos` in the level in place, `dimension`, from `nbt` (its compound, as `/summon <kind> <pos> <nbt>` takes it; passengers included): its id. |
| `set_experience_orb` | `ServerHost` | Writes the experience orb `id`'s `value` (the points a pickup gives), `count` (orbs merged into it, each worth `value`) and `age` (ticks lived; an orb despawns at 6000); a `None` is left as it is. |
| `experience_orbs` | `ServerHost` | The experience orbs of the server level `dimension` whose box meets the box `min`..`max`, in the level's order: what a mod reads before it cancels an orb in `entity_join_level`, to give its points with `PlayerRef::give_experience_points`. |
| `spawn_falling_block` | `ServerHost` | Throws a falling block of `state` from `pos` (its bottom centre) at `velocity` (blocks a tick) in the level in place, `dimension`: it falls, lands and places itself as sand does, or drops as an item where it can't. |

### `fluids`

| Method | On | What it does |
| --- | --- | --- |
| `cauldron_content_for_block` | `Host` | Contents registered for `block`, or none if absent or no table has been built. |
| `cauldron_content_for_fluid` | `Host` | Contents registered for `fluid`, or none if absent or no table has been built. |
| `cauldron_level` | `Host` | Current level of `state`, or none without registered contents or a built table. |

### `item_entities`

| Method | On | What it does |
| --- | --- | --- |
| `item_entity` | `Host` | Narrows an entity to a dropped item, or None for another kind, a stale handle, a client, or an item temporarily taken out for ticking. |

### `item_particles`

| Method | On | What it does |
| --- | --- | --- |
| `item_particle_options` | `ClientHost` | Resolve vanilla's `ItemParticleOption` for this stack using the ground display context and seed 0, exactly like the game's eating particles. |

### `items`

| Method | On | What it does |
| --- | --- | --- |
| `max_stack_size` | `Host` | The most of the item `item` one slot holds (`Item.getDefaultMaxStackSize`), or `None` for an unknown item: for a mod's container that fills or merges stacks itself. |
| `max_damage` | `Host` | Default durability of the resolved item, or none if its metadata is unavailable. |
| `item_key` | `Host` | Resolve an item once at load, including vanilla items, to its process-local identity. |
| `item_default_component` | `Host` | Item `item`'s default value of component `component`, in its prototype (`Item.components().get`): `minecraft:max_damage` on an iron sword is `250`, data packs' and native mods' changes included. |
| `item_burn_time` | `Host` | Item `item`'s cooking-fuel burn time in ticks (`FuelValues.burnDuration` of its `cooking_fuel` component, the data pack's context int provider resolved), None where it is no fuel. |
| `item_compost_chance` | `Host` | Item `item`'s chance to raise a composter's level (its `compostable` component's layers provider resolved), None where it is not compostable. |
| `enchantments_compatible` | `Host` | Whether enchantments `a` and `b` may share an item (`Enchantment.areCompatible`), by the world's enchantment definitions. |
| `entity_type_in_tag` | `Host` | `EntityType.is(TagKey)`: whether `entity_type` (`minecraft:player`) is in the entity type tag `tag` (`minecraft:cannot_be_pushed_onto_boats`). |
| `item_from_snbt` | `Host` | The stack an SNBT item compound stands for, read as `ItemStack.CODEC` reads a saved item: `{id:"minecraft:diamond_sword",count:1,components:{"minecraft:damage":5}}` (`count` 1 when missing, `stone` is `minecraft:stone`). |
| `dispense_default` | `ServerHost` | Drop one item as DefaultDispenseItemBehavior does. |
| `scratch_stack` | `ServerHost` | Holds `stack` outside the world, in the open transaction `tx` (`ItemAccess.forStack`): the `ItemPlace::Scratch` to run item capabilities against. |
| `scratch_take` | `ServerHost` | Takes the stack out of scratch place `place`, in `tx`, ending the place: none where it is empty, no longer held, or not a scratch place. |
| `scratch_slots` | `ServerHost` | Holds `stack` outside the world in a two-slot copy whose item may change, in the open transaction `tx`: the `ItemPlace::ScratchSlots` a dispensed fluid container runs in (`DispenseFluidContainer`), `stack` in its first slot. |
| `scratch_take_slots` | `ServerHost` | Takes both slots out of two-slot place `place`, in `tx`, ending the place: what the first holds and what the second caught. |
| `dispense_with_remainder` | `ServerHost` | `DefaultDispenseItemBehavior.consumeWithRemainder`: one of `stack` is used and `remainder` replaces it, a single `stack` by becoming it, else into the dispenser at `pos` (`DispenserBlockEntity.insertItem`) or, where it doesn't fit, out of its front with the click and smoke. |
| `give` | `ClientHost` | Creative mode only: gives the player `count` of `item` (`minecraft:diamond`) as taking it from the creative inventory does (`ServerboundSetCreativeModeSlot`): the selected slot first, then the hotbar, then the inventory. |
| `hover_name` | `ClientHost` | `ItemStack.getHoverName()` as plain text: the custom name, else a written book's non-blank title, else the item's name in the client's language (potion contents, a head's owner and the `item_name` component as vanilla). |
| `enchantments` | `ClientHost` | Registered enchantments, in registry order. |
| `enchantment_min_cost` | `ClientHost` | `Enchantment.getMinCost(level)`: the enchanting power needed, or None for an unknown id. |
| `enchantment_supports` | `ClientHost` | Whether enchantment `id` can go on `item` (`Enchantment.isSupportedItem`). |

### `keys`

| Method | On | What it does |
| --- | --- | --- |
| `biome_key` | `Host` | before world data is installed, or for an unknown id. |
| `game_event_key` | `Host` |  |
| `block_key` | `Host` | The key of the block named `id`, for a name that only arrives at run time (data, config, a command, a network message). |
| `effect_key` | `Host` | The key of the effect named `id`: the full registry name, `minecraft:speed` or `mymod:rage`. |
| `registry_entry` | `Host` | Resolve a canonical entry in the current side's live registry. |
| `registry_entries` | `Host` | Entry identifiers in the current table, each once. |
| `registry_entry_in_tag` | `Host` | Membership in the current side's merged tags. |
| `registry_entry_key_slot` | `Host` | Resolve a live typed key's load-owned slot, never its protocol numeric id. |
| `block_keys` | `Host` | Every block key in the registry's registration order: vanilla's first, then mod blocks in the order they registered. |
| `item_keys` | `Host` | Every item key in the registry's registration order: vanilla's first, then mod items in the order they registered. |
| `entity_type_keys` | `Host` | Every built-in entity-type key in registry order (`BuiltInRegistries.ENTITY_TYPE` order): vanilla's first, then mod types in the order they registered. |
| `menu_keys` | `Host` | Every built-in menu-type key in registry order (`BuiltInRegistries.MENU` order): vanilla's first, then mod types in the order they registered. |
| `enchantment_keys` | `Host` | Every enchantment key in global sorted loaded-registry order, including vanilla and mod data-pack entries together (`RegistryDataLoader` sorts by id). |
| `registry_protocol_id` | `Host` | The network id of `name` in `registry` (`Registry.getId`): the number a packet carries for it, for a mod that builds or reads raw packets (`minecraft:particle_type` and `minecraft:flame`). |

### `levels`

| Method | On | What it does |
| --- | --- | --- |
| `is_loaded` | `Host` | Whether a mod with the `horizon-mod.toml` id `id` is installed: a native, wasm or content package (Fabric's `FabricLoader.isModLoaded`, NeoForge's `ModList.isLoaded`). |
| `chunk` | `Host` | Find a loaded chunk in an open server level. |

### `living`

| Method | On | What it does |
| --- | --- | --- |
| `living` | `Host` | The entity as a living entity (vanilla's `LivingEntity`: a mob, a player, an armor stand or the dragon), or None for another kind or a stale handle. |

### `math`

| Method | On | What it does |
| --- | --- | --- |
| `math_profile` | `Host` | The native engine's fixed selection for the seven platform-dependent Java Math operations. |

### `menus`

| Method | On | What it does |
| --- | --- | --- |
| `open_menu` | `ServerHost` | Opens a chest menu of `rows` rows (1 to 6) titled `title` for the player, showing `items` (by slot, padded with empty ones), in place of the menu they had open: its window id, a new one each time (`ServerPlayer.openMenu`). |
| `menu` | `ServerHost` | The menu `open_menu` opened that the player has open as window `window` (the window id `open_menu`, `menu_clicked` and `menu_closed` give). |
| `close_menu` | `ServerHost` | Closes the player's native menu (a `menu_closed` follows). |
| `menu_set_items` | `ServerHost` | Replaces everything the player's open `open_menu` menu `window` shows with `items` (by slot, padded with empty ones, extra ones dropped, as `open_menu` takes them) and sends it at once as one `ClientboundContainerSetContentPacket` (`AbstractContainerMenu.sendAllDataToRemote`), without reopening the menu: the title, the cursor and the screen stay. |
| `local_player` | `ClientHost` | The local player in a logged-in client world. |

### `merchants`

| Method | On | What it does |
| --- | --- | --- |
| `merchant` | `ServerHost` | Narrows an entity to a villager or wandering trader, or None for another kind, a stale handle, or a client. |

### `minecarts`

| Method | On | What it does |
| --- | --- | --- |
| `minecart` | `ServerHost` | Narrows an entity to a minecart of any kind, or None for another kind of entity, a stale handle, a client, or an entity held out of storage. |

### `mobs`

| Method | On | What it does |
| --- | --- | --- |
| `mob` | `Host` | The entity as a vanilla Mob: a mob or the ender dragon. |
| `kill_mob` | `ServerHost` | Kills mob `id` as `/kill` does: it falls and drops its loot. |
| `dragon` | `ServerHost` | The entity as the ender dragon, or None for another kind, a stale handle or a client. |

### `mod_enchantments`

| Method | On | What it does |
| --- | --- | --- |
| `enchantment_level_for` | `ServerHost` | and the caller's `mod_slot` named, so a mod can apply a per-wearer level. |

### `mod_state`

| Method | On | What it does |
| --- | --- | --- |
| `attachment_query` | `Host` | Capture a SNAPSHOT of the current dimension's attachment residency and values. |
| `attachments` | `Host` | The server-owned holders' attachments; `TYPE.get(host, holder)` reads one. |
| `attachment_validate_write` | `ServerHost` | Validate captured residency against the current dimension immediately before an ordinary journaled attachment write. |
| `transaction_open` | `ServerHost` | Opens a transaction, nested in the open one if any (`Transaction.open`), and gives back the seal that ends it. |
| `transaction_close` | `ServerHost` | Closes the transaction `seal` ends, committed or aborted, aborting first any opened inside it and left open. |
| `transaction_lifecycle` | `ServerHost` | Where the transaction system is (`Transaction.getLifecycle`): none open, one open, a nested one closing, or the outermost closing while the game tells the world of a commit. |
| `transaction_current` | `ServerHost` | The innermost open transaction, to write in (`Transaction.getCurrentOpenedTransaction`); none when none is open. |
| `transaction_depth` | `ServerHost` | How deeply `tx` is nested (`TransactionContext.depth`): 0 for the outermost, none for a transaction that is not open. |
| `schedule_task` | `ServerHost` | Asks for `Hooks::scheduled_task` to be called with `token` after `delay_ticks` server ticks (`MinecraftServer.tell(new TickTask(getTickCount() + delay, ..))`; 0 and 1 both mean the next tick, and a paused server does not advance it). |
| `cancel_task` | `ServerHost` | Withdraws a task the calling mod scheduled: true if it was still waiting. |

### `network`

| Method | On | What it does |
| --- | --- | --- |
| `player_send_packet` | `ServerHost` | Sends `packet` to that player's client as it is (`ServerPlayer.connection.send`), as a Paper plugin sends a packet through ProtocolLib or PacketEvents: a team, an objective, a score, a Tab-list entry the server doesn't keep. |
| `open_chat` | `ClientHost` | Opens the chat screen with `text` typed in for the player to finish and send, as T and / open it; nothing is sent. |
| `chat_line` | `ClientHost` | Adds `line` to this client's chat HUD and history (vanilla's `ChatComponent.addClientSystemMessage`): it shows here only, so nothing is sent to the server. |
| `send_chat` | `ClientHost` | A mod's chat line or command (`/` first), sent as `send_chat` sends it (ClientPacketListener.sendChat / sendCommand), to any server: no chat history, and no client chat event (vanilla's `ClientChatEvent` comes only from ChatScreen). |
| `send_packet` | `ClientHost` | Submits a vanilla play packet to any server, including unrelated plugin payloads. |
| `disconnect` | `ClientHost` | Leaves the server or the single-player world (NeoForge's `IClientCommonPacketListenerExtension.disconnect(Component)`), at the end of this frame, not inside the calling hook. |

### `payload_tokens`

| Method | On | What it does |
| --- | --- | --- |
| `check_payload_token` | `Host` | Whether `token` is one this load's registrar handed out: the frozen declaration at its slot has its id, protocol, direction and schema, and its owning mod is on. |

### `payloads`

| Method | On | What it does |
| --- | --- | --- |
| `player_send_payload` | `Host` | Queue exact application bytes for this live player's connection. |
| `send_payload_to_server` | `ClientHost` | Queue exact application bytes to the connected server, using the frozen declaration for its current protocol. |

### `players`

| Method | On | What it does |
| --- | --- | --- |
| `item_cooldown_group` | `Host` | `ItemCooldowns.getCooldownGroup(stack)`: the group `stack`'s cooldown is filed under, the group its `use_cooldown` component names (its own, else its item's default), else its item id. |
| `player` | `Host` | Resolve a real online player by UUID, never a fake. |
| `fake_player` | `ServerHost` | Create or retrieve a connectionless player for the level currently in place. |
| `server_max_players` | `ServerHost` | The most players who may be on at once (`PlayerList.getMaxPlayers`, `max-players`), as the server list shows it and logins are refused beyond. |
| `server_set_max_players` | `ServerHost` | Sets the player limit from the next login and server list ping on (Paper's `Server.setMaxPlayers`); players already on stay, even when they are more than the new limit. |
| `server_define_terrain_view` | `ServerHost` | Define or replace a view owned by the calling mod. |
| `player_set_terrain_view` | `ServerHost` | Select the calling mod's view for this connected player and dimension, or release its selection with `None`. |
| `profile_by_uuid` | `ServerHost` | The profile of the player with this UUID, as the server knows it, never asked of Mojang: an online player's whole profile (the properties the player list gives, so with those `player_set_profile_property` set), else the user cache's name (`usercache.json`, a dedicated server's), without properties. |
| `profile_by_name` | `ServerHost` | The profile of the player with this name, any case, as `profile_by_uuid` knows it: an online player's, else an unexpired user cache entry. |
| `push_player` | `ClientHost` | For a client mod: a push added to the local player's velocity, in blocks a tick, as an explosion's knockback is (the window's player moves itself, as vanilla's client does). |
| `select_hotbar_slot` | `ClientHost` | For a client mod: chooses the local player's hotbar slot, `slot` (0 to 8), as `Inventory.setSelectedSlot` does for a number key. |
| `set_collision_exclusions` | `ClientHost` | For a client mod: the local player's collision exclusions, the client half of `EntityRef::set_collision_exclusions`, with the same boxes, rules and refusals (a client with no player of its own answers `Stale`). |
| `cross_passage` | `ClientHost` | Moves the window's player through `passage` (arismoko/horizon#3775), one its server told it of, as Immersive Portals' client does: its feet, where it was a tick ago (so the frames between two ticks do not jump), its velocity and its look come out at the far side, turned with the passage, and the server is told the passage and where the feet were. |
| `last_passage_refusal` | `ClientHost` | The server's refusal of the window's last crossing (`Self::cross_passage`) and the passage it named, until the next crossing; None when it was not refused or no answer came yet. |

### `projectiles`

| Method | On | What it does |
| --- | --- | --- |
| `spawn_projectile_from_rotation` | `ServerHost` | projectile: snowball, egg, pearl, potion, experience bottle, wind charge, or a registered mod thrown type. |
| `projectile` | `ServerHost` | Narrows an entity to a projectile: one in the projectile list, a trident a player threw or a firework rocket. |
| `fishing_hook` | `ServerHost` | Narrows an entity to a cast fishing hook. |

### `recipe_properties`

| Method | On | What it does |
| --- | --- | --- |
| `mod_recipe_property_contains` | `Host` | Whether a mod-created recipe property set accepts an item ID. |

### `recipes`

| Method | On | What it does |
| --- | --- | --- |
| `recipes_of_type` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `recipe_ingredient` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `recipe_result` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `known_recipes` | `Host` | The `book_id`s of the recipes the player's recipe book holds now (the server's book on a server); empty outside a world. |
| `recipe_book_category` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `panel_items` | `ClientHost` | Shows the panel beside container screens with these items (at most 8192), as the game's own slots in pages, with a search box; the player's clicks, searches and keys over items come back as `panel_event`. |
| `panel_style` | `ClientHost` | How mod `mod_id`'s item panel (`panel_items`) looks from now on: `PanelStyle::RecipeBook` (the default: the recipe book's page arrows and a plain search box), `PanelStyle::Viewer` (square page buttons on a bar over the items and the viewer's search box, as Horizon's Recipes panel draws them) or `PanelStyle::Classic` (JEI's list on Minecraft 1.12: flush right under page buttons that always show and wrap around, the search box at the foot of the screen with a settings button that opens the mod's page in Options > Mods). |
| `place_recipe` | `ClientHost` | Places a recipe in the open screen's grid as a click in the recipe book does (`ServerboundPlaceRecipePacket`): the inventory's grid or a crafting table's. |
| `transfer_recipe` | `ClientHost` | Move available ingredients for a viewer recipe into the open crafting grid, without requiring the recipe to be learned. |
| `transfer_recipe_id` | `ClientHost` | A viewer's "+" for a recipe by its id, into the open menu: a machine's mod menu as well as a crafting grid. |
| `transfer_recipe_check` | `ClientHost` | What `transfer_recipe(book_id)` would do now, worked out by the server without moving anything: the player's inventory, the grid and every container a mod lends the grid's recipes (a crafting station's chests, which the client can't see) counted. |
| `recipe_book_ids` | `ClientHost` | The `book_id`s of the recipe `recipe_id` (`minecraft:stick`), the ids `transfer_recipe`, `transfer_recipe_check`, `place_recipe` and `known_recipes` use: one for each way the recipe book shows it, in the book's order. |

### `resources`

| Method | On | What it does |
| --- | --- | --- |
| `resource` | `Host` | The winning bytes of resource `id` in `pack_type`'s stack, as `ResourceManager.getResource` answers it: the highest selected pack that supplies the file wins. |

### `screens`

| Method | On | What it does |
| --- | --- | --- |
| `preset_editor` | `ClientHost` | The world-preset editor currently open over Create World, or none. |
| `preset_editor_dimensions` | `ClientHost` | A copy of the open preset editor's working dimension stems. |
| `set_preset_editor_dimensions` | `ClientHost` | Replace the open editor's working dimension stems. |
| `screen_pos` | `ClientHost` | The block of the mod screen that is open (`screen`), as the server named it when the block opened its menu type; None when no mod screen is open. 401. |
| `screen_extra_data` | `ClientHost` | What the block's mod wrote for the mod screen that is open (`menu_extra_data` on the server), empty when it wrote nothing; None when no mod screen is open. |
| `screen_exclusion` | `ClientHost` | Where the open screen draws outside its `rect`, and where mods draw over it: the recipe book's panel, the creative tab rows, the active effects beside the inventory, then whatever `screen_exclusion_areas` adds. |
| `screen_hovered_item` | `ClientHost` | The id of the item in the slot under the pointer on the open container screen (`AbstractContainerScreen.getHoveredSlot().getItem()`), None over no slot or an empty one. |
| `apply_screen_blur` | `ClientHost` | Requests vanilla's menu background blur for this frame, with a per-frame guard so blur is applied at most once per frame. |
| `menu_blur_radius` | `ClientHost` | The user's configured menu background blur radius (`menuBackgroundBlurriness` in options.txt, usually 0.0 to 10.0; default 5.0). 0.0 means blur is turned off. |
| `mod_screen_open` | `ClientHost` | Opens a screen of mod `mod_id`'s own over the world: the screen hooks fire over it as over any other, so the mod draws it in `screen_render_post` and takes its clicks, scroll, keys and typing there; the mouse is freed, and Escape closes it. |
| `mod_screen_close` | `ClientHost` | Closes mod `mod_id`'s own screen, if it is open. |
| `close_screen` | `ClientHost` | Closes the open screen as its Escape key does: a container screen closes as the player's container close (`ServerboundContainerClosePacket` with the menu's id, then the client menu closed), a pause menu closes to the game, and another screen runs its own close. |
| `request_client_quit` | `ClientHost` | Queues the primary window's normal Quit Game action after the current hook dispatch. |
| `open_inventory` | `ClientHost` | Opens the player's inventory as the inventory key does: locally, with no packet (the player menu is always open on the server). |
| `sign_editor` | `ClientHost` | The sign the sign editor is on, its block position and side; None when no sign editor is open or outside a world. |
| `mod_screen` | `ClientHost` | The mod whose own screen is open, if one is (Escape, or another screen replacing it, closes it). |
| `screen_title` | `ClientHost` | The open menu's title (the block's name, or the one it was given), as plain text. |
| `window_focused` | `ClientHost` | The last window focus state reported by the window system. |
| `observed_screen_id` | `ClientHost` | The same screen identifier provided to screen hooks. |
| `is_key_down` | `ClientHost` | Whether a specific key is currently held down. |
| `request_screenshot` | `ClientHost` | Takes a screenshot as the screenshot key (F2) does (`Screenshot.grab`): the next frame is read back at the size F2 saves (the game's resolution times the screenshot quality), the `screenshot` hook runs on it as for F2, so every mod's screenshot listener sees it, and it is written to the player's `screenshots` folder under vanilla's file name, with F2's chat line. |

### `services`

| Method | On | What it does |
| --- | --- | --- |
| `commands` | `ServerHost` | The commands service. |
| `levels` | `ServerHost` | The levels service. |
| `messages` | `ServerHost` | The messages service. |
| `players` | `ServerHost` | The players service. |
| `profiles` | `ServerHost` | The profiles service. |
| `runtime_levels` | `ServerHost` | The runtime_levels service. |
| `instance_groups` | `ClientHost` | Persistent model groups. |
| `client_session` | `ClientHost` | The client_session service. |
| `render_assets` | `ClientHost` | The render_assets service. |
| `client_server` | `Host` | The connected server, not a stale reconnect target. |
| `client_language` | `Host` | The applied language code, not a pending options selection. |
| `client_latency` | `Host` | The local player's latency in milliseconds as the tab list draws it (`PlayerInfo.getLatency`): the server's smoothed keep-alive average, refreshed by its own broadcast. 0 before the server's first measurement, as vanilla; None outside a logged-in client world or before the server lists this player. |
| `client_tab_list` | `Host` | The player list as the Tab overlay draws it: each listed player's drawn name, profile name, UUID, game mode and latency in drawn order, at most the 80 it draws, with the header and footer. |
| `client_frame_stats` | `Host` | Existing local frame counters; None outside a logged-in client world. |
| `refresh_commands` | `Host` | A mod changed the commands it registers (`register_commands`): at the start of the next server tick every mod is asked again, the commands players may run are rebuilt, and every player on the server is sent their tree again, after `player_commands_sent`. |
| `feedback` | `Host` | A command's success feedback, routed to its originating source using vanilla's silence, send-command-feedback and block/cart track-output rules. |
| `run_command` | `Host` | Runs `line` (no slash) as the server's console does: permission level 4, from the overworld's spawn, with no output. |
| `run_command_with_output` | `Host` | `run_command` with its feedback kept: the same line, source and run path (one leading slash is dropped here), answering with whether it ran without failing (`success`, what `run_command` returns) and what the console would have printed for it as plain text (`feedback`, as a mod that answers a remote console wants; Paper's `Server.dispatchCommand` with a capturing `CommandSender`). |
| `list_levels` | `Host` | Loaded server levels, in vanilla dimension order. |
| `text_allowed` | `Host` | Whether the server's chat filter (`server.toml` `[chat_filter]`) lets `text` through: true without a filter, None inside a player's hook (where the server is out of reach). |
| `broadcast` | `Host` | A system line in every player's chat and the server's log. |
| `server_players` | `Host` | The players on this server, in the order they joined; in single player that includes the world's owner, who joins its server like any player. |
| `player_by_name` | `Host` | Find a connected player by their account name, case-insensitively. |
| `acting_player` | `Host` | The player whose action or command a hook hears, as their state is swapped in (`run_mod_command`, `right_click_block`, `native_use_on`, `break_block`...): None outside such a hook. |
| `resolve_name` | `Host` | Resolve a Minecraft account name (`Notch`) through the game's profile resolver: the session server's profile, with the signed `textures` property a skin mod replaces a player's skin with. |
| `resolve_id` | `Host` | Resolve an account's UUID through the game's profile resolver, as `resolve_name` does by name. |
| `open` | `Host` | Open the level `id` (`mymod:instance/ab`), in the calling mod's namespace, as `level` describes. |
| `unload` | `Host` | Save and close the level `id` the calling mod opened, keeping its folder, once this server tick's levels have ticked: `level_unload` fires then. |
| `delete` | `Host` | Remove the level `id` the calling mod owns, folder and all, once this server tick's levels have ticked. |
| `is_open` | `Host` | Whether the level `id` is open: false until a queued `open` is applied, true until a queued `unload` or `delete` is. |
| `model` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `render_pipeline` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `animation` | `Host` | Published asset/data read: available in Setup, Reloading and World. |
| `create_group` | `Host` | Allocate up to 16,384 copies, initially length zero. |

### `tags`

| Method | On | What it does |
| --- | --- | --- |
| `tags` | `Host` | The merged-tag service, using load-resolved item and tag keys. |
| `unresolved_fluid_id` | `Host` | The saved name for an engine-issued unresolved fluid. |
| `block_in_tag` | `Host` | Whether the resolved block is in the world's merged block tag, on a client the tag its server sent. |
| `state_outline` | `Host` | The outline of `state` as `BlockState.getShape` answers it with an empty getter at the origin (`EmptyBlockGetter.INSTANCE`, `BlockPos.ZERO`): its boxes, each `[min x, y, z, max x, y, z]` in the block's own space, with no random offset and none of a world's context (a state that reads its neighbours or block entity answers what its state alone gives). |
| `state_fluid_state` | `Host` | The fluid state `state` holds (`BlockState.getFluidState`): water for a waterlogged state, the empty fluid for most. |
| `fluid_in_tag` | `Host` | Whether `fluid` is in the fluid tag `tag` (`FluidState.is(TagKey)`, `minecraft:water` or `water`), over the merged fluid tags, on a client those its server sent. |
| `fluid_is_same` | `Host` | Whether `fluid` and `other` are the same fluid (`Fluid.isSame`): a source and its flowing form are, so `minecraft:water` and `minecraft:flowing_water` are the same, water and lava not. |
| `fluid_bucket` | `Host` | The bucket item that picks `fluid` up (`Fluid.getBucket`), `minecraft:water_bucket` for water and flowing water. |
| `fluid_pickup_sound` | `Host` | The sound event a bucket plays as it picks `fluid` up (`Fluid.getPickupSound`), `minecraft:item.bucket.fill_lava` for lava and flowing lava. |
| `fluid_empty_sound` | `Host` | The sound event a bucket plays as it empties `fluid` (`BucketItem.playEmptySound`, the `BUCKET_EMPTY` sound of NeoForge's `FluidType`), `minecraft:item.bucket.empty_lava` for lava and flowing lava and `minecraft:item.bucket.empty` for water and a mod's world fluid. |

### `text`

| Method | On | What it does |
| --- | --- | --- |
| `text_from_json` | `Host` | Reads a text component as `/tellraw` and `ComponentSerialization.CODEC` take it: one JSON or SNBT value (`"plain"`, `{"text":"Hi","color":"gold"}`, `["a",{"text":"b"}]`, `translate`, `extra`, click and hover events), with nothing but whitespace after it (Paper's `GsonComponentSerializer.deserialize`). |
| `text_from_legacy` | `Host` | Reads legacy formatting codes (`§a`, `§l`, `§r`...) as the font draws them (`Style.applyLegacyFormat`; Bukkit's `ChatColor` strings): a colour code resets the decorations, `§r` resets everything, an unknown code is dropped. |
| `text_to_nbt` | `Host` | The text as item components and block entities store it (`ComponentSerialization` to NBT): what `minecraft:custom_name` and each `minecraft:lore` line hold. |

### `world_edit`

| Method | On | What it does |
| --- | --- | --- |
| `edit_blocks` | `ServerHost` | For a server mod: the runs made at once, straight into the chunks of the authoritative level `dimension` (the one being ticked, as for `native_set_block`), with what `fill ... strict` does for block entities (a block replaced by another loses its block entity, and a new block gets the one `native_set_block` would give it, a mod block's included); no neighbour is told, and the chunks changed go to the players whole. |
| `upgrade_block_entity_nbt` | `ServerHost` | A block entity's data saved at `data_version` (a schematic's, a structure file's, an older world's), upgraded as the chunk loader upgrades a block entity of a chunk of that version: its type's datafixes up to the game's DataVersion. |
| `upgrade_entity_nbt` | `ServerHost` | An entity's data saved at `data_version`, upgraded as the entity chunk loader upgrades an entity of a chunk of that version: its type's datafixes, its passengers' included, up to the game's DataVersion. |
| `surface_heights` | `ServerHost` | For a server mod: each column's height by `heightmap` in the authoritative level `dimension` (the one being ticked), none for a column not loaded. |
| `read_columns` | `ServerHost` | For a server mod: `below` blocks under and `above` blocks from each column's height by `heightmap`, as a palette of complete `BlockState` values and indexes, with biome tints (air outside the column and in columns not loaded). |
| `terrain_surface` | `ServerHost` | For a server mod: what the generator makes at each of `columns` of the level `dimension` (the one being ticked) before its features are placed, whether or not the chunk is loaded: it reads the generator and creates no chunk. |
| `structure_template` | `ServerHost` | For a server mod: structure template `id` (`mymod:hut`, found as `StructureTemplateManager.get` finds it: the world's saved structures, then the game's and the mods' own) as `/place template id` would place it at `at` with `rotation` and `mirror`, without placing anything. |

### `world_request`

| Method | On | What it does |
| --- | --- | --- |
| `request_world` | `ClientHost` | Asks the player whether to open `saves/<folder>` the way the title screen's Play button does: the request is queued, and the title's next frame shows a question that names the mod and the save, with Yes and No. |
