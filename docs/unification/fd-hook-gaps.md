удит: NeoForge-API Farmer's Delight против хуков MAIN (Magnetite/Horizon)

## 0. Важная поправка к постановке

**Java-исходников FD в рабочем каталоге нет.** Проверено: `farmers-delight-src/**/*.java` — пусто, `package vectorwing` — пусто, `**/*.jar` — пусто. `farmers-delight-src/farmers-delight/` — это **Rust-порт** (`Cargo.toml`, `horizon-mod.toml`, `src/*.rs`, 16 файлов) плюс датапак/ресурспак оригинала (`README.md:4-8`). Поэтому вместо `@SubscribeEvent`-строк ниже цитируются: (а) Rust-хуки, которые порт реализует вместо каждого события NeoForge (комментарии порта называют исходные FD-классы), (б) NeoForge-артефакты, уцелевшие в датапаке (`neoforge/biome_modifier/`, `neoforge:compound`, `minecraft:copy_components`), (в) два документа MAIN, где соответствия уже частично зафиксированы: `docs/design/mod-api/farmers-delight-hooks.md` (34 mixin-точки FD на существующие хуки) и `docs/unification/step0-core-map.md` (разведка ядра под ТЗ).

Каталог хуков MAIN: `crates/horizon-mod-vocab/src/hooks/*.rs` (738 файлов, не компилируются, читаются `syn`-генератором). Грамматика: `crates/horizon-mod-vocab/src/hooks/mod.rs:1-52`; ключи-аналоги объявлены атрибутом `#[hook(...)]`:

```
#[hook(side = "server", contract = "collect", out = "drops",
       neoforge = "LivingDropsEvent, BlockDropsEvent")]
fn loot_drops();
```

Формат значений: точное имя события, список через запятую, либо `none (причина)`. `fabric` — тот же стиль, но заполнен **всего у 5 хуков** (`client_tick_pre.rs:11`, `client_tick_post.rs:11`, `living_death_post.rs:18`, `server_stopping.rs:16`, `server_tick.rs:12`).

---

## 1. Итоговая таблица

| NeoForge API/событие | где в FD (файл:строка) | зачем FD | аналог в MAIN (путь) | статус | что добавить в ядро |
|---|---|---|---|---|---|
| `DeferredRegister` / регистры блоков, предметов, `BlockEntityType`, `MenuType`, `CreativeModeTab` | `src/lib.rs:160` (`Hooks::register`), `src/lib.rs:179-370` | объявить доски, плиту, сковороду, котёл, корзины, шкафы, меню | `Registrar` из `Hooks::register`; `horizon-port-pack/src/events.rs:141-166` (таблица `API`: регистрационные события → методы `Registrar`) | **есть** | — |
| `VillagerTradesEvent`, `WandererTradesEvent` | `data/farmersdelight/villager_trade/*.json` (8 файлов), `data/minecraft/tags/villager_trade/farmer/level_{1,2}.json` | семена у бродячего торговца, покупка урожая фермером | данных-путь есть: `crates/horizon-client/src/merchant.rs:363-427`; каталог хуков НЕ имеет; `events.rs:191-192` классифицирует как `DATA` | **частично** | см. §2.2 |
| `Capabilities.*`, `RegisterCapabilitiesEvent` | `src/lib.rs:179-300` (`registrar.block_container(ModContainer{…})` для доски/плиты/сковороды/котла/корзин), `src/basket.rs:8`, `src/storage.rs:14` | воронка/трубы в станции, 27-слотовые шкафы | `Registrar::block_container`; ключи `mod_api::ITEMS`/`FLUIDS`/`ENERGY`/`ITEM_*`/`ENTITY_*` (`events.rs:141-166`); `crates/horizon-mod-vocab/src/parts/{capabilities,capability_registration,default_capabilities}.rs` | **есть** | — |
| `ItemStackHandler`, меню, `MenuType`, `ContainerData` | `src/lib.rs:288-300` (`menu: Some("farmersdelight:cooking_pot")`), `src/cooking_pot.rs:648` | 9 слотов котла, слот блюда, слот тары, `data`-слоты | `crates/horizon-mod-vocab/src/parts/menus.rs`; `Registrar::menu_recipe_book` (используется в `src/lib.rs:315`) | **есть** | — |
| `RecipeBookCategoriesEvent` (категория книги) | `src/lib.rs:434` (`recipe_book_categories`) | книга рецептов котла | `crates/horizon-mod-vocab/src/hooks/recipe_book_categories.rs` (`neoforge = "none (RegisterRecipeBookSearchCategoriesEvent registers search groups only…)"`) | **есть** | — |
| `RegisterEvent` (кастомные `RecipeType`/сериализаторы) | `src/lib.rs:165-176` (`register_recipe_type` + `register_raw_recipe_serializer` для `cutting`/`cooking`/`dough`) | три своих типа рецептов | `crates/horizon-mod-vocab/src/parts/recipe_serializers.rs`, `parts/crafting_recipes.rs:162-170` | **есть** | — |
| `CustomRecipe` (рецепт без сетки; тесто = пшеница + ведро воды) | `src/lib.rs:17-20` (`DOUGH_RECIPE_TYPE`, «the engine cannot match»), `src/lib.rs:172-176` | тесто, ведро возвращается | `CraftingRecipeKind` с `matches/assemble/remaining_items` — `parts/crafting_recipes.rs:79-107`, сетка 1..=3 `:17-34` | **частично** | см. §2.5 |
| `IGlobalLootModifier` / `LootModifier` | `src/loot_modifiers.rs:1-2` («FD's global loot modifiers (NeoForge's `LootModifiers.java`)»), `src/lib.rs:471-491` | 38 модификаторов: нож-падальщик, сундучные pool'ы, лук с иллагеров, самородки | `hooks/loot_drops.rs:5` (`neoforge = "LivingDropsEvent, BlockDropsEvent"`), `hooks/loot_function.rs` (`neoforge = "RegisterEvent"`), `hooks/loot_table_load.rs` | **есть** | — |
| `minecraft:copy_components` (ванильная loot-функция) | `data/farmersdelight/loot_table/blocks/cooking_pot.json:23` + 15 файлов (все шкафы, корзины) | содержимое котла/шкафа выпадает с блоком: `custom_name`, `farmersdelight:meal`, `farmersdelight:container` | **нет**: закрытый `match` в `crates/horizon-world/src/loot.rs:1520-2007`; `set_components` есть (`:2007`), `copy_components` отсутствует; ад-хок в `crates/horizon-client/src/game/blocks/spill.rs:28,78,169` | **нет** | см. §2.1 |
| `IGlobalLootModifier` с JSON-файлами `global_loot_modifiers` | — | — | `hooks/loot_table_load.rs` (`target = horizon_world::loot::native_loot_table_load`) | **есть** | — |
| `BiomeModifier` / `farmersdelight:add_features_by_filter` | `data/farmersdelight/neoforge/biome_modifier/*.json:6` (9 файлов: `wild_cabbages`, `wild_onions`, `wild_tomatoes`, `wild_rice`, `wild_carrots`, `wild_potatoes`, `wild_beetroots`, `brown_mushroom_colony`, `red_mushroom_colony`) | дикие культуры и колонии грибов в биомах по `allowed_biomes`/`denied_biomes` | рантайм читает только `neoforge:`/`forge:`: `crates/horizon-client/src/mod_content_biomes.rs:375-450`, неизвестный тип → `warn` + skip (`:430`). Трансляция **только на этапе порта**: `crates/horizon-port-pack/src/biome_modifiers.rs:108-127`. Программный аналог: `parts/biome_modifiers.rs` (`declare_biome_modifier`) | **частично** | см. §2.3 |
| `neoforge:compound` (OR-ингредиент) | `data/farmersdelight/recipe/cooking/rabbit_stew.json:13`, `recipe/salvaging/*.json:5` (13 файлов) | OR-группы в рецептах «разобрать мебель» и супов | `Ingredient` с `Shape::Compound` есть (`docs/unification/step0-core-map.md` §2, `item/crafting.rs:69-80`), хук `hooks/ingredient.rs:39` (`neoforge = "ICustomIngredient"`); но порт их теряет — `README.md:101-102`, `src/lib.rs:80` («FD's `item_ability` actions… the rest the recipe can live without») | **частично** (есть в ядре, не доходит через `host.recipe_ingredient`) | — (шаг 1, §2.3) |
| `IItemExtension.canPerformAction` / `ItemAbility` (нож = меч+ножницы+тяпка) | `src/cutting_board.rs:73` (`is_carving_tool`), `src/lib.rs:129` (`tools` из рецепта) | доска режет только инструментом с действием `knife_dig` | `hooks/can_perform_action.rs` (`neoforge = "IItemExtension.canPerformAction"`) | **есть** | — |
| `IBlockExtension.*` (`canSustainPlant`, `getBlockPathType`, `isStickyBlock`, `canHarvestBlock`, `canBeConnectedTo`, `getBounceRestitution`, `canDropFromExplosion`, `canEntityDestroy`, `canBeHydrated`, `getAdjacentBlockPathType`, `getBeaconColorMultiplier`) | `src/lib.rs:112` (`covered`, костровая готовка), `src/basket.rs:26`, `src/cooking_pot.rs:203-231`, `src/stove.rs:112-146` | рост культур на богатой земле, мокрая земля, костёр под станцией | `hooks/can_sustain_plant.rs:21`, `block_path_type.rs`, `block_sticky.rs`, `can_harvest_block.rs:18`, `can_be_connected_to.rs:25`, `bounce_restitution.rs:34`, `can_drop_from_explosion.rs:23`, `can_entity_destroy.rs:32`, `can_be_hydrated.rs:20`, `adjacent_block_path_type.rs:21`, `beacon_color_multiplier.rs:23` | **есть** (полный набор `IBlockExtension`) | — |
| `IItemExtension.*` (`applyEnchantments`, `canEquip`, `canFitInsideContainerItems`, `canGrindstoneRepair`) | `src/cutting_board.rs:246`, `src/skillet.rs:41` | нож/сковорода как предметы с поведением | `hooks/apply_enchantments.rs:24`, `can_equip.rs:45`, `can_fit_inside_container_items.rs:31`, `can_grindstone_repair.rs:32` | **есть** | — |
| `IShearable` / `DispenseItemBehavior` (ножницы, раздатчик) | `src/config.rs:39` (`enableCuttingBoardDispenserBehavior`) | раздатчик режет на доске перед собой | `hooks/dispense_item.rs` (`scope = "item"`, `neoforge = "none (vanilla DispenserBlock.getDispenseMethod…)"`), `hooks/block_dispense_armor.rs`, `hooks/block_failed_dispense.rs`, `hooks/block_shear_entity.rs` | **есть** | — |
| `BlockGrowFeatureEvent`, `CropGrowEvent.Pre/Post`, `BonemealEvent` | `src/lib.rs:112`, дикие культуры из ворлдгена | рост культур, костная мука | `hooks/block_grow_feature.rs:13`, `crop_grow.rs:16,27`, `bonemeal.rs:16`, `is_valid_bonemeal_target.rs:25`, `perform_bonemeal.rs:17`, `is_bonemeal_success.rs:18` | **есть** | — |
| `BlockEntityTicker` (тики станций) | `src/cooking_pot.rs:528`, `src/stove.rs:320`, `src/skillet.rs:179`, `src/cutting_board.rs:384`, `src/basket.rs:113` | готовка, жарка, ловля предметов | `hooks/block_entity_tick.rs` — `pos`+`entity`+`nbt`/`items`/`data`, **без `LevelRef`/измерения**; обход — `host.dimension()` (`statион` работает только в оверворлде: `README.md:93`) | **частично** | см. §2.4 |
| `OnDatapackSyncEvent` | `src/lib.rs:426` (`datapack_sync` → публикация таблиц в `datapack_sync`) | таблицы рецептов станций клиенту | `hooks/datapack_sync.rs` (`neoforge = "OnDatapackSyncEvent"`, `out = "recipe_types"`) | **есть** | — |
| `AddServerReloadListenersEvent` | не используется портом (публикация через `datapack_sync`) | пересборка таблиц при `/reload` | `hooks/add_server_reload_listeners.rs:15` (`neoforge = "AddServerReloadListenersEvent"`) | **есть** | — (ТЗ шаг 5 — использовать) |
| `ModConfigSpec` / `ModConfigEvent.Loading`/`Reloading`/`Unloading` | `src/config.rs:1-8` (23 опции FD), `src/lib.rs:456-469` | гейты HUD, тултипов, крафта ящиков | `hooks/mod_configs.rs:18,32,41,52` (`register_configs`, `config_loading`, `config_reloading`, `config_unloading`) | **есть** | — |
| `ItemTooltipEvent`, `RenderGuiEvent.Post`, `RenderGuiLayerEvent.Pre`, `ScreenEvent`, `ContainerScreenEvent.Render` | `src/lib.rs:493` (`item_tooltip`), `:524` (`gui_render_post`), `:512` (`health_hud`), `src/pot_screen.rs:45,63`, `src/storage_screen.rs:40` | тултипы еды, оверлеи Nourishment/Comfort, экран котла | `hooks/item_tooltip.rs` (`ItemTooltipEvent`), `hooks/gui_render_post.rs` (`RenderGuiEvent.Post`), `hooks/health_hud.rs` (`RenderGuiLayerEvent.Pre`), `hooks/screen.rs`, `hooks/screen_draw.rs` | **есть** | — |
| Клиентский рендер блок-сущностей (`BlockEntityRenderer`) | `src/cooking_pot.rs:518`, `src/cutting_board.rs:364`, `src/stove.rs:296`, `src/skillet.rs:170` | предметы на доске/плите/сковороде/в котле | `hooks/block_entity_submit.rs:25`, `hooks/block_entity_update_tag.rs:21` | **есть** | — |
| `PayloadRegistrar` (сетевые пакеты) | `src/sync.rs`, заменено на `block_entity_update_tag` + `datapack_sync` | синхронизация станций | `crates/horizon-mod-vocab/src/parts/payloads.rs` (файл есть; форма API не вычитана) | **есть** (не верифицировано детально) | — |
| `RegisterEvent` (кастомные `ConsumeEffect`) | `src/lib.rs:562` (`consume_effect`: `farmersdelight:remove_one_harmful`, `farmersdelight:heal`) | какао снимает один вредный эффект | `hooks/consume_effect.rs` (`neoforge = "none (runtime ConsumeEffect.apply; RegisterEvent registers effect types, not invocations)"`) | **есть** | — |
| `MobEffect`-тики (`applyEffectTick`, `shouldApplyEffectTickThisTick`) | `src/lib.rs:538,551`, `src/effects.rs` | Nourishment каждый тик, Comfort раз в 80 | `hooks/should_apply_effect_tick_this_tick.rs:16`, `hooks/apply_effect_tick.rs` | **есть** | — |
| `Item.verifyComponentsAfterLoad` / дефолтные компоненты | `src/lib.rs:598` (`modify_default_components`) | Nourishment на ванильных супах | `hooks/modify_default_components.rs:22` | **есть** | — |
| События `Block.getStateForPlacement` | `src/basket.rs:83`, `src/cooking_pot.rs:357`, `src/stove.rs:147` | поворот станции к игроку | `hooks/state_for_placement.rs` (`scope = "block"`, `neoforge = "none (vanilla Block.getStateForPlacement…)"`) | **есть** | — |
| `useItemOn` (взаимодействие с блоком) | `src/cutting_board.rs:246`, `src/cooking_pot.rs:382`, `src/skillet.rs:41`, `src/stove.rs:166` | положить/снять/резать/жарить | `hooks/block_use_item_on.rs` (`scope = "block"`, `contract = "replace"`) + глобальный `hooks/use_item_on_block.rs` | **есть** | — |
| Мутабельный использованный стак (флип сковороды) | `README.md:96-98` («Clavisha „подбросить"… компонентная запись в использованный предмет недоступна мод-хукам») | перевернуть блюдо из руки на сковороду | `BlockClick.stack: Option<&ItemStack>` — иммутабельная ссылка (`crates/horizon-mod-vocab/src/block/types.rs:423-441`) | **нет** | см. §2.6 |

---

## 2. Предложения общих хуков (для «нет/частично»)

### 2.1 `copy_components` — не хук, ванильная loot-функция (ТЗ шаг 6, часть 1)

Закрытый `match` в `crates/horizon-world/src/loot.rs:1520-2007` уже знает `set_components` (`:2007`) — `copy_components` добавляется рядом как вариант `source: block_entity` + `include`-фильтр компонентов. Это дешёвая правка без нового API-имени, ровно как записано в ТЗ (`ТЗ ядро Horizon…md`, «Шаг 6», пункт `minecraft:copy_components`).

- **Кто выиграет:** все FD-шкафы/корзины/котёл (16 файлов), Sophisticated Storage, Create (basin/toolbox), любые моды с контейнером в block-entity.
- **Куда ложится:** ТЗ шаг 6, приоритет №2 по порядку работ (`ТЗ…md`, «Порядок работ», п.2).
- **Fabric-аналог:** нет отдельного события — функция ванильная, Fabric-мод пишет те же `minecraft:copy_components` в loot-таблице. Ключ `fabric` не нужен.

### 2.2 Доступ к `villager_trade`/`trade_set` вне `minecraft:` — фикс загрузчика + общий хук

Корень читается только в одном неймспейсе: `crates/horizon-client/src/merchant.rs:365` — `let data = root.join("data/minecraft")`, затем `walk(&data.join("villager_trade"))`, `walk(&data.join("trade_set"))`. Оверлей паков неймспейс-агностичен (`PackOverlay::kind`/`tag_kind`, `crates/horizon-world/src/loot.rs:67-88`), поэтому `.hmod`-контент доходит, а **датапак-файлы вида `data/farmersdelight/villager_trade/onion.json` из корня — нет**. Именно это и есть блокер «сделки FD с жителями», а не отсутствие хука.

Предлагаю два уровня:

1. **Фикс (не хук):** обойти `data/*/villager_trade` и `data/*/trade_set` по всем неймспейсам, как уже сделано для тегов (`merchant.rs:411-427`), с сохранением неймспейса в id.
2. **Общий хук**, если нужна рантайм-добавка без файлов:

```rust
// hooks/villager_trade_load.rs
pub trait Hooks {
    /// A `villager_trade` or `trade_set` definition is about to join the table ...
    #[hook(
        side = "server",
        contract = "collect",
        out = "trades",
        neoforge = "none (VillagerTradesEvent and WandererTradesEvent are data in 26.3: trade_set and villager_trade)"
    )]
    fn villager_trade_load();
}
```

- **Кто выиграет:** любой мод, добавляющий профессии/сделки из своего датапака (Villagers Reborn, Trade Cycling, стор-моды).
- **Куда ложится:** ТЗ шаг 6, часть «торговля» (порядок работ, п.7) — там же стоит вопрос «почему валидатор запрещает `trade_set` в своём неймспейсе».
- **Fabric-аналог:** `fabric-content-registries-v0` (`VillagerProfession`-регистры) — в `docs/fabric-ledger.md`, строка `fabric-content-registries-v0` помечена **Covered in part** только для `fire_odds`, регистры жителей — **Gap**. Ключ `fabric` ставить нечего.

### 2.3 Преобразование данных при загрузке — ТЗ шаг 1 (закрывает `add_features_by_filter` и `neoforge:compound`)

Это не «хук на биом-модификатор», а общая точка «прочитать/переписать JSON до разбора». Сейчас её нет: `data_pack_files` (`hooks/data_pack_files.rs`) покрывает только data-таблицы и не трогает `neoforge/biome_modifier/` и не переписывает содержимое; биом-модификаторы применяются позже, на этапе сборки контента (`crates/horizon-client/src/mod_content.rs:2682`).

```rust
// hooks/data_load_transform.rs
pub trait Hooks {
    /// The server is about to parse its data packs into tables ...
    #[hook(
        side = "server",
        contract = "collect",
        out = "files",
        context = "data_load",
        neoforge = "none (no NeoForge event transforms pack JSON before parsing; GatherDataEvent and IDataProvider run at development time)"
    )]
    fn data_load_transform();
}
```

Отличие от `data_pack_files`: тот **добавляет** файлы ниже пака, этот **видит и меняет** уже собранный набор (включая `neoforge/biome_modifier/`), поэтому `farmersdelight:add_features_by_filter` разворачивается в `neoforge:add_features` с `biomes: "#..."` без правки порта.

- **Кто выиграет:** все моды с самописными `BiomeModifier` и кастомными ингредиентами (`neoforge:compound` — 12 salvage-рецептов FD, Create's sequenced assembly на этапе 8).
- **Куда ложится:** ТЗ шаг 1; там же прямо названы оба случая (`add_features_by_filter`, compound-ингредиенты).
- **Уже есть альтернатива в коде:** `BiomeModifiersRegistrar::declare_biome_modifier` (`crates/horizon-mod-vocab/src/parts/biome_modifiers.rs`) — мод может выразить то же программно; хук нужен, чтобы **не переписывать датапак**.
- **Fabric-аналог:** `fabric-biome-api-v1` (`BiomeModifications`, `BiomeSelectors`) — в `docs/fabric-ledger.md` строка помечена **Gap** («programmatic biome modification phases… data-driven biomes alone do not provide these callbacks»). Ключ `fabric` = `fabric-biome-api-v1` только для биомной части, и он **не Covered**.

### 2.4 `LevelRef` и измерение в `block_entity_tick` — ТЗ шаг 3

Сейчас: `hooks/block_entity_tick.rs` несёт `pos`, `entity`, `nbt`, `items`, `data` — измерения нет; обход `host.dimension()` возвращает `&'static str` текущего подменённого уровня (`docs/unification/step0-core-map.md` §7, риск 2). Следствие в FD: `README.md:93` — «station block-entity ticks run in the overworld only»; в коде это видно на `src/basket.rs:117` (хардкод `host.level("minecraft:overworld")`).

Правка контракта (не новый хук, а `hook_version` на существующем):

```rust
#[hook(
    side = "server",
    contract = "listen",
    scope = "block_entity",
    key = "entity",
    version = 2,
    neoforge = "none (vanilla BlockEntityTicker; NeoForge adds no event or extension for it)"
)]
fn block_entity_tick();
```

- **Кто выиграет:** любая станция в Незере/Энде (Create, Immersive Engineering, Mekanism).
- **Куда ложится:** ТЗ шаг 3 (порядок работ, п.3).
- **Fabric-аналог:** нет — Fabric тоже не имеет события на тик BE (`fabric-lifecycle-events-v1` даёт только `ServerTickEvents`/`WorldTickEvents`). Ключ `fabric` не нужен.

### 2.5 `CustomRecipe` — ТЗ шаг 2, но наполовину уже покрыт

`CraftingRecipeKind` (`crates/horizon-mod-vocab/src/parts/crafting_recipes.rs:79-107`) уже даёт `decode/write/matches/assemble/remaining_items` и сетку 1..=3 (`:17-34`), а `remaining_items` — ровно механизм возврата ведра. Порт зарегистрировал тесто **raw** (`src/lib.rs:172-176`, `register_raw_recipe_serializer`) и потому «not matched». Значит блокер — не отсутствие API, а то, что `register_raw_recipe_serializer` не умеет нести колбэки.

Предложение: разрешить `register_raw_recipe_serializer(name, type, kind: Option<Box<dyn CraftingRecipeHandler>>)` — или явно задокументировать, что `farmersdelight:dough` надо регистрировать как `CraftingRecipeKind`. Новое имя хука не требуется.

- **Кто выиграет:** любой мод с `CustomRecipe` (Create, Farmer's Delight, поваренные моды).
- **Куда ложится:** ТЗ шаг 2 (порядок работ, п.4).
- **Fabric-аналог:** `fabric-recipe-api-v1` (`CustomRecipeSerializer`) — `docs/fabric-ledger.md`: **Covered in part** (через `native_ingredient`/`native_ingredient_test`), формы `CustomRecipe` — **Gap**. Ключ `fabric = "fabric-recipe-api-v1"` возможен, но с оговоркой.

### 2.6 Мутабельный использованный стак — ТЗ шаг 4

`BlockClick.stack: Option<&'a ItemStack>` (`crates/horizon-mod-vocab/src/block/types.rs:423-441`), структура `Copy`, транспорт — owned-снапшоты. Флип сковороды (`README.md:96-98`, `src/skillet.rs:41`) невозможен. ТЗ уже описывает решение (шаг 4, «Мутабельный доступ к использованному предмету»); это смена контракта `use_item_on` с `hook_version`, затрагивающая wire-кодоген (`docs/unification/step0-core-map.md` §7, риск 1).

- **Кто выиграет:** сковорода, любые «взять/подменить предмет в руке» моды (Create's wrench, Quark's pickarang).
- **Куда ложится:** ТЗ шаг 4 (порядок работ, п.6).
- **Fabric-аналог:** `fabric-events-interaction-v0` `UseBlockCallback` — возвращает `ActionResult`, стак не мутирует; в `docs/fabric-ledger.md` раздел «Event counterparts: effects, interaction, damage and chat» явно пишет «None of them is **Covered**… the result shape» → ключ `fabric` не ставится.

### 2.7 Что проверено и признано закрытым (не требует ядра)

Все 17 хуков, которые реализует порт (`src/lib.rs:160-647`), имеют аналог в каталоге MAIN; ни одного FD-специфичного хука не нужно — это же зафиксировано в `docs/design/mod-api/farmers-delight-hooks.md:1-9` («No Farmer's Delight-specific hook is proposed»). Капабилити, меню, `ItemStackHandler`, данные-мапы (`data_maps/` читаются: `crates/horizon-client-render/src/datapack.rs:738`), `neoforge:conditions`, `IBlockExtension`/`IItemExtension`-набор, диспенсеры, костная мука, тултипы, экраны, конфиги, `datapack_sync`, reload-листeners — всё есть.

---

## 3. Прямые Fabric-аналоги для предложенного (для ключа `fabric`)

| Предложение | Fabric API 26.3 | ставить ли `fabric` |
|---|---|---|
| `copy_components` (2.1) | нет отдельного API — та же ванильная loot-функция | нет |
| `villager_trade_load` (2.2) | `fabric-content-registries-v0` (регистры профессий) | нет: в `docs/fabric-ledger.md` помечено **Gap** для регистров жителей |
| `data_load_transform` (2.3) | `fabric-biome-api-v1` `BiomeModifications`/`BiomeSelectors` | только для биомной части, со статусом **Gap** |
| `block_entity_tick` + LevelRef (2.4) | нет BE-тик-события (`fabric-lifecycle-events-v1` — только серверный/мировой тик) | нет |
| `CustomRecipe` (2.5) | `fabric-recipe-api-v1` `CustomRecipeSerializer` | да, с оговоркой **Covered in part** |
| мутабельный стак в `use_item_on` (2.6) | `fabric-events-interaction-v0` `UseBlockCallback` | нет: результат не мутирует стак, статус **Gap** |

Правило заполнения то же, что у существующих пяти: `fabric = "..."` ставится только при подтверждённом совпадении момента/стороны/контракта; иначе ключ опускается, а причина уходит в `docs/fabric-ledger.md`.

---

## 4. Привязка к шагам ТЗ

| Шаг ТЗ | Что закрывает из этого аудита | Доказательство готовности |
|---|---|---|
| 1 | `add_features_by_filter`, `neoforge:compound` | §2.3; альтернатива уже в коде — `parts/biome_modifiers.rs` |
| 2 | `CustomRecipe` (тесто) | §2.5; `CraftingRecipeKind` уже с `remaining_items` |
| 3 | тики станций вне оверворлда | §2.4; риск зафиксирован в `step0-core-map.md` §7 п.2 |
| 4 | флип сковороды | §2.6; риск зафиксирован в `step0-core-map.md` §7 п.1 |
| 5 | `/reload` без игроков | `add_server_reload_listeners` уже есть — правок ядра не нужно, только перенос публикации порта |
| 6 | `copy_components`; сделки жителей | §2.1 (чистая правка `loot.rs`), §2.2 (фикс `merchant.rs:365` + опциональный хук) |
| 7 | Lua-биндинги для новых хуков | каталог `vocab` — единый источник, эмиттера Lua в `api-gen` пока нет (`step0-core-map.md` §7 п.7) |
| 8 | база под Create | §2.2/2.3/2.4 напрямую нужны Create (жидкие контейнеры, ворлдген, кинетика в Незере) |

**Итог:** из 4 блокеров, названных в ТЗ, два (`copy_components`, торговля) — не хуки, а точечные фиксы ядра (`loot.rs:1520-2007`, `merchant.rs:365`); один (`add_features_by_filter`) закрывается шагом 1 целиком; четвёртый (`CustomRecipe`) уже на 80 % покрыт `CraftingRecipeKind` и требует лишь разрешить raw-сериализатору нести колбэки. Новых имён хуков сверх NeoForge нужно ровно два: `villager_trade_load` и `data_load_transform`; остальное — `hook_version`-правки существующих.
