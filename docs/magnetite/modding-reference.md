# Справочник функций для модов

Собран из исходников мод-API скриптом `tools/modding_reference.py` (не править руками).
Здесь все хуки каталога и все методы `Host`: имя, сторона, контракт и одна строка о том,
что делает функция. Полное описание с параметрами — в rustdoc крейта `horizon-mod-api`
(`cargo doc -p horizon-mod-api --no-deps --open`). Как всё это устроено — в
[гайде по моддингу](modding.md). English: [modding-reference.en.md](modding-reference.en.md).

Хуков: **761**, методов Host: **364**.

**Контракт хука:** «слушает» — узнаёт о событии; «может отменить» — может запретить
действие; «собирает ответы» — ответы всех модов складываются; «заменяет результат» —
первый ответивший мод заменяет ванильный ответ.

## Хуки

### Глобальные хуки (`Hooks`)

#### Запуск, настройки и конфиги

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `client_setup` | клиент | слушает | `FMLClientSetupEvent` в NeoForge: клиент запустился. |
| `client_stopped` | клиент | слушает | Клиент завершил остановку после `client_stopping`. |
| `client_stopping` | клиент | слушает | Клиент закрывается: событие срабатывает один раз на игровой процесс, в потоке рендера, при завершении игры, после выхода из мира игрока и его сохранения, когда игра ещё хранит настройки и ресурсы, поэтому слушатель может прочитать их через `ClientHost` и сохранить своё клиентское состояние. |
| `component_types` | обе | собирает ответы | Зарегистрировать NBT-типы компонентов с пространством имён до загрузки содержимого. |
| `config_loading` | обе | слушает | Зарегистрированная конфигурация была прочитана (`ModConfigEvent.Loading`): `config` указывает, чья она, какой файл и его текст, который мод анализирует (обычно TOML). |
| `config_reloading` | обе | слушает | Файл загруженной конфигурации изменился на диске (`ModConfigEvent.Reloading`): его новый текст. |
| `config_unloading` | сервер | слушает | Конфигурация `Server` выгружается при остановке сервера (`ModConfigEvent.Unloading`), после `server_stopping`; следующий мир снова её читает. |
| `console_line` | сервер | слушает | Строка, записанная сервером в журнал (то, что выводит его консоль и сохраняет `logs/latest.log`): `level` — это `INFO`, `WARN` или `ERROR`, `thread` — поток, названный в строке (`Server thread`), `message` — строка без времени и потока. |
| `default_components_bound` | обе | слушает | Стандартные компоненты предметов связаны и доступны для чтения (`DefaultDataComponentsBoundEvent` NeoForge); `cause` сообщает, каким образом, как и у `tags_updated`. |
| `difficulty_changed` | сервер | слушает | Сложность сервера изменилась с `from` на `to` (`peaceful`, `easy`, `normal`, `hard`; `DifficultyChangeEvent` NeoForge). |
| `extend_poi_types` | сервер | собирает ответы | Добавьте состояния блоков к существующему типу POI (NeoForge `ExtendPoiTypesEvent`): кровать мода — к `minecraft:home`, его рабочую станцию — к `minecraft:armorer`. |
| `game_rule_changed` | сервер | слушает | Серверное правило игры `rule` (`minecraft:keep_inventory`) было установлено в `value` (`GameRuleChangedEvent`, через `GameRules.set` с сервером: `/gamerule`, экран правил игры, протокол управления, тестовая среда, `server.properties`). |
| `game_shutting_down` | обе | слушает | Игровой процесс завершает работу после остановки клиента или выделенного сервера. |
| `map_decoration_types` | обе | собирает ответы | Типы меток карты, которые добавляет мод (событие регистрации NeoForge `RegisterEvent` для `Registries.MAP_DECORATION_TYPE`): передайте по одному `MapDecorationType` на каждый тип. |
| `mod_mismatch` | обе | собирает ответы | Сохранённый список модов (кодовые и контентные моды) был прочитан при открытии мира. |
| `permissions_changed` | сервер | может отменить | Событие `PermissionsChangedEvent` NeoForge: список операторов собирается выдать игроку `player` (`name`) уровень прав `new` вместо `old` через `PlayerList.op` (`/op`, `op` в консоли, `operators/add` и `operators/set` в API управления) или `PlayerList.deop` (`/deop` и то же самое в консоли и API управления; тогда `new` равно `All`). |
| `scheduled_task` | сервер | слушает | Наступает срок задачи, запланированной модом через `schedule_task`: в начале серверного тика, когда она должна выполниться, после стартовой фазы `server_tick`, в порядке планирования задач. |
| `setting_changed` | клиент | слушает | Игрок изменил настройку `key` мода `mod_id` на её странице в разделе «Настройки > Моды» (`Registrar::register_setting`); новое значение уже сохранено, поэтому считывайте его с помощью `setting_bool` и родственных методов. |
| `whitelist_state_update` | сервер | слушает | Белый список выделенного сервера изменился на `enabled`. |

#### Регистрация и загрузка данных

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `add_attribute_tooltips` | клиент | слушает | `AddAttributeTooltipsEvent`: строки, следующие за строками атрибутов стака (`ItemStack.addAttributeTooltips`), после модификаторов, показываемых стаком и оставленных `gather_skipped_attribute_tooltips`. |
| `add_client_reload_listeners` | клиент | слушает | Зарегистрируйте клиентские слушатели перезагрузки ресурсов до того, как перезагрузка подготовит свои ресурсы (NeoForge `AddClientReloadListenersEvent`, клиентский аналог `add_server_reload_listeners`): первая загрузка в потоке загрузчика, затем каждая последующая перезагрузка — игрок изменил ресурспаки, нажал F3+T или сервер прислал свои ресурспаки. |
| `add_pack_finders` | обе | слушает | Добавьте обнаруживаемые ресурспаки или датапаки до выбора паков репозиторием (`AddPackFindersEvent`). |
| `add_section_geometry` | клиент | слушает | Дополнительная геометрия в сетках секций чанков (NeoForge `AddSectionGeometryEvent`): `register(renderer)` добавляет собственную `SectionGeometry` мода, у которой мессер запрашивает добавление квадов каждый раз при построении сетки секции в его рабочих потоках. |
| `add_server_reload_listeners` | сервер | слушает | Зарегистрируйте слушатели перезагрузки данных сервера после выбора паков. |
| `build_creative_mode_tab_contents` | клиент | собирает ответы | Стаки, которые мод добавляет во вкладку креативного режима `tab` (`minecraft:ingredients` или `<mod id>:<name>` другого мода), во время построения содержимого вкладок (событие NeoForge `BuildCreativeModeTabContentsEvent`, внутри ванильного `CreativeModeTabs.buildAllTabContents`): добавляйте каждый стак в `items` в порядке (`accept`). |
| `create_fluid_source` | сервер | слушает | Источник жидкости в `pos` (`state`) вот-вот будет засчитан при превращении текущей жидкости рядом в источник (`CreateFluidSourceEvent`, `FlowingFluid.getNewLiquid` в NeoForge): два засчитанных источника рядом с блоком над сплошным блоком или источником делают его источником. |
| `create_skull_models` | клиент | собирает ответы | Модели голов для собственных типов голов мода (`EntityRenderersEvent.CreateSkullModels` в NeoForge): добавьте по одной модели `SkullModel` для каждого типа, указав её `kind`, слой модели, из которого она запекается, и текстуру по умолчанию. |
| `create_spawn_position` | сервер | может отменить | Прежде чем новый серверный мир выберет начальную точку появления в мире (`LevelEvent.CreateSpawnPosition` в NeoForge). |
| `data_map_types` | сервер | собирает ответы | Сервер собирает свои таблицы данных, как для `data_pack_files` (первый запуск мира и каждый `/reload`): добавьте типы карт данных, которые регистрирует ваш мод (`RegisterDataMapTypesEvent` NeoForge). |
| `data_maps_updated` | обе | слушает | Карты данных реестра перезагружены на сервере или синхронизированы с клиентом (`DataMapsUpdatedEvent` NeoForge): место для построения кэшей из новых значений или их постобработки. |
| `data_pack_files` | сервер | собирает ответы | Сервер собирает свои таблицы данных из датапаков мира (`ReloadableServerResources`): открывается мир или была выполнена `/reload`; после этого строятся рецепты, теги, лут и достижения. |
| `datapack_sync` | сервер | собирает ответы | Соберите типы рецептов, когда присоединяющийся игрок уже существует, после допуска и до отправки рецептов этого игрока. |
| `gather_effect_screen_tooltips` | клиент | слушает | `GatherEffectScreenTooltipsEvent`: подсказка панели эффектов, на которую наведён курсор, рядом с инвентарём выживания или творческим инвентарём. |
| `gather_skipped_attribute_tooltips` | клиент | слушает | `GatherSkippedAttributeTooltipsEvent`: модификаторы характеристик, которые подсказка стака пропускает (`ItemStack.addAttributeTooltips`), запрашиваются один раз перед построением строк характеристик. |
| `loot_condition` | сервер | собирает ответы | Условие лута собственного типа мода: запись с именем `type` в пространстве имён вне `minecraft:`. |
| `loot_drops` | сервер | собирает ответы | Блок сломался или моб умер, а `drops` — предметы, которые вот-вот выпадут, в порядке выпадения: результат таблиц лута (после изменений `loot_table_load`), а для моба — то, что он носил, держал или переносил (предмет в пасти лисы, инвентарь аллая, пиглина или коня), но не переносимый эндерменом блок и не сохранённая экипировка медного голема, которые выпадают позже. |
| `loot_function` | сервер | собирает ответы | Функция лута собственного типа мода: запись с именем `type` в пространстве имён вне `minecraft:`, а бросок сохранил её для этого вызова. |
| `loot_table_load` | сервер | собирает ответы | Точка хука нативного мода (docs/modding/src/how-to/native-mods.md), объявленная в `hooks/*.rs` вместе с игровыми: таблица лута `id` (`minecraft:chests/simple_dungeon`, `minecraft:blocks/stone`, `minecraft:entities/zombie`) при загрузке, её JSON после датапаков, перед любым броском (хук NeoForge `LootTableLoadEvent`, хук Fabric `LootTableEvents.MODIFY`). |
| `modify_custom_spawners` | сервер | слушает | `ModifyCustomSpawnersEvent` NeoForge: мир `dimension` собирается впервые тикать свои пользовательские рассадники. |
| `modify_default_components` | обе | собирает ответы | ModifyDefaultComponentsEvent NeoForge 26.3: устанавливает или удаляет записи в `components` — карте компонентов по умолчанию для `item`, включая ванильные предметы. |
| `recipe_book_categories` | обе | слушает | Добавьте категории и ванильные отображения для пользовательских типов рецептов. |
| `recipe_property_sets` | обе | собирает ответы | Предметы, которые собственный тип рецептов мода принимает в слот станции, добавляются в наборы свойств рецептов (`RecipeManager.RECIPE_PROPERTY_SETS`): что принимает слот печи, доменной печи, коптильни, костра, кузнечного стола или зельеварки, и что клиент считает принимаемым. |
| `recipe_sources` | сервер | собирает ответы | Дополнительные контейнеры ингредиентов для серверного игрока `player`, размещающего или проверяющего рецепт в модовом меню крафта `menu` (`namespace:path`) в `pos`. |
| `recipes_received` | клиент | слушает | Рецепты сервера типа `recipe_types` получены (NeoForge `RecipesReceivedEvent`): с этого момента помощники рецептов (`native_recipes`, `native_recipes_of_type`) возвращают серверные рецепты этих типов вместо собственных рецептов окна, поэтому перестройте то, что вы на них построили. |
| `register_block_state_models` | клиент | собирает ответы | Типы определений состояний блоков, которые добавляет мод (NeoForge `RegisterBlockStateModels`, её `registerDefinition`): добавьте каждый id типа, `<mod id>:<path>`. |
| `register_block_tint_sources` | клиент | слушает | Источники оттенка блоков (NeoForge `RegisterColorHandlersEvent.BlockTintSources`, ванильная `BlockColors.register`): `register(layers, blocks)`, и грань модели с `tintindex` i умножается на слой i из списка блока; грань вне списка не тонируется. |
| `register_cauldron_fluid_content` | сервер | собирает ответы | Укажите, какую жидкость и в каком количестве содержит один из ваших блоков котла (NeoForge `RegisterCauldronFluidContentEvent`), чтобы capability жидкости (`FLUIDS`) наполняла и опустошала его так же, как ванильные котлы с водой и лавой. |
| `register_cauldron_interaction` | обе | слушает | Зарегистрируйте взаимодействия предметов с котлом, по точному предмету или по тегу предмета, на обеих сторонах до публикации снимка. |
| `register_client_commands` | клиент | собирает ответы | Собственные команды клиента (NeoForge `RegisterClientCommandsEvent`), которые выполняет `command`: добавьте по одному `Command` для каждого использования (`x <mode:word>`). |
| `register_color_resolvers` | клиент | слушает | Резолверы цвета, через которые тона блоков смешиваются на основе биомов вокруг блока (NeoForge `RegisterColorHandlersEvent.ColorResolvers`): `register(resolver)` добавляет собственный `BiomeColorResolver` мода и возвращает `ColorResolver`, который его источники тона блока передают в `BlockAndTintGetter::block_tint`. |
| `register_commands` | сервер | собирает ответы | Команды, которые нативные моды добавляют в дерево команд сервера: добавьте свои. |
| `register_configs` | обе | собирает ответы | Файлы конфигурации, которые хранят нативные моды (`registerConfig` из NeoForge): добавляйте свои, каждый определённого типа, с текстом, записываемым в `config/`, если файл отсутствует. |
| `register_debug_entries` | клиент | собирает ответы | Отладочные записи F3, которые добавляет мод (NeoForge `RegisterDebugEntriesEvent`): добавьте по одной `DebugEntry` на каждую запись, её id `<mod id>:<path>` и ванильную группу, к которой присоединяются её строки. |
| `register_debug_renderers` | клиент | собирает ответы | Отладочные рендереры в мировом пространстве, которые сейчас есть у мода (NeoForge `RegisterDebugRenderersEvent`): добавьте по одному `DebugRenderer` на каждый рендерер. |
| `register_dimension_transition_screen` | клиент | собирает ответы | Зарегистрируйте экраны загрузки один раз при запуске клиента (NeoForge `RegisterDimensionTransitionScreenEvent`). |
| `register_enchantment_damage_effects` | сервер | собирает ответы | Зарегистрируйте эффекты значений урона зачарований с пространством имён один раз до загрузки контента. |
| `register_entity_layers` | клиент | слушает | Слои, которые мод добавляет к рендерерам живых сущностей (NeoForge `EntityRenderersEvent.AddLayers`): `add(target, layer)`, и каждая живая сущность этого типа, или каждый игрок этой модели игрока, в зоне видимости вызывает `EntityRenderLayer::submit` после своей модели и ранее добавленных слоёв. |
| `register_entity_renderers` | клиент | слушает | Определяемые кодом рендереры типов сущностей (NeoForge `EntityRenderersEvent.RegisterRenderers`, `registerEntityRenderer`): `register(type, renderer)`, и каждая сущность типа `type` (`mymod:bolt`) в зоне видимости рисуется через `EntityRenderer::submit` вместо собственного рисования типа. |
| `register_entity_spectator_shaders` | клиент | собирает ответы | Зарегистрируйте эффект экрана, накладываемый при наблюдении за типом сущности, один раз при запуске клиента (NeoForge `RegisterEntitySpectatorShadersEvent`), как ванильные крипер, паук и эндермен. |
| `register_fluid_models` | клиент | собирает ответы | Модели ваших жидкостей в мире (NeoForge `RegisterFluidModelsEvent`): добавьте по одной `mod_api::FluidModel` для каждой жидкости, чей внешний вид не передаёт всё полностью. |
| `register_game_tests` | сервер | слушает | Зарегистрируйте функции тестов с пространством имён до загрузки определений тестов мира. |
| `register_gui_layers` | клиент | собирает ответы | HUD-слои, которые мод добавляет среди ванильных (NeoForge `RegisterGuiLayersEvent`): добавьте по одному `GuiLayer` для каждого слоя, его id `<mod id>:<path>` и его `GuiLayerPlacement`. |
| `register_item_models` | клиент | собирает ответы | Типы узлов моделей предметов, которые добавляет мод (NeoForge `RegisterItemModelsEvent`): добавьте каждый id типа, `<mod id>:<path>`. |
| `register_item_tint_sources` | клиент | собирает ответы | Источники тонирования предметов, которые добавляет мод (NeoForge: `RegisterColorHandlersEvent.ItemTintSources`): передайте каждый id, `<mod id>:<path>`. |
| `register_json_animation_types` | клиент | собирает ответы | Цели и интерполяции анимаций, которые добавляет мод для именования в файлах покадровой анимации (NeoForge: `RegisterJsonAnimationTypesEvent`): передайте каждый id, `<namespace>:<path>`, как `JsonAnimationType::Target` или `JsonAnimationType::Interpolation`. |
| `register_layer_definitions` | клиент | собирает ответы | Слои моделей сущностей, собранные в коде (NeoForge: `EntityRenderersEvent.RegisterLayerDefinitions`): передайте id каждого слоя как `ModelLayerLocation` вида `<mod id>:<model>#<layer>` (`mymod:golem#main`) и его `LayerDefinition`, собранный `createBodyLayer()` мода. |
| `register_load_conditions` | сервер | собирает ответы | Зарегистрируйте именованные условия один раз при загрузке мода, до допуска серверных данных. |
| `register_model_loaders` | клиент | собирает ответы | Загрузчики моделей, которые добавляет мод (NeoForge: `ModelEvent.RegisterLoaders`): передайте id каждого загрузчика, `<mod id>:<path>`. |
| `register_particle_groups` | клиент | собирает ответы | Группы отрисовки, в которых рисуются частицы мода (NeoForge: `RegisterParticleGroupsEvent`): передайте по одному `ParticleGroup` на группу, её id `<mod id>:<path>`, способ смешивания и типы частиц в ней. |
| `register_particle_providers` | клиент | слушает | Определяемые кодом провайдеры частиц для собственных типов частиц мода (NeoForge: `RegisterParticleProvidersEvent`): `register(name, provider)`, и каждая частица типа `name` (`ParticleOptions::named("mymod:spark")`) перемещается каждый тик через `ParticleProvider::tick`. |
| `register_preset_editors` | клиент | собирает ответы | Зарегистрируйте клиентские экраны для ключей пресетов мира один раз при запуске. |
| `register_range_select_item_model_property` | клиент | собирает ответы | Числовые свойства моделей предметов, которые добавляет мод (NeoForge: `RegisterRangeSelectItemModelPropertyEvent`): передайте каждый id, `<mod id>:<path>`. |
| `register_render_pipelines` | клиент | собирает ответы | Зарегистрируйте собственные пайплайны отрисовки мода один раз при запуске клиента (NeoForge: `RegisterRenderPipelinesEvent`; в 1.21.1 моды использовали `RegisterShadersEvent`): передайте каждый `RenderPipeline` в `event.pipelines`. |
| `register_runtime_sprites` | клиент | собирает ответы | Соберите статичные сгенерированные модом PNG-файлы для атласов блоков и GUI. |
| `register_select_item_model_property` | клиент | собирает ответы | Категориальные свойства моделей предметов, которые добавляет мод (NeoForge: `RegisterSelectItemModelPropertyEvent`): передайте каждый id, `<mod id>:<path>`. |
| `register_special_model_renderer` | клиент | слушает | Специальные типы моделей предметов, рисуемые модом (NeoForge: `RegisterSpecialModelRendererEvent`): `register(type, renderer)`, и каждая `minecraft:special` модель предмета, у которой объект `model` имеет тип `type` (`mymod:skillet`), рисуется через `SpecialModelRenderer::extract`. |
| `register_sprite_sources` | клиент | собирает ответы | Типы источников спрайтов атласа, которые добавляет мод (NeoForge: `RegisterSpriteSourcesEvent`): передайте id каждого типа, `<mod id>:<path>`. |
| `register_texture_atlases` | клиент | собирает ответы | Объявите атласы наряду с игровыми (`RegisterTextureAtlasesEvent`), каждый с определением атласа, перечисляющим его спрайты. |
| `register_villager_professions` | сервер | собирает ответы | Объявите профессии жителей один раз при каждой загрузке мода, до допуска мира. |
| `tags_updated` | обе | слушает | Теги изменились, и зависящие от них данные нужно перестроить (NeoForge `TagsUpdatedEvent`); `cause` сообщает как. |

#### Бой, урон и опыт

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `apply_effect_tick` | сервер | слушает | `MobEffect.applyEffectTick`: разрешённый `EffectKey` `effect` контента мода действует на `entity` в измерении `dimension` (`minecraft:overworld`). |
| `armor_hurt` | сервер | может отменить | Броня собирается получить урон прочности от удара (NeoForge `ArmorHurtEvent`, ванильный `LivingEntity.doHurtEquipment`): один раз за удар для тех деталей, что будут изнашиваться, и сначала только для шлема, когда падающий блок ударяет игрока (`hurtHelmet`). |
| `arrow_loose` | сервер | может отменить | NeoForge `ArrowLooseEvent`: игрок отпускает лук `stack` с `has_ammo` и `charge` тиков удержания. |
| `arrow_nock` | сервер | может отменить | Игрок готовится вложить стрелу в `bow` через `hand` (`BowItem.use`, NeoForge `ArrowNockEvent`): `has_ammo` указывает, нашёл ли `getProjectile` боеприпасы, или игрок в креативе. |
| `attack_entity` | сервер | может отменить | Игрок атакует сущность `entity` (`Entity.getId`) типа `kind` (`minecraft:item_frame`, `minecraft:cow`, `minecraft:player`), стоящую в `pos` в мире `dimension` (`&str`): атака `ServerGamePacketListenerImpl.handleInteract`, один раз в пределах досягаемости, там, где начинается `Player.attack`. |
| `block_screen_effect_draw` | клиент | слушает | Рисует наложение блока, о котором мод сообщил, что оно рисуется само (`custom_draw` в `extract_block_screen_effect`), перед ванильным наложением такого вида, по одному разу в кадр. |
| `client_attack_target` | клиент | может отменить | Перед тем как обычный клик ближнего боя клиента атакует сущности (не укол копьём), после хука отмены клавиши взаимодействия. |
| `consume_effect` | сервер | слушает | Эффект расходника собственного типа мода: запись `on_consume_effects` предмета относится к типу в отдельном пространстве имён, и загрузчик сохранил её для этого вызова. |
| `creeper_ignite` | сервер | слушает | Игрок поджёг крипера с помощью предмета. |
| `creeper_power` | сервер | слушает | Молния зарядила крипера. |
| `critical_hit` | сервер | слушает | `CriticalHitEvent` NeoForge: удар ближнего боя игрока по живому `target_kind` (`minecraft:zombie`, `minecraft:armor_stand`), наносящий урон, при котором `Player.attack` решает, критический ли он. |
| `debug_renderer_draw` | клиент | слушает | Отрисуйте один активный отладочный рендерер (`register_debug_renderers`); вызывается для каждого вида мира после `submit_custom_geometry`. |
| `dragon_egg_form` | сервер | может отменить | Битва с Эндер-драконом собирается разместить первое яйцо дракона в `pos`. |
| `dragon_fight_won` | сервер | заменяет результат | Битва с Эндер-драконом выиграна (`EnderDragonFight.setDragonKilled`), при каждой победе, а не только первой. |
| `effect_particle_modification` | сервер | слушает | `EffectParticleModificationEvent` NeoForge, `LivingEntity.updateSynchronizedMobEffectParticles` vanilla: вызывается для каждого активного эффекта `effect` живой сущности `entity` ( `LivingRef`, моб или игрок), включая скрытые ванилью, когда сервер собирается отправить их вихри в синхронизированных данных (`DATA_EFFECT_PARTICLES`, `DATA_EFFECT_AMBIENCE_ID`). |
| `enchant_effect` | сервер | слушает | Эффект `minecraft:post_attack` собственного типа мода для моба, в которого попал оружием игрок: запись содержит `enchanted` атакующего и `affected` жертву, её `requirements` пройдены, а `effect` указывает `type` в пространстве имён вне `minecraft:`. |
| `ender_entity_teleport` | сервер | может отменить | Эндермен или шалкер собирается телепортироваться самостоятельно (`EntityTeleportEvent.EnderEntity`). |
| `ender_pearl_teleport` | сервер | может отменить | Эндер-жемчуг серверного игрока или моба собирается телепортировать своего `owner` (`EntityTeleportEvent.EnderPearl`, `ThrownEnderpearl.onHit`). |
| `enderman_anger` | сервер | может отменить | `player` смотрит на `enderman` (`EndermanAngerEvent` NeoForge, в конце `Enderman.isBeingStaredBy`): взгляд находится в конусе эндермена, в линии видимости, и резная тыква не закрывает лицо. |
| `exp_bottle` | сервер | слушает | Флакон опыта разбился, и опыт из него был начислен. |
| `experience_orb_can_merge` | сервер | слушает | Может ли опыт-сфера `into` вобрать сферу `from` (vanilla `ExperienceOrb.canMerge`): одинаковое значение и id с разницей, кратной 40. |
| `experience_orb_merge` | сервер | может отменить | Опыт-сфера `into` (`into_value` очков, `into_count` сфер) собирается вобрать `from` (`from_value` очков, `from_count` сфер) после положительного `experience_orb_can_merge` (`from` — 0, а `from_count` — 1 для новой появляющейся сферы). |
| `explosion_detonate` | сервер | собирает ответы | Взрыв в `center` в измерении `dimension` (`minecraft:overworld`) вот-вот разрушит `blocks` и затронет `entities`: `ServerExplosion.explode` с его `toBlow`, до поражения какого-либо блока или сущности. |
| `explosion_knockback` | сервер | собирает ответы | Взрыв в `center` в измерении `dimension`, вот-вот разрушающий `blocks`, отбрасывает `entity` (None для игрока, чьё имя даёт `host.players().acting_player()`): `ServerExplosion.hurtEntities`, после расчёта отбрасывания. |
| `explosion_prime` | сервер | слушает | Серверный взрыв вот-вот начнётся в `center` с радиусом `radius` (Bukkit `ExplosionPrimeEvent`). |
| `explosion_start` | сервер | может отменить | Серверный взрыв вот-вот произойдёт (`ExplosionEvent.Start`, в `ServerLevel.explode` до `ServerExplosion.explode`): любой: ТНТ, криперы, кровати и якоря, огненные шары, иссушители, кристаллы Края и порывы ветра. |
| `extract_block_screen_effect` | клиент | может отменить | Перед отрисовкой одного из наложений блоков вида от первого лица (`ScreenEffectRenderer`), когда каждый тип показывается один раз за кадр: `Block` с `state`, блокирующим обзор, в `pos`, внутри которого находится камера; `Water` с `state` блока глаза в `pos`, пока глаз под водой; и `Fire` с стандартным `state` огня в `pos` блока игрока, пока он горит. |
| `firework_explode` | сервер | слушает | Фейерверочная ракета завершила свои эффекты взрыва. |
| `gui_render_post` | клиент | слушает | После полной обычной пакетной отрисовки GUI, включая экраны, подсказки, тосты, отладочные наложения, указатель контроллера и IME. |
| `item_expire` | сервер | слушает | Сущность выпавшего предмета на сервере достигает предельного срока жизни (`ItemEntity.tick`, NeoForge `ItemExpireEvent`). |
| `living_experience_drop` | сервер | может отменить | Умирающий моб или игрок `entity` типа `kind` вот-вот уронит `original` единиц опыта; ванильные условия проверены. |
| `mob_effect_added` | сервер | слушает | `MobEffectEvent.Added` из NeoForge, `MobEffect.onEffectStarted` из ванили: каждый `LivingEntity.addEffect` эффекта `effect` мода контента, новый он или нет, изменённый или нет, после `on_effect_added`. |
| `mob_effect_applicable` | сервер | заменяет результат | `MobEffectEvent.Applicable` из NeoForge, `LivingEntity.canBeAffected` из ванили: спрашивается везде, где игра проверяет, может ли разрешённый `EffectKey` `effect` (`minecraft:poison`) примениться к живой сущности `entity` (`LivingRef`, включая самого игрока), до того как `addEffect` примет его. |
| `mob_effect_expired` | сервер | может отменить | `MobEffectEvent.Expired` из NeoForge, `LivingEntity.tickEffects` из ванили: разрешённый `EffectKey` `effect` (`minecraft:speed`, усилитель `amplifier`) у живой сущности `entity` (`LivingRef`, включая самого игрока) достиг нуля и вот-вот исчезнет. |
| `mob_effect_remove` | сервер | может отменить | `MobEffectEvent.Remove` из NeoForge, `LivingEntity.removeEffect` из ванили: разрешённый `EffectKey` `effect` (`minecraft:speed`, усилитель `amplifier`) у живой сущности `entity` (`LivingRef`, включая самого игрока) вот-вот будет удалён любым путём через ванильный конвейер: командой или молоком у игрока, лечением зомби-жителя, вспомогательным удалением мода. |
| `on_effect_added` | сервер | слушает | `MobEffect.onEffectAdded`: `LivingEntity.addEffect` добавило `entity` (как в `native_apply_effect_tick`) эффект `effect` контента мода, которого у неё не было. |
| `pickup_xp` | сервер | может отменить | Перед тем как `player` подбирает сферу опыта `orb` в `dimension`. |
| `player_attack_cooldown_reset` | сервер | слушает | Игрок на сервере сбросил силу атаки взмахом (`handlePunch`). |
| `player_brewed_potion` | сервер | слушает | Игрок на сервере взял `stack` из слота бутылки варочной стойки (`BrewingStandMenu.PotionSlot.onTake`): `player` — его UUID, а `player_entity_id` — его сетевой id. |
| `player_change_beacon_effect` | сервер | слушает | Игрок на сервере изменил выбранные эффекты маяка после их сохранения. |
| `player_exp_cooldown_change` | сервер | слушает | Пауза подбора сфер опыта серверного игрока изменилась до `ticks`. |
| `player_pickup_arrow` | сервер | слушает | Серверный игрок подобрал стрелу после её удаления и применения подбора. |
| `player_ready_arrow` | сервер | слушает | Серверный игрок выстрелил из натянутого лука с выбранной `arrow` по клавише использования предмета или вызовом `LivingRef::release_using_item` мода. |
| `player_xp_change` | сервер | может отменить | Очки опыта серверного игрока скоро изменятся через `Player.giveExperiencePoints` (`PlayerXpEvent.XpChange` NeoForge): подбор сферы опыта после починки, награда за достижение или `/xp add ... points`. |
| `player_xp_level_change` | сервер | может отменить | Уровень серверного игрока скоро изменится через `Player.giveExperienceLevels`: один раз за каждый уровень, полученный или потерянный очками опыта, а также для наковальни, `/experience add ... levels` и `PlayerRef::give_levels`. |
| `potion_brew_post` | сервер | слушает | Зельеварка в `pos` сварила (`PotionBrewEvent.Post`, в конце `BrewingStandBlockEntity.doBrew`), или `potion_brew_pre` был отменён: `stacks` содержит её пять слотов после варки (или в том состоянии, в котором их оставили отменяющие моды), и то, что моды оставляют там, становится содержимым зельеварки. |
| `potion_brew_pre` | сервер | может отменить | Зельеварка в `pos` собирается сварить (`PotionBrewEvent.Pre`, в `BrewingStandBlockEntity.doBrew`): `stacks` — копия её пяти слотов (три бутылки, ингредиент и топливо). |
| `potion_splash` | сервер | слушает | Брошенное взрывное или выдержанное зелье применило эффекты попадания. |
| `raid_spawn_wave` | сервер | слушает | Волна набега заспавнена в `pos`; `wave` — порядковый номер волны, начинающийся с 1, а `raiders` считает рейдеров, а не мобов, на которых они едут. |
| `raid_state_change` | сервер | слушает | Серверный набег в `pos` перешёл в `state` (`triggered`, `victory`, `loss` или `stopped`). |
| `render_arm` | клиент | может отменить | Перед отрисовкой `left` или правой руки игрока этого окна от первого лица (`RenderArmEvent`, в `renderPlayerArm` и `renderMapHand`): рука пустой основной руки и руки под удерживаемой картой; не вызывается при невидимости игрока. |
| `render_fog` | клиент | слушает | Итоговый туман мира после хука вычисляемого цвета, обычных вычислений и параметра тумана Horizon, перед загрузкой. |
| `render_frame_post` | клиент | слушает | Один раз за кадр, в мире или вне его, сразу после отрисовки кадра (NeoForge: `RenderFrameEvent.Post`). |
| `render_frame_pre` | клиент | слушает | Один раз за кадр, в мире или вне его, после дополнений Horizon и непосредственно перед отрисовкой кадра (NeoForge: `RenderFrameEvent.Pre`). |
| `render_gui_post` | клиент | слушает | HUD (не рисуется при F1), после мини-карты (NeoForge: `RenderGuiEvent.Post`): разместите рамку через `corners.place` (рамки складываются в сторону от своего края) и нарисуйте её с помощью `native_fill` и `native_text`. |
| `render_gui_pre` | клиент | может отменить | Перед отрисовкой внутриигрового HUD (NeoForge: `RenderGuiEvent.Pre`): верните `Flow::Cancel`, и он не будет отрисован, как при F1: нет ванильного слоя (а значит, нет `gui_layer_pre` или `gui_layer_post`), нет `render_gui_post` и нет чата или всплывающих уведомлений. |
| `render_hand` | клиент | может отменить | Перед отрисовкой одной руки от первого лица игрока этого окна (`RenderHandEvent`, клиент): рука с предметом или картой, рука пустой основной руки и пустая вторая рука, где ваниль ничего не рисует. |
| `render_item_in_frame` | клиент | может отменить | Перед отрисовкой предмета или карты в видимой рамке с предметом. |
| `render_level_stage` | клиент | слушает | Отрисуйте геометрию мира на одной стадии вида мира (NeoForge 26.3: `RenderLevelStageEvent`), вызывается для каждого `RenderLevelStage` в порядке кадра. |
| `render_living_post` | клиент | слушает | После отрисовки живой сущности (NeoForge: `RenderLivingEvent.Post`): один раз за кадр для каждого моба и игрока в поле зрения, чей `render_living_pre` не был отменён, после его модели, ванильных слоёв и слоёв модов. |
| `render_living_pre` | клиент | может отменить | Перед отрисовкой живой сущности (NeoForge: `RenderLivingEvent.Pre`): один раз за кадр для каждого моба и игрока в поле зрения (для отсечённых кадром не вызывается), до его позы, модели и слоёв. |
| `render_name_tag` | клиент | слушает | Видимость именной таблички и необязательный замещающий текст. |
| `render_needs` | клиент | собирает ответы | Вызывается при построении и отрисовке кадра: укажите в `RenderNeeds`, что нужно моду, включая запуски сущностей и глубину мира без шейдер-пака. |
| `render_player_post` | клиент | слушает | После отрисовки игрока (NeoForge: `RenderPlayerEvent.Post`): один раз за кадр для каждого игрока, чей `render_player_pre` не был отменён, после его `render_living_post` или сразу после его `render_living_pre`, если был отменён только он. |
| `render_player_pre` | клиент | может отменить | Перед отрисовкой игрока (NeoForge: `RenderPlayerEvent.Pre`): один раз за кадр для каждого другого игрока в поле зрения и для игрока этого окна от третьего лица (а также при виде спереди на F5), до его `render_living_pre`. |
| `render_tooltip_gather_components` | клиент | может отменить | `RenderTooltipEvent.GatherComponents`: элементы подсказки `stack` (`ItemStack.getTooltipLines` как текстовые строки, затем компонент изображения, к которому ответила часть `tooltip_component` его предмета) после запуска `item_tooltip` и перед любыми переносами или измерениями. |
| `render_tooltip_pre` | клиент | может отменить | `RenderTooltipEvent.Pre`: подсказка, уже разложенная и ожидающая размещения и отрисовки. |
| `render_tooltip_texture` | клиент | слушает | `RenderTooltipEvent.Texture`: вид рамки подсказки, которую вот-вот нарисуют. |
| `screen_render_background` | клиент | слушает | Открытый экран собирается нарисовать своё содержимое (`ScreenEvent.Render.Background`, после ванильного `renderBackground` и перед `render`): здесь рисуйте то, что должно располагаться под виджетами, предметами и подсказками экрана. |
| `screen_render_foreground` | клиент | слушает | Открытый экран нарисовал своё содержимое (`ScreenEvent.Render.Foreground`, точка в ванили перед отложенными подсказками и оверлеем предтекстового ввода). |
| `screen_render_inventory_mob_effects` | клиент | может отменить | Перед панелями эффектов рядом с инвентарями выживания и творчества (`ScreenEvent.RenderInventoryMobEffects`, ванильный `EffectsInInventory.extractRenderState`): `available_space` — это ширина в GUI-пикселях справа от контейнера, не менее 32 (если нет эффектов для отображения или места меньше, ваниль ничего не рисует, и хук не вызывается). |
| `screen_render_post` | клиент | слушает | Открытый экран нарисовал себя и свои подсказки (`Screen.extractRenderStateWithTooltipAndSubtitles`, точка вызова `ScreenEvent.Render.Post`): рисуйте поверх него с помощью `native_fill`, `native_sprite` и `native_text`, `screen.rect` — панель его контейнера (или всё окно), а `screen.id` — сам экран. |
| `screen_render_pre` | клиент | может отменить | Перед тем как открытый экран рисует себя (`ScreenEvent.Render.Pre`, ванильный `Screen.renderWithTooltips`): рисуйте, чтобы разместить под ним что-то, или установите `canceled`, и экран не будет нарисован, равно как и его `screen_render_background` или `screen_render_foreground`. |
| `should_apply_effect_tick_this_tick` | сервер | слушает | `MobEffect.shouldApplyEffectTickThisTick` для эффекта `effect` контентного мода (это `EffectKey` для `ruby:bleeding`), каждый тик, когда эффект есть у игрока или моба, в `MobEffectInstance.tickServer` перед обратным отсчётом: `tick_count` — оставшиеся тики (счётчик тиков сущности, если эффект бесконечен). |
| `sweep_attack` | обе | может отменить | NeoForge `SweepAttackEvent`: после уронного удара игрока ближнего боя по `target` (`minecraft:zombie`, `minecraft:oak_boat`) решается, произойдёт ли взмах после `critical_hit`. |
| `target_hit` | сервер | слушает | Снаряд попал в целевой блок в `pos`, который теперь выдаёт сигнал мощностью `strength`. |
| `tnt_primed` | сервер | может отменить | TNT в `pos` вот-вот подожжена (primed): кресалом или огненным зарядом, огнём, сигналом редстоуна, стрелой или кресалом/предметом TNT раздатчика. |
| `village_siege` | сервер | может отменить | Осада деревни нашла позицию спавна на сервере. |
| `wind_burst` | сервер | собирает ответы | До того, как порыв ветра взаимодействует с блоками, `wind_burst(dimension, center, blocks)` получает перемешанные позиции-кандидаты. |
| `xp_orb_targeting` | сервер | слушает | Сфера опыта ищет игрока, за которым будет следовать (NeoForge `XpOrbTargetingEvent`, `ExperienceOrb.followNearbyPlayer`), на сервере. |

#### Игроки

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `calculate_player_turn` | клиент | слушает | Чувствительность мыши и параметр кинематографической камеры, считываемые поворотом мыши зажатого игрока в этом кадре (`MouseHandler.turnPlayer`). |
| `can_player_sleep` | сервер | слушает | Серверный игрок пытается уснуть в кровати, головная часть которой находится в `pos` (событие NeoForge `CanPlayerSleepEvent`, ванильное `ServerPlayer.startSleepInBed`). |
| `client_player_change_game_type` | клиент | слушает | `ClientPlayerChangeGameTypeEvent` в NeoForge: наблюдение за изменением типа игры локального игрока. |
| `client_player_clone` | клиент | слушает | После замены игрока на клиенте при возрождении или смене измерения (`ClientPlayerNetworkEvent.Clone` в NeoForge). |
| `client_player_logging_in` | клиент | слушает | Клиент принял вход в игру и инициализировал мир и идентификацию игрока (`ClientPlayerNetworkEvent.LoggingIn` в NeoForge). |
| `client_player_logging_out` | клиент | слушает | Перед разрывом соединения клиента (`ClientPlayerNetworkEvent.LoggingOut` в NeoForge). |
| `player_advancement_done` | сервер | слушает | Игрок на сервере завершил достижение после записи прогресса. |
| `player_advancement_progress` | сервер | слушает | Критерий достижения игрока на сервере был выдан или отозван после изменения его прогресса и до любых наград за завершение. |
| `player_arm_swing` | сервер | слушает | Сервер обработал взмах рукой гостя. |
| `player_bed_enter` | сервер | может отменить | Игрок на сервере собирается лечь в кровать после успешных проверок сна. |
| `player_bed_leave` | сервер | слушает | `PlayerWakeUpEvent`: игрок на сервере встаёт с кровати в `pos` (`Player.stopSleepInBed`); событие приходит первым, когда он ещё считается спящим там. |
| `player_break_speed` | обе | может отменить | Скорость разрушения игроком блока в `pos` может быть изменена или отменена. |
| `player_bucket_entity` | сервер | слушает | Игрок на сервере поймал моба в ведро после его удаления и выдачи наполненного ведра. |
| `player_bundle_item_select` | сервер | слушает | Игрок на сервере выбрал предмет в свёртке в `slot` по `index`. |
| `player_changed_dimension` | сервер | слушает | `player` (`name`) сменил измерение (событие `PlayerChangedDimensionEvent` NeoForge): `from` и `to` — идентификаторы измерений (`minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`). |
| `player_client_loaded_world` | сервер | слушает | Игрок на сервере завершил загрузку мира вокруг себя после входа, возрождения или смены измерения. |
| `player_clone` | сервер | слушает | Данные игрока были скопированы в его нового игрока (событие `PlayerEvent.Clone` NeoForge, из `ServerPlayer.restoreFrom`): `was_death` равно true после смерти и false при выходе из Энда после титров (`respawn` с `keepAllPlayerData`). |
| `player_command` | сервер | может отменить | Команда, которую ввёл игрок (событие `CommandEvent` NeoForge, до выполнения `Commands.performCommand`), без слэша: любая команда — ванильная и собственных команд модов, в том числе та, которую игрок не может выполнить. |
| `player_commands_sent` | сервер | собирает ответы | Дерево команд, которое сервер отправляет игроку (стадия «commands sent to a player» в NeoForge и Paper): при подключении и при изменении его уровня прав (`/op`, `/deop`, настройки локальной сети). |
| `player_container_close` | сервер | слушает | `player` (`name`) закрыл меню, о котором сообщил `player_container_open` (событие `PlayerContainerEvent.Close` NeoForge, отправляется в `ServerPlayer.doCloseContainer` после `removed` меню): он закрыл его, вышел из досягаемости, погиб, сменил мир или открыл другое меню. |
| `player_container_open` | сервер | слушает | `player` (`name`) открыл меню (событие `PlayerContainerEvent.Open` NeoForge, отправляется, когда `ServerPlayer.openMenu` устанавливает новый `containerMenu`): любое меню — ванильное или от мода. |
| `player_deep_sleep` | сервер | слушает | Игрок на сервере спит в кровати уже 100 тиков и теперь считается крепко спящим для проверки пропуска ночи. |
| `player_destroy_item` | сервер | слушает | Предмет `item` серверного игрока `player` (`name`) сломался в слоте `slot` (`mainhand`, `offhand`, `head`, `chest`, `legs`, `feet`, `body` или `saddle`): `LivingEntity.onEquippedItemBroken`, событие Bukkit `PlayerItemBreakEvent`. |
| `player_destroyed_block` | сервер | слушает | Ломание игрока удалило блок `state` из `pos`: `ServerPlayerGameMode.destroyBlock` вызывает `Block.destroy(level, pos, state)` только после успешного `Level.removeBlock`, перед выдачей блока обратно креативному игроку и дропом `playerDestroy`. |
| `player_died` | сервер | слушает | `player` умер: там, где начинается `ServerPlayer.die`, с ключом сообщения о смерти, если игра его знает (`death.attack.lava`; отсутствует для сообщения, построенного из сражения). |
| `player_edit_book` | сервер | может отменить | Пакет редактирования или подписания книги серверным игроком вот-вот будет применён. |
| `player_egg_throw` | сервер | слушает | Серверный игрок бросил яйцо после запуска снаряда и применения использования предмета. |
| `player_elytra_boost` | сервер | слушает | Серверный игрок использовал ракету фейерверка для ускорения во время полёта на элитре после запуска и расходования ракеты. |
| `player_enchant_item` | сервер | слушает | Серверный игрок зачаровал `item` на столе зачарований (`EnchantmentMenu.clickMenuButton`): `enchantments` — это зачарования, которые предмет получил, по порядку, в виде (id, level). |
| `player_fish` | сервер | может отменить | Удочка серверного игрока с `state`: 0 — заброс (`FishingRodItem.use`, выбрасывающий поплавок), 1 — поклёвка рыбы (`FishingHook.catchingFish`), 2 — свёртывание лески при поклёвке, дающее улов, 3 — свёртывание лески без улова, 4 — свёртывание лески, когда поплавок зацепился за сущность или предмет (аналогично `PlayerFishEvent` в Bukkit; `ItemFishedEvent` в NeoForge происходит позже, в `FishingHook.retrieve`). |
| `player_flower_pot_manipulate` | сервер | слушает | Серверный игрок посадил растение в цветочный горшок после сохранения состояния блока горшка с растением и расхода предмета. |
| `player_flyable_fall` | сервер | слушает | Событие NeoForge `PlayerFlyableFallEvent`: серверный игрок `player` (`name`) приземлился после падения на `distance` блоков с множителем урона `multiplier`, пока ему было разрешено летать (креативный или наблюдательный игрок шёл), поэтому `Player.causeFallDamage` завершился рано и урон не был нанесён. |
| `player_game_mode_changed` | сервер | может отменить | Игровой режим серверного игрока вот-вот изменится (событие NeoForge `PlayerChangeGameModeEvent`): с `from` на `to`, оба значения — общий `GameMode`, используемый командами и `PlayerRef`. |
| `player_handshake` | сервер | может отменить | Рукопожатие соединения, которое вот-вот войдёт или будет передано внутрь (событие Paper `PlayerHandshakeEvent`), до начала входа и до проверки версии протокола, пересылки прокси, имени, белого списка или банов. |
| `player_harvest_check` | обе | собирает ответы | Возможность игрока добывать блок в `pos` может быть изменена. |
| `player_heart_type` | клиент | слушает | Какие сердца полоса здоровья локального игрока рисует в этом кадре (событие NeoForge `PlayerHeartTypeEvent` из `Gui.HeartType.forPlayer`): `kind` приходит как выбор ванили (отравление, иначе иссушение, иначе заморозка при полной заморозке, иначе обычное); установите его, чтобы рисовать другой тип. |
| `player_input` | сервер | слушает | Гость отправил текущие флаги `PlayerInput` сервера после их применения на хаб. |
| `player_insert_lectern_book` | сервер | слушает | Серверный игрок вставил книгу в кафедру после изменения блок-сущности и инвентаря. |
| `player_inventory_changed` | сервер | слушает | `player_inventory_changed(player, stacks)`: инвентарь серверного игрока изменился в момент проверки ванильного `InventoryChangeTrigger` (проверка на тик нашла другой предмет или количество в слоте), вызывается до любого критерия и независимо от наличия в дереве достижений `inventory_changed`. |
| `player_item_cooldown` | сервер | может отменить | Время восстановления предмета вот-вот начнётся у серверного игрока. |
| `player_item_frame_change` | сервер | слушает | Серверный игрок изменил предметную рамку после изменения её содержимого или поворота. |
| `player_item_mend` | сервер | слушает | Починка восстановила предмет серверного игрока за счёт опыта из сферы. |
| `player_item_smithed` | сервер | слушает | `PlayerEvent.ItemSmithedEvent`: серверный игрок забрал `result` из кузнечного стола (`SmithingMenu.onTake` или `onQuickCraft` при shift-клике): `player` — его UUID, `player_entity_id` — его сетевой id, `inputs` — шаблон, основа и дополнение в том виде, в котором они были до использования при взятии (`None` для пустого слота). |
| `player_jump` | сервер | слушает | После принятого движения гостя сервер заставил его прыгнуть с земли. |
| `player_kicked` | сервер | слушает | Сетевого или фиктивного гостя выгнали после того, как хаб записал сформированную `reason`. |
| `player_launch_projectile` | сервер | слушает | Серверный игрок запустил метательный предмет после его создания. |
| `player_lectern_page_change` | сервер | слушает | Серверный игрок изменил страницу кафедры после изменения её страницы и импульса редстоуна. |
| `player_level_change` | сервер | слушает | Уровень опыта серверного игрока изменился после применения нового уровня. |
| `player_load_from_file` | сервер | слушает | Игрок был загружен из сохранения мира (`PlayerEvent.LoadFromFile` в NeoForge, `PlayerList.loadPlayerData`): его `.dat` был прочитан и применён, но он ещё не помещён в мир. |
| `player_logged_in` | сервер | слушает | `player` (`name`) зашёл в игру (`PlayerLoggedInEvent` в NeoForge): помещён в мир и перечислен в `host.players().server_players()`, поэтому `PlayerRef::send_payload` доходит до него. |
| `player_logged_out` | сервер | слушает | `player` (`name`) вышел из игры (`PlayerLoggedOutEvent` в NeoForge) любым способом: потеря соединения, кик, остановка или сбой сервера. |
| `player_login` | сервер | может отменить | Игрок, чей вход сервер принял (стадия `PlayerNegotiationEvent` в NeoForge): проверки имени, белого списка, бана и заполненности сервера пройдены, и игрок занимает место, но его сохранённое состояние ещё не загружено и он ещё не помещён в мир. |
| `player_login_profile` | сервер | слушает | Игровой профиль игрока при входе, до отправки сервером подтверждения входа: UUID и имя, подтверждённые при входе, и свойства, которые будут отправлены. |
| `player_loom_pattern_select` | сервер | слушает | Серверный игрок выбрал узор ткацкого станка после того, как меню ткацкого станка приняло нажатие кнопки. |
| `player_map_filled` | сервер | слушает | Серверный игрок заполнил пустую карту после создания заполненной карты и её сохранённых данных. |
| `player_move` | сервер | может отменить | Отчёт о движении игрока вот-вот будет применён: он перейдёт с `from` на `to` или повернётся с `from_rotation` на `to_rotation` (`[yaw, pitch]`). |
| `player_name_entity` | сервер | слушает | Серверный игрок назвал моба с помощью именной бирки после изменения его имени и отсутствия тайм-аута, а также расхода бирки. |
| `player_name_format` | обе | собирает ответы | Отображаемое имя игрока было отформатировано. |
| `player_open_sign` | сервер | слушает | Серверный игрок открыл табличку для редактирования после создания состояния редактора. |
| `player_passage_cross` | сервер | может отменить | Клиент `player` пересёк `passage` (arismoko/horizon#3775), один из проходов мира игрока, из `from` (его ноги до этого), и сервер вот-вот последует за ним: переход в `from` прошёл проверки движения, оба конца отслеживаются для игрока, а сервер ещё не определил, где игрок выйдет. |
| `player_pick_item` | сервер | может отменить | Серверный игрок взял предмет из блока или сущности до того, как он попадёт в хотбар. |
| `player_pong` | сервер | слушает | Клиент гостя ответил на пинг (`ServerboundPongPacket`, ванильный `ServerGamePacketListenerImpl.handlePong`): `id` — число, которое содержал пинг. |
| `player_recipe_book_click` | сервер | слушает | Книга рецептов серверного игрока запросила размещение известного рецепта в сетке крафта. |
| `player_recipe_book_settings_change` | сервер | слушает | Авторитетная пара книги рецептов серверного игрока изменилась. |
| `player_recipe_discovered` | сервер | может отменить | Рецепт скоро будет открыт серверному игроку. |
| `player_respawn_position` | сервер | слушает | `PlayerRespawnPositionEvent`: место, где `player` возрождается, когда сервер его определит: кровать или якорь, у которых он встаёт (обратившись к ним), или свободный столбец точки возрождения мира. |
| `player_respawned` | сервер | слушает | `player` (`name`) возродился (`PlayerRespawnEvent` NeoForge): в конце `PlayerList.respawn`, когда игрок размещён, его здоровье, сытость и опыт сброшены, а состояние заменено. |
| `player_riptide` | сервер | слушает | Серверный игрок был запущен трезубцем с заговором течения после применения скорости, вращающей атаки и подъёма с земли. |
| `player_save_to_file` | сервер | слушает | Игрок был сохранён в хранилище мира (`PlayerEvent.SaveToFile`, `PlayerList.save` NeoForge): когда игрок выходит или мир закрывается, когда выделенный или встроенный сервер сохраняет своих гостей, и для владельца встроенного сервера, когда мир сохраняет его. |
| `player_set_spawn` | сервер | может отменить | `PlayerSetSpawnEvent` NeoForge: серверному игроку вот-вот будет установлена точка возрождения (`ServerPlayer.setRespawnPosition`) — по одному разу на каждый вызов ванили: каждый сон в кровати (в том числе в той же, которая обновляет её угол), якорь, который становится новой точкой, `/spawnpoint`, и копирование новому игроку, когда игрок возрождается и его блок возрождения был найден (`pos` — none для игрока без точки). |
| `player_shear_entity` | сервер | может отменить | Игрок собирается остричь моба. |
| `player_shield_disable` | сервер | слушает | Щит серверного игрока был отключён отбитой ближней атакой. |
| `player_spawn_phantoms` | сервер | заменяет результат | `PlayerSpawnPhantomsEvent`: спавнер фантомов собирается решить, пытаться ли заспавнить фантомов рядом с серверным игроком, который не наблюдатель, до проверок сложности и отдыха. |
| `player_sprint` | сервер | слушает | Спринт серверного игрока начался или остановился: его клиент отправил `START_SPRINTING` или `STOP_SPRINTING` (клавиша спринта, двойное нажатие вперёд или отпускание), и игрок на сервере теперь `sprinting` (`setSprinting` ванили в `handlePlayerCommand`). |
| `player_start_spectating_entity` | сервер | слушает | Серверный игрок начал наблюдать за `entity_id` после перемещения камеры на него (`ServerPlayer.setCamera` с новой целью: действие наблюдателя или `/spectate`) и уведомления клиента. |
| `player_start_tracking` | сервер | слушает | Клиент начал отслеживать сущность (`PlayerEvent.StartTracking` NeoForge, отправляется в конце `ServerEntity.addPairing`): `player` (`name`) теперь получает `entity` (её ID, `uuid` — её UUID, `kind` — её тип, `minecraft:zombie`) и её изменения. |
| `player_stat_award` | сервер | может отменить | Серверному игроку скоро будет начислена статистика до изменения его счётчика и критериев таблицы результатов. |
| `player_stays_on_ground_surface` | обе | слушает | Отступ от края при приседании (`Player.maybeBackOffFromEdge`) проверяет, остаётся ли игрок на поверхности земли (`Player.isStayingOnGroundSurface`, ванильно: зажата клавиша Shift). |
| `player_stonecutter_recipe_select` | сервер | слушает | Серверный игрок выбрал рецепт камнереза после того, как меню приняло его кнопку. |
| `player_stop_spectating_entity` | сервер | слушает | Серверный игрок перестал наблюдать за `entity_id` после возвращения камеры к игроку. |
| `player_stop_tracking` | сервер | слушает | Клиент прекратил отслеживать сущность (`PlayerEvent.StopTracking` NeoForge, отправляется в конце `ServerEntity.removePairing`): она вышла за пределы дистанции, покинула мир или умерла, либо `player` (`name`) вышел, сменил мир или возродился, что останавливает отслеживание всего, за чем он следил. |
| `player_stop_using_item` | сервер | слушает | Серверный игрок отпустил клавишу использования предмета, или это сделал `LivingRef::release_using_item` мода, до завершения использования предмета. |
| `player_swap_hands` | сервер | может отменить | Вызывается перед обменом рук у не-наблюдательного игрока. |
| `player_swap_with_equipment_slot` | сервер | слушает | Серверный игрок поменял местами основную и вторую руки после изменения обоих слотов инвентаря. |
| `player_switch_hotbar_slot_post` | сервер | слушает | `PlayerSwitchHotbarSlotEvent.Post`: серверный игрок переключился со слота хотбара `old_slot` на `new_slot` (0–8) после изменения выбранного слота и остановки использования предмета в основной руке. |
| `player_switch_hotbar_slot_pre` | сервер | может отменить | `PlayerSwitchHotbarSlotEvent.Pre`: серверный игрок переключается со слота хотбара `old_slot` на `new_slot` (0–8) после проверки диапазона слотов и до изменения выбранного слота. |
| `player_tab_list_name_format` | сервер | собирает ответы | ServerPlayer.getTabListDisplayName: имя, которое показывает Tab list для `player`; None — чтобы клиент выбрал его сам. |
| `player_take_lectern_book` | сервер | слушает | Серверный игрок взял книгу с кафедры после изменения меню и блок-сущности. |
| `player_teleport_end_gateway` | сервер | может отменить | Серверный игрок собирается пройти через врата Энда на `gateway`. |
| `player_teleported` | сервер | слушает | Команда телепортировала серверного игрока (`EntityTeleportEvent` NeoForge для игрока): `/tp` и `/teleport`, в его мире или в другой. |
| `player_tick_post` | обе | слушает | После тика игрока на обеих логических сторонах (NeoForge 26.3 `PlayerTickEvent.Post`). |
| `player_tick_pre` | обе | слушает | До тика игрока на обеих логических сторонах (NeoForge 26.3 `PlayerTickEvent.Pre`). |
| `player_toggle_entity_age_lock` | сервер | слушает | Серверный игрок переключил блокировку возраста моба, способного взрослеть, золотым одуванчиком после изменения блокировки и использования предмета. |
| `player_toggle_flight` | сервер | может отменить | Клиент игрока начал (`flying`) или прекратил полёт (`handlePlayerAbilities` ванили, учитывающий это только для игрока, который может летать: креативный или наблюдатель); хук вызывается один раз на каждое изменение на сервере, с заменённым состоянием игрока, чтобы `PlayerRef::tell` и другие помощники обращались к нему. |
| `player_toggle_sneak` | сервер | слушает | Клиент гостя изменил приседание в пакете ввода игрока после применения нового состояния хаба. |
| `player_totem` | сервер | слушает | Тотем бессмертия спас серверного игрока `player` (`name`): полученный урон должен был убить его, и тотем в любой руке сработал (`LivingEntity.checkTotemDeathProtection`; `EntityResurrectEvent` Bukkit); хук вызывается прямо перед использованием тотема и применением его эффектов. |
| `player_trade` | сервер | слушает | Серверный игрок завершил сделку с жителем или странствующим торговцем. |
| `player_unleash_entity` | сервер | слушает | Серверный игрок снял поводок с сущности после его очистки. |
| `player_velocity` | сервер | слушает | Сервер применил `velocity` игроку при синхронизации изменившегося движения с его клиентом. |
| `spread_players_teleport` | сервер | может отменить | Серверная сущность вот-вот будет помещена командой `/spreadplayers` (NeoForge `EntityTeleportEvent.SpreadPlayersCommand`, ванильный `SpreadPlayersCommand.setPlayerPositions`): один раз для каждой сущности, после того как распределение вычислит её место. |

#### Сущности и мобы

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `animal_tame` | сервер | может отменить | Игрок `tamer` (UUID) и `tamer_entity_id` собирается приручить моба `entity` типа `kind` (`AnimalTameEvent`): бросок на приручение прошёл успешно (кость для волка, рыба для кота, семена для попугая или пища для наutilus; лошадь, осёл, мул или лама, на которой едет игрок). |
| `baby_entity_spawn` | сервер | может отменить | Два животных разведены, и сейчас появится детёныш (NeoForge `BabyEntitySpawnEvent`, ванильный `Animal.spawnChildFromBreeding`): `parent` и `partner` — идентификаторы сущностей, `player` и `player_entity_id` — UUID и идентификатор сущности игрока, который скормил им еду для любви (`getCausedByPlayer`, None, если никого), `kind` — тип детёныша (у лошади и осла появляется мул). |
| `block_entity_type_add_blocks` | обе | собирает ответы | Расширяет допустимые блоки существующих типов (событие NeoForge `BlockEntityTypeAddBlocksEvent`). |
| `block_shear_entity` | сервер | слушает | Раздатчик остриг `entity` типа `kind` на блоке перед его собственным блоком в `pos`, после эффектов стрижки. |
| `custom_spawner_tick` | сервер | слушает | `CustomSpawner.tick` для рассадника, который мод добавил через `modify_custom_spawners`: мир `dimension` тикает рассадник `key` один раз за тик в порядке списка, а `spawn_enemies` получает значение `ServerLevel.isSpawningMonsters` (правило `spawn_monsters`, и режим не мирный). |
| `enchanted_entity_loot` | сервер | слушает | Таблица лута сущности считывает уровень зачарования (`EnchantedEntityLootEvent` NeoForge): вызывается во время броска, каждый раз при вычислении функции `enchanted_count_increase` или условия `random_chance_with_enchanted_bonus`, для смерти и для `/loot ... kill`. |
| `entity_break_door` | сервер | слушает | Моб сломал дверь в `pos`. |
| `entity_breed` | сервер | слушает | Два моба завершили действие разведения. |
| `entity_change_block` | сервер | слушает | Моб изменил блок в `pos` после применения нового состояния. |
| `entity_change_block_pre` | сервер | может отменить | Моб или игрок собирается изменить блок в `pos`: затаптывание при посадке (farmland → dirt, разрушение яйца черепа), до обновления соседних блоков и событий. |
| `entity_constructing` | обе | слушает | Создаётся новая сущность (NeoForge `EntityEvent.EntityConstructing`, vanilla `Entity.<init>`): `kind` — её тип (`minecraft:zombie`, `minecraft:player` для игрока), `holder` хранит её вложения. |
| `entity_drop_item` | сервер | слушает | Моб собирается выбросить `count` `item` в `pos` (Bukkit `EntityDropItemEvent`). |
| `entity_enter_block` | сервер | слушает | Пчела влетела в улей в `pos` и была туда сохранена. |
| `entity_enter_love_mode` | сервер | слушает | Моб вошёл в режим любви после взаимодействия с игроком. |
| `entity_entering_section` | обе | слушает | `entity` переместилась из 16x16x16 секции `old_section` в `new_section` (NeoForge `EntityEvent.EnteringSection`, vanilla `EntitySectionStorage` коллбэки при `Entity.setPosRaw`). |
| `entity_interact` | обе | заменяет результат | Игрок совершает правый клик в мире `dimension` (`&str`) по сущности `entity` (`Entity.getId`) типа `kind` (`minecraft:cow`) с помощью `hand` (`Hand::Main` или `Hand::Off`) с предметом `stack` (в пустой руке — ничего): это точка старта `Player.interactOn`, до собственного `interact` сущности и `interactLivingEntity` предмета. |
| `entity_interact_specific` | сервер | может отменить | Правый клик по сущности с точкой клика: `entity_interact_specific` получает сущность, её тип, удерживаемый стак и точку клика, измеренную от позиции сущности, до `native_interact_entity` (аналог NeoForge `PlayerInteractEvent.EntityInteractSpecific`); перехват взмахивает рукой и завершает клик. |
| `entity_invulnerability_check` | обе | слушает | Определяет, неуязвима ли сущность к этому конкретному источнику урона (`Entity.isInvulnerableTo`). |
| `entity_join_level` | сервер | может отменить | Сущность присоединяется к серверному миру `dimension` (NeoForge `EntityJoinLevelEvent`): vanilla `PersistentEntitySectionManager.addEntity`. |
| `entity_leave_level` | сервер | слушает | Сущность покидает серверный мир `dimension` (NeoForge `EntityLeaveLevelEvent`): vanilla `onTrackingEnd`, когда она удаляется по любой причине (подобрана, объединена, деспавн, сгорание, взрыв, убийство, забрана воронкой или мобом, прошла через портал) или при разгрузке чанка, включая закрытие мира, когда в нём никого нет. |
| `entity_mob_griefing` | сервер | слушает | Может ли `entity` изменять мир? |
| `entity_mount` | сервер | может отменить | Сущность садится на транспорт или слезает с него (NeoForge `EntityMountEvent`, публикуется из `Entity.startRiding` и `removeVehicle`): `entity` типа `kind` (`minecraft:player`, `minecraft:zombie`) в серверном мире `dimension`, садящаяся (`mounting`) или слезающая с `vehicle` типа `vehicle_kind` (`minecraft:oak_boat`, `minecraft:minecart`, `minecraft:horse`, `minecraft:cushion`). |
| `entity_move` | сервер | может отменить | Один физический вызов `Entity.move` (arismoko/horizon#2414, дизайн arismoko/horizon#2308, решение мейнтейнера по той же issue): сущность `entity` типа `kind` (`minecraft:zombie`) собирается сместиться на `delta` (блоков за тик) в связи с `cause` — самой себя, игрока за позиционным пакетом, поршня, крышки шалкерового ящика, открывающейся крышки моба-шалкера. |
| `entity_outline` | клиент | слушает | Наличие светящегося контура у сущности, рисуемой клиентом, и его цвет (`Entity.isCurrentlyGlowing` и `getTeamColor`, переопределения свечения и цвета команды в Fabric и NeoForge). |
| `entity_place` | сервер | может отменить | `BlockEvent.EntityPlaceEvent` (и `EntityMultiPlaceEvent`, расширяющий его): `placer` устанавливает блок `state` в `pos` напротив `placed_against` до записи. |
| `entity_portal` | сервер | слушает | Серверная сущность завершила ожидание в портале и начинает покидать `dimension` через `pos`. |
| `entity_pose_change` | сервер | слушает | Синхронизируемая поза гостя изменилась с `from` на `to`. |
| `entity_shoot_bow` | сервер | слушает | Лук или арбалет игрока выстрелили стрелой с `force` после попадания снаряда в мир. |
| `entity_size` | обе | собирает ответы | Размер сущности был пересчитан (NeoForge `EntityEvent.Size`, vanilla `Entity.refreshDimensions`): её поза изменилась на `pose`, она выросла, изменился её `minecraft:scale` или она новая. |
| `entity_struck_by_lightning` | сервер | может отменить | Молния собирается ударить `entity` (NeoForge `EntityStruckByLightningEvent`, до `Entity.thunderHit`): один раз на удар для каждой сущности, чей бокс пересекает бокс удара (3 блока вокруг, 9 вверх), включая игроков. |
| `entity_tame` | сервер | слушает | Моб был приручен после установки владельца и постоянного состояния. |
| `entity_teleport` | сервер | слушает | Серверная сущность была перемещена командой телепортации в пределах того же мира после изменения её позиции. |
| `entity_tick_listened` | сервер | собирает ответы | Слушает ли мод `entity_tick_pre` и `entity_tick_post`: мод, перехватывающий их, перехватывает и этот, и устанавливает `listened` (HEAD, `*listened = true`). |
| `entity_tick_post` | сервер | слушает | Сущность `entity` типа `kind` оттикала в серверном мире `dimension` (NeoForge `EntityTickEvent.Post`): для каждой сущности, пропущенной `entity_tick_pre`, после её тика, а также для умершей или удалённой в тике. |
| `entity_tick_pre` | сервер | может отменить | Сущность типа `kind` (`minecraft:zombie`, `minecraft:item`) с ID `entity` собирается оттикать в серверном мире `dimension` (NeoForge `EntityTickEvent.Pre`, публикуемый вокруг `Entity.commonTick` и `tick` в `ServerLevel.tickNonPassenger`). |
| `entity_toggle_glide` | сервер | слушает | Состояние планирования гостя изменилось на сервере. |
| `entity_travel_to_dimension` | сервер | может отменить | `entity` собирается перейти в мир `dimension` (NeoForge `EntityTravelToDimensionEvent`) до перемещения чего-либо из неё: сущность ещё находится там, где была, в своём мире. |
| `entity_unleash` | сервер | слушает | Поводок привязанного моба был снят до обработки выпадения поводка (Bukkit `EntityUnleashEvent`). |
| `finalize_spawn` | сервер | может отменить | NeoForge `FinalizeSpawnEvent`: новый моб типа `kind` находится в `pos`, а `Mob.finalizeSpawn` (снаряжение, характеристики, варианты, собственные настройки класса) вот-вот выполнится по причине `reason`. |
| `hears_projectile_impacts` | сервер | собирает ответы | Слышат ли моды удары снарядов (`native_projectile_impact`): задайте `hears`. |
| `item_despawn` | сервер | слушает | Сущность выпавшего предмета достигла срока жизни в 6000 тиков и готова быть удалена (Bukkit `ItemDespawnEvent`). |
| `item_spawn` | сервер | слушает | Сущность выпавшего предмета была успешно добавлена в серверный мир. |
| `living_breathe` | сервер | слушает | Дыхание сущности в этом тике (`LivingBreatheEvent`, публикуется из `LivingEntity.baseTick` каждый тик для живой сущности с общей логикой воздуха): `entity` — её ID и `kind` — её тип (`minecraft:player`, `minecraft:zombie`). |
| `living_change_target` | сервер | может отменить | Моб собирается сменить цель. |
| `living_conversion_post` | сервер | слушает | Моб преобразован в другой тип (NeoForge `LivingConversionEvent.Post`): `converted` заменяет `entity`. |
| `living_conversion_pre` | сервер | может отменить | Моб собирается преобразоваться в другой тип (NeoForge `LivingConversionEvent.Pre`, ванильный `Mob.convertTo`): утопающий зомби, пиглин или хоглин в верхнем мире, растущий головастик, мухомор, подстриженный раздатчиком, свинья или деревенский житель, поражённые молнией. |
| `living_damage_post` | сервер | слушает | NeoForge `LivingDamageEvent.Post`: сущность `entity` типа `kind` получила урон от удара, описанного `container`, с итоговыми значениями (урон здоровью, понижения, тики неуязвимости после атаки). |
| `living_damage_pre` | сервер | слушает | NeoForge `LivingDamageEvent.Pre`: `LivingEntity.actuallyHurt` для сущности `entity` типа `kind` (`minecraft:player`, `minecraft:zombie`) от `source` (называемого так, как `native_living_incoming_damage`) уже учёл броню и магию, теперь наступает очередь сердец поглощения: `container` хранит начальный урон, текущий (`new_damage`, который может быть 0) и каждое применённое понижение (броня, зачарования, эффекты мобов). |
| `living_death` | сервер | может отменить | Моб или серверный игрок вот-вот умрёт (хук NeoForge `LivingDeathEvent`, ванильный `LivingEntity.die`): здоровье иссякло, а лут, опыт и сообщение о смерти ещё не отработали. |
| `living_death_message` | сервер | собирает ответы | `living_death_message` запрашивает сообщение о смерти, которое прирученный моб отправляет владельцу (`TamableAnimal.die`), после формирования сообщения и только когда оно будет отправлено (владелец в сети, `show_death_messages` включён). |
| `living_death_post` | сервер | слушает | Смерть моба или серверного игрока завершилась (хук Fabric `ServerLivingEntityEvents.AFTER_DEATH`, ванильный `LivingEntity.die` на `broadcastEntityEvent`): выпали лут, снаряжение и опыт, а сообщение о смерти отправлено. |
| `living_destroy_block` | сервер | может отменить | Моб вот-вот разрушит блок (хук NeoForge `LivingDestroyBlockEvent`): иссушитель, ломающий блоки вокруг себя; разоритель, ломающий листья; проверка стен эндер-драконом; эндермен, поднимающий блок; чешуйница, будящая своих собратьев (заражённые блоки ломаются); зомби, ломающий дверь или яйцо черепахи. |
| `living_drops` | сервер | может отменить | Не `loot_drops`: это шаг модификатора лута, который изменяет полученные из таблицы стаки (блоки, стрижка, таблица моба). |
| `living_drown` | сервер | может отменить | Сущность, которая не может дышать и у которой нет воздуха (`LivingDrownEvent`, вызывается из `LivingEntity.baseTick`), с ванильными ответами по умолчанию: она тонет при -20 воздуха, получает 2 урона и показывает 8 пузырей. |
| `living_entity_use_item_finish` | обе | слушает | Хук NeoForge `LivingEntityUseItemEvent.Finish`: использование `entity` предмета `stack` в руке `hand` завершилось (`LivingEntity.completeUsingItem`); `duration` — оставшиеся тики, всегда 0. |
| `living_entity_use_item_start` | обе | может отменить | Хук NeoForge `LivingEntityUseItemEvent.Start`: `entity` начинает использовать `stack` в руке `hand` (`LivingEntity.startUsingItem`) на `duration` тиков (ответ `use_duration`). |
| `living_entity_use_item_stop` | обе | может отменить | Хук NeoForge `LivingEntityUseItemEvent.Stop`: `entity` отпускает `stack` в руке `hand` до завершения использования (`LivingEntity.releaseUsingItem`: кнопка использования отпущена, открыто окно); `duration` — оставшиеся тики. |
| `living_entity_use_item_tick` | обе | может отменить | Хук NeoForge `LivingEntityUseItemEvent.Tick`: каждый тик, когда `entity` использует `stack` в руке `hand` и начинается `LivingEntity.updatingUsingItem`; `duration` — оставшиеся тики. |
| `living_equipment_change` | сервер | слушает | Слот экипировки моба, стойки для брони или игрока получил другой стак (хук NeoForge `LivingEquipmentChangeEvent`, вызывается в `LivingEntity.collectEquipmentChanges`): `entity` — ID сущности, `kind` — её тип (`minecraft:zombie`, `minecraft:player`), `slot` — `EquipmentSlot` (`mainhand`, `offhand`, `feet`, `legs`, `chest`, `head`, `body`, `saddle`), `from` — содержимое слота, когда хук последний раз срабатывал (нет — для пустого), а `to` — содержимое сейчас. |
| `living_equipment_listened` | сервер | собирает ответы | Больше не нужно: игра каждый тик читает экипировку всех мобов и подключённых игроков и сообщает в `living_equipment_change`, если включённый мод подписан на этот хук. |
| `living_fall` | обе | может отменить | Живая сущность вот-вот обработает падение (`LivingFallEvent`). |
| `living_flying_speed` | обе | слушает | Воздушное ускорение живой сущности (`LivingEntity.getFlyingSpeed`) вот-вот будет использовано при перемещении. |
| `living_get_projectile` | обе | собирает ответы | Вызов `LivingEntity.getProjectile(weapon)` (хук NeoForge `LivingGetProjectileEvent`) для игрока (`player` и `name`) или моба (`entity`, `kind`): выстрел скелета или иллюзионера либо заряженный арбалет разбойника или пиглина. |
| `living_heal` | обе | может отменить | Живая сущность вот-вот восстановит здоровье. |
| `living_incoming_damage` | сервер | может отменить | NeoForge `LivingIncomingDamageEvent`: `LivingEntity.hurtServer`, пройдя проверки (неуязвимая или мёртвая сущность, огонь у огнестойкой сущности), собирается нанести урон сущности `entity` типа `kind` (`minecraft:zombie`, `minecraft:player`) через `source`: её тип урона (`minecraft:fall`), сущность, вызвавшую урон (`DamageSource.getEntity`: стрелок стрелы), прямую сущность (`getDirectEntity`: идентификатор сущности снаряда, когда тип урона называет её, у игрока, моба или Эндер-дракона; 0 для `magic()` и любого другого источника без прямой сущности, даже во время попадания этого снаряда) и откуда исходит (`getSourcePosition`). |
| `living_jump` | обе | слушает | Живая сущность прыгнула (`LivingEvent.LivingJumpEvent`). |
| `living_jump_power` | обе | слушает | Живая сущность: сила прыжка (`LivingEntity.getJumpPower`: атрибут силы прыжка, коэффициент прыжка блока и усиление прыжка) вот-вот будет использована `jumpFromGround`. |
| `living_knockback` | сервер | может отменить | NeoForge `LivingKnockBackEvent`: `LivingEntity.knockback` собирается отбросить сущность `entity` типа `kind` (`minecraft:zombie`, `minecraft:player`) с силой `strength` в сторону от направления (`x`, `z`) по исходной силе, до того как сопротивление отбрасыванию сущности поглотит часть силы и изменится её движение. |
| `living_shield_block` | сервер | слушает | Хук NeoForge `LivingShieldBlockEvent`: сущность `entity` типа `kind` (`minecraft:player`, `minecraft:zombie`) получает `damage` от `source` (её тип и сущности, как называет их `native_living_incoming_damage`), и `LivingEntity.applyItemBlocking` вот-вот определит, что блокирует поднятый предмет с `blocks_attacks`. |
| `living_speed` | обе | слушает | Скорость передвижения живой сущности (`LivingEntity.getSpeed`) вот-вот будет определять движение по земле и через жидкости. |
| `living_use_totem` | сервер | может отменить | Хук NeoForge `LivingUseTotemEvent`: смертельный удар по сущности `entity` типа `kind` от `source` (называется, как в `native_living_incoming_damage`) будет принят предметом `death_protection` `item` в руке `hand` (основной или второй; `LivingEntity.checkTotemDeathProtection`). |
| `living_visibility` | сервер | слушает | Насколько `entity` видна для `looking` (хук NeoForge `LivingVisibilityEvent`, последний вызов в `LivingEntity.getVisibilityPercent`): дистанция, с которой проверка нацеливания моба видит её, масштабируется значением `visibility`. |
| `mob_despawn` | сервер | слушает | `MobDespawnEvent` из NeoForge: моб `mob` (типа `kind`, в `pos`) собирается выполнить `Mob.checkDespawn`, которое срабатывает один раз за тик для каждого тикаемого моба или только проверяемого на деспавн, включая постоянных мобов и мобов в мирном режиме. |
| `mob_killed` | сервер | слушает | Ближний удар `player` убил моба `entity` типа `kind` (о других смертях пока не сообщается). |
| `mob_spawn_placement_check` | сервер | слушает | `MobSpawnEvent.SpawnPlacementCheck` из NeoForge: может ли тип сущности `kind` заспауниться в `pos` в мире `level` по причине `reason`? |
| `mob_spawn_position_check` | сервер | слушает | `MobSpawnEvent.PositionCheck` из NeoForge: может ли только что созданный моб типа `kind` стоять в `pos` (его позиция, центр моба) в мире `level` по причине `reason`? |
| `mob_spawned` | сервер | слушает | Новый моб `kind` появился в измерении `dimension` в `pos`: `natural`, когда это сделал спавн мира, и false, когда что-то другое (разведение, рейд, спаунер, превращение, `/summon`, а также второй моб у всадника, заспавненного миром). |
| `mob_split` | сервер | может отменить | Слайм, лавовый куб или серный куб умирает и вот-вот разделится на меньших (`MobSplitEvent` из NeoForge, `AbstractCubeMob.remove` из ванили): `parent` — идентификатор его сущности, `kind` — тип, `size` — размер, `child_size` — размер потомков (половина текущего) и `count` — сколько создала ваниль (от 2 до 4 для слайма или лавового куба, 2 для серного куба; взведённый не создаёт потомков, и его не спрашивают). |
| `play_level_sound_at_entity` | обе | может отменить | Звук, привязанный к `entity`, собирается заиграть (NeoForge `PlayLevelSoundEvent.AtEntity`, в перегрузке `Level.playSeededSound` для сущности, `playSound(except, sourceEntity, ...)`): рог козы, язык лягушки, поедание и спавн, поедание животного (лягушка, коза, нюхач), длинный прыжок, таран козы, заряд и отклонение бриза, стрижка (овца, мухомор, снежный голем, bogged, медный голем), окрашивание овцы, удар топора по медному голему, предмет и удвоение аллея, вылупление гастлинга, бросок трезубца (трезубец) или течение (игрок), а также звук мода, привязанный к сущности. |
| `portal_spawn` | сервер | может отменить | Огонь, размещённый в `pos` (`state`), обнаружил пустую обсидиановую рамку, `shape`, и готовится заполнить её порталом Незера (`BlockEvent.PortalSpawnEvent`, в `BaseFireBlock.onPlace`), в Верхнем мире или в Незере. |
| `potential_spawns` | сервер | может отменить | Возможные естественные спавны на серверной позиции (`LevelEvent.PotentialSpawns` из NeoForge, `NaturalSpawner.mobsAt`). |
| `projectile_impact` | сервер | может отменить | Снаряд попал во что-то (NeoForge `ProjectileImpactEvent`, публикуется там, где ванильный снаряд обнаружил столкновение, до воздействия попадания): `projectile` (id его сущности) типа `kind` (`minecraft:arrow`), выпущенный `owner` (id сущности моба или игрока, 0 если нет) в мире `dimension`. |
| `projectile_launch` | сервер | слушает | Серверный снаряд был добавлен в мир. |
| `spawn_change` | сервер | слушает | Точка появления в мире изменилась на `pos` и `angle`. |
| `trade_with_villager` | сервер | собирает ответы | Сделка с жителем или бродячим торговцем совершена (`MerchantResultSlot.onTake`, когда предложение взято): `merchant` — id сущности торговца, `kind` — его тип (`minecraft:villager`, `minecraft:wandering_trader`), `offer` — живой объект в торговом сеансе (не копия), каким его оставила сделка: `uses` посчитаны, `result` — то, что получил игрок. |

#### Блоки

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `alter_ground` | сервер | собирает ответы | Декоратор дерева `alter_ground` (`AlterGroundDecorator.place`, подзол под большой елью) собирается изменить землю вокруг `positions`: самый нижний блок ствола или корней (`logs`, если корней нет; `logs` и `roots`, если они начинаются на одном уровне; иначе `roots`); вокруг него размещаются только те позиции, чья Y совпадает с Y первой. |
| `beacon_state_change` | сервер | слушает | Маяк в `pos` активировался или деактивировался после проверки луча. |
| `bell_ring` | сервер | может отменить | На авторитетном сервере колокол в `pos` вот-вот зазвонит. |
| `block_break_progress_update` | сервер | слушает | Анимация разрушения блока у клиента сервера изменилась на `progress` в `pos` (от 0 до 9). |
| `block_burn` | сервер | может отменить | На авторитетном сервере огонь вот-вот сожжёт блок в `pos`. |
| `block_changed` | сервер | слушает | Авторитетный сервер зафиксировал изменённое состояние через `Level.setBlock`. |
| `block_cook` | сервер | слушает | Блок завершил приготовление `item` до `result` в `pos`. |
| `block_damage_abort` | сервер | слушает | Игрок на сервере остановил копку `pos` до разрушения блока, после того как сервер сбросил прогресс копки и отправил блок обратно на клиент. |
| `block_dispense_armor` | сервер | слушает | Раздатчик надел `item` на первую подходящую цель `target` перед своим блоком в `pos`. |
| `block_drops` | сервер | может отменить | NeoForge `BlockDropsEvent`: блок `state` разрушился в `pos`, и `Block.dropResources` создал предметные сущности `drops`, которые будут заспавнены, но в мире пока ничего нет. |
| `block_fade` | сервер | может отменить | Блок в `pos` собирается перейти из `from` в `to` из-за таяния или гибели кораллов. |
| `block_failed_dispense` | сервер | слушает | Раздатчик в `pos` не смог выдать `item`. |
| `block_form` | сервер | может отменить | Осадки собираются образовать лёд или разместить либо утолщить снег в `pos`. |
| `block_grow_feature` | сервер | может отменить | Саженец, азалия, гриб или грибок в `pos` собирается вырасти в настроенный `feature`, `None`, если ванили нечего разместить (одинокий саженец тёмного дуба). |
| `block_highlight` | клиент | может отменить | Перед построением контура выделения подходящего целевого блока. |
| `block_ignite` | сервер | может отменить | На авторитетном сервере огонь вот-вот будет размещён в `pos` огнивом, огненным зарядом, лавой, молнией или распространением огня. |
| `block_lock_check` | сервер | может отменить | Проверка блокировки инвентаря блока у гостя в `pos`. |
| `block_receive_game` | сервер | может отменить | Скалковый датчик или скалковый крикун получил вибрацию для `event` в `pos`. |
| `block_spread` | сервер | может отменить | На авторитетном сервере жидкость течёт или огонь распространяется из `from` в `to` (`kind` — `fluid` или `fire`): `FlowingFluid.spreadTo`, а `FireBlock.tick` поджигает `to` или сжигает его (`checkBurnOut`). |
| `block_step_on` | сервер | слушает | `Block.stepOn` для блока мода на его авторитетном серверном мире. |
| `block_tool_modification` | обе | может отменить | Инструмент вот-вот преобразует блок. |
| `blocks` | обе | собирает ответы | Блоки, добавляемые этим модом, объявляемые в коде, а не по одному файлу на каждый блок (`DeferredRegister.Blocks` NeoForge, выгружаемый через `RegisterEvent`): поместите по одному `mod_api::Block` на блок, то же объявление, которое хранит `data/<id>/horizon/block/<path>.json` контентного мода (ванильные `template`, `properties`, `state_properties`, `shapes`...), с id в пространстве имён, принадлежащем манифесту вашего мода. |
| `bonemeal` | обе | может отменить | `BonemealEvent`: костная мука из `stack` была использована на блоке `state` в `pos` (`BoneMealItem.growCrop`), игроком `player` или, если он `None`, раздатчиком или жителем. |
| `break_block` | сервер | может отменить | Игрок ломает блок `state` в `pos`, когда игра это разрешила (удерживаемый предмет может ломать блоки, палочка отладки не перехватила клик): с этого начинается `ServerPlayerGameMode.destroyBlock`. |
| `bucket_empty` | сервер | может отменить | Серверный игрок опорожняет ведро в `pos` (`fluid`: `minecraft:water`, `minecraft:lava`, `minecraft:powder_snow`; ведро с рыбой или аксолотлем опорожняет `minecraft:water`): там, где `BucketItem.use` нашёл блок, в который помещается жидкость и где игрок может строить, прямо перед её размещением. |
| `campfire_start` | сервер | слушает | Предмет был помещён на костёр в `pos` и начал готовиться. |
| `crop_grow_post` | сервер | слушает | Блок, похожий на урожай, вырос (`CropGrowEvent.Post` NeoForge). |
| `crop_grow_pre` | сервер | слушает | Блок, похожий на урожай, в `pos` собирается вырасти на случайном тике (`CropGrowEvent.Pre` NeoForge): урожаи, стебли, адский нарост, какао, сладкие ягоды и бамбук — в собственном блоке; кактус и хоровые цветы — в блоке над ними; сахарный тростник — в собственном блоке; ламинария, плакучие и вьющиеся лианы и пещерные лианы — в блоке, в который они прорастают. |
| `dispenser_acts` | сервер | заменяет результат | Раздатчик в `pos` в `dimension`, направленный в `facing`, собирается воздействовать на `stack` (`DispenserBlock.getDispenseMethod`) один раз за действие раздатчика на авторитетном сервере (не для дропперов). |
| `enchanted_block_loot` | сервер | слушает | `EnchantedBlockLootEvent` NeoForge: таблица лута блока считывает уровень `enchantment` у `tool`, чтобы определить, что выпадет из `state` в `pos`; происходит в `ApplyBonusCount.run` или `BonusLevelTableCondition.test`. |
| `farmland_trample` | сервер | может отменить | Вызывается на сервере перед превращением пашни в базовый блок при приземлении. |
| `fill_bucket` | сервер | может отменить | Игрок наполняет (`stack` — пустое ведро) или опорожняет ведро в `pos`: `BucketItem.use`, где запрашивается `Level.mayInteract`. |
| `fluid_flow` | обе | слушает | Поток блока жидкости в `pos` (`FlowingFluid.getFlow`, вектор, отбрасывающий сущность, стоящую в ней) вот-вот отбросит игрока, моба, предмет, сферу опыта, стойку брони или стрелу. |
| `fluid_level_change` | сервер | может отменить | Текучая жидкость в `pos` вот-вот изменит уровень. |
| `fluid_place_block` | сервер | может отменить | Жидкость в `liquid_pos` вот-вот поставит `state` в `pos`. |
| `fluid_tick` | сервер | может отменить | Жидкость в `pos` вот-вот выполнит назначенный тик (`FlowingFluid.tick`). |
| `fluid_tooltip` | клиент | слушает | `FluidTooltipEvent`: подсказка `stack`, жидкости, слот которой игра отображает (ёмкости жидкости в просмотрщике рецептов), после того как её имя, количество в миллибакетах и, при F3+H, её id попадут в `lines`; для каждой строки цвет передан как `rgba` (`u32::MAX` сохраняет собственный цвет текста). |
| `grindstone_place` | сервер | может отменить | Точильный камень в `pos` сформировал результат после изменения входных предметов для серверного игрока `player` (UUID) и `player_entity_id` (`GrindstoneEvent.OnPlaceItem`, в `GrindstoneMenu.createResult`): `place` содержит входные предметы, ванильный результат и изначально пустое переопределение. |
| `grindstone_take` | сервер | может отменить | Серверный игрок `player` (UUID) и `player_entity_id` забрал результат точильного камня в `pos` (`GrindstoneEvent.OnTakeItem`, в `onTake` слота результата): `take` содержит входные предметы, опыт (-1 для ванильного или значение, заданное `grindstone_place`) и содержимое входных предметов после (пусто). |
| `hanging_break` | сервер | может отменить | Какая-либо рамка предмета (`minecraft:item_frame`, `minecraft:glow_item_frame`) или картина (`minecraft:painting`) вот-вот сломается; `kind` — тип её сущности, `pos` — блок, в котором она висит, мира `dimension` (тип `&str`): удар серверного игрока (`player`, их состояние подставлено, поэтому `PlayerRef::tell` отвечает им), взрыв или разрушенная стена, на которой она висит (`player` — None в обоих случаях). |
| `hanging_place` | сервер | может отменить | Серверный игрок размещает висящую сущность: рамку предмета (`minecraft:item_frame`, `minecraft:glow_item_frame`), картину (`minecraft:painting`) или узел поводка (`minecraft:leash_knot`, когда поводок привязан к забору, к которому ещё не привязано), `kind` — тип её сущности и `pos` — блок, который она занимает (для узла — блок забора). |
| `hopper_inventory_search` | сервер | слушает | Воронка выбрала контейнер во время поиска предметов для отправки или извлечения. |
| `hopper_pickup_item` | сервер | может отменить | Воронка на `pos` вот-вот подберёт `count` предмета `item`. |
| `instantiate_block_state_definition` | клиент | собирает ответы | Читается файл blockstate, у которого мод зарегистрировал `neoforge:definition_type` (`register_block_state_models`): для каждого состояния из `states`, отображающего модель, добавьте `BlockStateModelPick`, или один `Refuse`, если файл не читается, для типа, которым владеет ваш мод. |
| `inventory_block_start` | сервер | может отменить | Серверный игрок вот-вот откроет инвентарь, предоставленный блоком в `pos`. |
| `leaves_decay` | сервер | может отменить | Непостоянный блок листвы на расстоянии 7 готов к разложению в `pos`. |
| `left_click_block` | сервер | может отменить | Игрок левой кнопкой нажимает на блок `state` в `pos`, `face` — задетая грань (`[0, 1, 0]` — верх): где `ServerPlayerGameMode.handleBlockBreakAction` получает начало копания, после проверок досягаемости, защиты спавна и границы мира, перед `Block.attack` (проигрывается нотный блок) или креативным уничтожением. |
| `menu_background` | клиент | слушает | Обычный запрос размытия фона меню до проверки конечности и диапазона. |
| `moisture_change` | сервер | может отменить | Пашня в `pos` вот-вот изменит влажность с `from` на `to`. |
| `neighbor_notify` | сервер | может отменить | Блок в `pos` (`state`) вот-вот сообщит соседям, что изменился (`BlockEvent.NeighborNotifyEvent` из NeoForge): в `ServerLevel.updateNeighborsAt`, который уведомляет все шесть сторон (`skipped_side` равно `None`), и в `updateNeighborsAtExceptFromFacing`, который уведомляет все стороны, кроме `skipped_side`. |
| `note_block_change` | сервер | может отменить | Игрок настраивает нотоблок на авторитетном сервере, а не в клиентском предсказании. |
| `note_block_play` | сервер | может отменить | Нотоблок в `pos` вот-вот сыграет (`NoteBlockEvent.Play` из NeoForge, в `NoteBlock.playNote`), когда блок над ним больше не заглушает его. |
| `piston_post` | сервер | слушает | На авторитетном сервере поршень в `piston` толкнул блоки в `push` на один шаг по `direction` (`[0, 1, 0]` — вверх; при втягивании поршень тянул, `extending` false) и разрушил те, что в `destroy` (событие `PistonEvent.Post` NeoForge в конце `PistonBaseBlock.moveBlocks`, после обновлений соседей). |
| `piston_pre` | сервер | может отменить | На авторитетном сервере поршень в `piston` собирается толкнуть блоки в `push` на один шаг по `direction` (`[0, 1, 0]` — вверх; при втягивании поршень тянет, `extending` false) и разрушить те, что в `destroy` (событие `PistonEvent.Pre` NeoForge в `PistonBaseBlock.moveBlocks`, когда его структура определена). |
| `portal_create` | сервер | слушает | Для перехода между измерениями был построен портал Незера, либо для прибытия была создана платформа Края. |
| `prepare_hopper_transfer` | сервер | слушает | Воронка в `pos` начала попытку перемещения предмета. |
| `right_click_block` | обе | может отменить | Игрок нажимает правой кнопкой по блоку (NeoForge: `PlayerInteractEvent.RightClickBlock`): по разу на руку, там, где начинается `useItemOn`, на нажавшем клиенте и на сервере (`click` говорит, где это, а также передаёт блок, точку попадания, руку с её стаком и игрока), и для наблюдателя тоже (сервер позволяет только открыть меню). |
| `screen_transparent_background` | клиент | может отменить | `Screen.extractTransparentBackground`: рисует полупрозрачный тёмный фон за игровым экраном (инвентарь, контейнер, творческий режим, книга, табличка, редактор). |
| `sculk_bloom` | сервер | слушает | Скалку-катализатор в `pos` начал расцветать после получения опыта. |
| `sign_changed` | сервер | может отменить | Игрок закончил редактировать табличку: `ServerGamePacketListenerImpl.updateSignText` получил пакет обновления, и все проверки пройдены (редактировалась именно эта табличка, она в пределах досягаемости, она не покрыта воском, и чат-фильтр пропустил текст), прямо перед установкой строк на табличку. |
| `sponge_absorb` | сервер | слушает | Губка в `pos` впитала воду и стала мокрой. |
| `use_item_on_block` | обе | может отменить | `UseItemOnBlockEvent`: после выбора фазы, до её части или ванильного поведения. |
| `vault_change_state` | сервер | слушает | Хранилище в `pos` изменило состояние на `state`. |
| `vault_loot` | сервер | слушает | Хранилище в `pos` вот-вот выдаст лут-предмет. |

#### Предметы, инвентарь, крафт

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `anvil_craft_post` | сервер | слушает | Игрок забрал результат наковальни в `pos`, и уровни, входные предметы и износ наковальни уже обработаны (`AnvilCraftEvent.Post`); сломанная наковальня уже превратилась в воздух. |
| `anvil_craft_pre` | сервер | может отменить | Игрок забрал результат наковальни в `pos` (`AnvilCraftEvent.Pre`), до списания уровней, использования входных предметов и износа наковальни. |
| `anvil_damaged` | сервер | слушает | Наковальня в `pos` получила урон, изменив свой блок с `from` на `to`. |
| `anvil_update` | обе | может отменить | Наковальня в `pos` создала ванильный результат с левым входом для `player` (UUID) и `player_entity_id` (`AnvilUpdateEvent`, после `AnvilMenu.createResult`) при каждом изменении входа или имени: `update` содержит входные предметы (правый может быть пуст, для переименования), текст переименования и ванильный результат; его `output`, `xp_cost` и `material_cost` начинаются с ванильных значений для изменения модами; установите `cancel`, чтобы результата не было. |
| `armor_stand_manipulate` | сервер | может отменить | Игрок берёт предмет с стойки для брони или надевает его на неё (`ArmorStand.interact`, после проверок отключённых слотов стойки и блокировок `swapItem` для взятия/размещения, до перемещения чего-либо): `stand_id` — `Entity.getId` стойки, а `slot` — слот, по которому пришёлся клик (`mainhand`, `offhand`, `feet`, `legs`, `chest` или `head`). |
| `brewing_complete` | сервер | слушает | Зельеварка закончила варку в `pos`, после изменения содержимого её бутылок. |
| `brewing_fuel` | сервер | слушает | Зельеварка в `pos` израсходовала `fuel` и получила топливо для варки. |
| `brewing_start` | сервер | слушает | Зельеварка в `pos` начала варку `ingredient`, после установки топлива и таймера. |
| `can_eat` | обе | заменяет результат | `Player.canEat` для `stack`: из `Consumable.canConsume` при начале поедания и пригодности взаимодействия с блоком для торта и торта со свечами. |
| `compost_item` | сервер | слушает | Игрок добавил компостируемый предмет в компостер в `pos`. |
| `container_loot_filled` | сервер | собирает ответы | Дополнение Horizon: аналога в NeoForge нет. |
| `container_opened` | сервер | может отменить | Игрок открывает одно из собственных меню игры (`ServerPlayer.openMenu`; роль `PlayerContainerEvent.Open` в NeoForge, которое наступает после открытия меню и не может остановить его): `kind` — тип меню (`minecraft:generic_9x3`, `minecraft:furnace`, `minecraft:crafting`, `minecraft:merchant`, `minecraft:anvil`, `minecraft:lectern`...; инвентарь лошади — `minecraft:horse_inventory`), а `pos` — блок, которому оно принадлежит; для жителя, ездового животного или сундука транспортного средства его нет. |
| `crafter_craft` | сервер | слушает | Крафтер в `pos` выполнил рецепт `recipe`. |
| `creative_slot` | сервер | может отменить | Игрок в творческом режиме устанавливает слот своего инвентаря из экрана творческого режима (`ServerGamePacketListenerImpl.handleSetCreativeModeSlot`): `slot` — это слот меню инвентаря (5-8 броня, 9-35 инвентарь, 36-44 хотбар, 45 вторая рука; ниже 0 — стак выброшен из экрана и выпадет), а `item` (`minecraft:air` для пустого слота) и `count` — то, что туда помещается. |
| `enchant_item` | сервер | слушает | Серверный игрок применил зачарование в `pos`; `item` — готовый предмет, а `levels` — потраченные уровни. |
| `enchantment_level_set` | сервер | слушает | Стол зачарований в `pos` выдал уровень одного предложения (`EnchantmentMenu.slotsChanged`, после `getEnchantmentCost`): `offer` содержит серверного игрока, чей это стол, строку (0–2), силу (учтённые книжные полки), предмет и выданный уровень. |
| `furnace_burn` | сервер | слушает | Печь в `pos` израсходовала топливо и начала его сжигать или возобновила сжигание. |
| `furnace_extract` | сервер | слушает | Серверный игрок забрал результат из печи в `pos`; `item` — id выходного предмета, а `count` — количество забранных предметов. |
| `furnace_smelt` | сервер | слушает | Печь завершила переплавку `item` в `result` в `pos`. |
| `furnace_start_smelt` | сервер | слушает | Печь в `pos` начала обрабатывать `item` (её таймер приготовления сдвинулся с нуля). |
| `get_enchantment_level` | обе | слушает | `GetEnchantmentLevelEvent`: зачарования, которые видит геймплей на `stack`, начиная с его собственных `minecraft:enchantments`. |
| `get_enchantment_level_for` | обе | слушает | `GetEnchantmentLevelForEvent`: то же чтение уровня зачарований, что и `get_enchantment_level`, но запрашивается только когда вызывающий знает, кто носит или держит стак. |
| `get_equipment_drop_chance` | сервер | слушает | Умирающий моб `entity` типа `kind` вот-вот определит шанс дропа предмета `stack` в слоте `slot` (`Mob.dropCustomDeathLoot`, хук NeoForge `GetEquipmentDropChanceEvent`). |
| `ingredient` | обе | собирает ответы | Ингредиент рецепта собственного вида мода: запись указывает тип в пространстве имён вне `minecraft:`, и загрузчик сохранил его для этого вызова. |
| `ingredient_test` | обе | слушает | Собственный вид ингредиента мода, сопоставляемый с конкретным стаком во время крафта: `test` ингредиента вызывает это с типом, его NBT-компаундом и сопоставляемым стаком, `accept` уже true (ненепустой список времени загрузки указал предмет; пустой пользовательский список его не фильтрует). |
| `inventory_move_item` | сервер | может отменить | Воронка вот-вот переместит один предмет между контейнерами, включая костную муку в компостере. |
| `item_attribute_modifier` | обе | слушает | `ItemAttributeModifierEvent`: модификаторы атрибутов, которые даёт `stack` (`ItemStack.getAttributeModifiers`), начиная с его собственного `minecraft:attribute_modifiers` или с модификаторов его предмета. |
| `item_consumption_teleport` | сервер | может отменить | Серверный игрок или лис, съевшие плод хоруса (или другой предмет с эффектом потребления `teleport_randomly`), готовятся попробовать переместиться в случайную точку (NeoForge `EntityTeleportEvent.ItemConsumption`, ванильный `TeleportRandomlyConsumeEffect.apply`): по одному разу за попытку (до 16) после того, как они перестали ехать верхом. |
| `item_crafted` | сервер | слушает | Игрок забрал результат сетки крафта (`ResultSlot.onTake`, NeoForge `PlayerEvent.ItemCraftedEvent`): `player` — uuid игрока, `recipe` — совпавший с сеткой рецепт (`minecraft:oak_planks`) и `stack` — забранный результат с его компонентами. |
| `item_fished` | сервер | собирает ответы | Серверный игрок `player` (UUID) и `player_entity_id` вытаскивает улов: `ItemFishedEvent` в `FishingHook.retrieve` после формирования лута рыбалки и его модификаторов. |
| `item_model_property` | клиент | слушает | Клиентская оценка условного свойства модели предмета мода с пространством имён. |
| `item_model_range_property` | клиент | слушает | Клиентское значение числового свойства модели предмета, зарегистрированного модом (`register_range_select_item_model_property`), запрашивается каждый раз, когда `minecraft:range_dispatch` по нему выбирает ветку. |
| `item_model_select_property` | клиент | слушает | Клиентское значение категориального свойства модели предмета, зарегистрированного модом (`register_select_item_model_property`), запрашивается каждый раз, когда `minecraft:select` по нему выбирает случай. |
| `item_model_update` | клиент | слушает | Узел типа, зарегистрированного модом (`register_item_models`, ванильный `ItemModel.update`), разрешается для стака, в руке, на экране или на земле: передайте имена `models` узла для отрисовки по порядку, только для типа, принадлежащего вашему моду. |
| `item_picked_up` | сервер | слушает | `player` касается наземной сущности выпавшего предмета `item` (`ItemEntity.playerTouch`) до проверки задержки подбора и цели: вызывается каждый тик контакта, даже когда предмет нельзя взять. |
| `item_picked_up_partial` | сервер | слушает | `player` подобрал часть наземного стака (`ItemEntity.playerTouch`, сразу после `Inventory.add`, который забрал часть и оставил остальное). |
| `item_picked_up_post` | сервер | слушает | `player` подобрал весь стак наземной сущности выпавшего предмета `item` (`ItemEntity.playerTouch`, сразу после `Inventory.add`, забравшего всё), сущность названа `item_picked_up`. |
| `item_smelted` | сервер | слушает | `FurnaceResultSlot.checkTakeAchievements` выполнен для серверного игрока у слота результата печи, доменной печи или коптильни: `player` — его UUID и `player_entity_id` — его сетевой id; `stack` и `amount_removed` — это то, что ваниль туда передаёт (`getSmelting`, `getAmountRemoved`). |
| `item_stacked_on_other` | обе | может отменить | NeoForge `ItemStackedOnOtherEvent`, до любого переопределения предмета. |
| `item_tint_source` | клиент | слушает | Клиентский цвет источника тонировки предмета, зарегистрированного модом (`register_item_tint_sources`, ванильный `ItemTintSource.calculate`), запрашивается для каждой его записи каждый раз, когда предмет разрешается, в руке, на экране или на земле. |
| `item_tooltip` | клиент | слушает | `ItemTooltipEvent`: весь текст подсказки `stack` (`ItemStack.getTooltipLines`), когда название, собственные строки предмета, строки компонентов, расширенные строки и предупреждение оператора уже находятся в `lines`, каждая с цветом, упакованным в `rgba` (`u32::MAX` сохраняет собственный цвет текста). |
| `item_toss` | сервер | может отменить | `ItemTossEvent`: `player` бросает сущность предмета `item` (`Player.drop`: клавиша выбрасывания, выброс из меню, то, что не помещается в инвентаре), а не то, что рассыпается при смерти. |
| `items` | обе | собирает ответы | Предметы, которые этот мод добавляет и объявляет в коде (NeoForge `DeferredRegister.Items`), как `blocks` объявляет блоки: по одному `mod_api::Item` на предмет, файл контента мода `data/<id>/horizon/item/<path>.json` (ванильные `template`, `max_stack_size`, `rarity`, `components`, `crafting_remainder`). |
| `menu_clicked` | сервер | слушает | Клик в меню, открытом модом (`native_open_menu`), который ничего не переместил: `slot` меню (сначала собственные слоты меню, затем инвентарь игрока, `native_menu_inventory_slot`; -999 — вне), `button` и `mode` клика (ID `ClickType`) игрока `uuid`. |
| `menu_closed` | сервер | слушает | Меню, открытое модом, закрыто: игрок закрыл его, сервер или мод открыл другое, или игрок вышел. |
| `menu_extra_data` | сервер | заменяет результат | Блок `block` в `pos` открывает тип меню мода `menu` для серверного игрока `player`: задайте `data`, чтобы передать экрану меню собственные байты (уровень машины, режим или рецепты), до 32 KiB, которые клиент читает через `screen_extra_data`, пока экран открыт (хук NeoForge `openMenu` с записью дополнительных данных). |
| `menu_links` | сервер | собирает ответы | Контейнеры, доступные через связанные слоты открытого меню мода `menu` (`namespace:path`) серверного игрока `player` в `pos`. |
| `mod_container_viewers` | сервер | слушает | Число игроков с открытым модовым контейнером в `pos` измерения `dimension` изменилось с `previous` на `viewers` (`ContainerOpenersCounter` из ванили, как у бочки): с 0 на больше 0 — это `onOpen`, обратно на 0 — `onClose`, любое изменение — `openerCountChanged`, поэтому блок может показывать себя открытым, пока `viewers` не равно 0. |
| `on_equip` | сервер | слушает | Слот экипировки игрока `slot` (`head`, `chest`, `legs`, `feet`, `offhand`, а также `mainhand` при смене рук или `/item`) получил другой предмет, `new` (отсутствует, если слот опустошён): `LivingEntity.onEquipItem`, до звука экипировки. |
| `prepare_item_craft` | сервер | заменяет результат | Результат из сетки крафта был подготовлен для серверного игрока; `pos` — верстак, или None для сетки инвентаря. |
| `prepare_item_enchant` | сервер | слушает | Варианты зачарования были рассчитаны для серверного игрока в `pos`. |
| `prepare_smithing` | сервер | заменяет результат | Результат кузнечного дела был подготовлен для серверного игрока в `pos`. |
| `right_click_item` | обе | может отменить | `PlayerInteractEvent.RightClickItem`, непосредственно перед `Item.use`. |
| `station_closing` | сервер | собирает ответы | Рабочая станция, открытая через `PlayerRef::open_station`, закрывается для `player` (UUID): игрок закрыл её, она перестала быть действительной или её заменило другое меню. |
| `trade_select` | сервер | слушает | Игрок на сервере выбрал сделку `index` у `villager` в меню торговли. |

#### Мир, чанки, тики

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `biome_colors` | клиент | слушает | Измените цвета биома `biome` (`minecraft:plains`) на этом клиенте: оттенки травы, листвы (листья, лозы), сухой листвы (листовой подстилки) и воды в формате 0xRRGGBB, такие, какие задают его файл биома и цветовые карты. |
| `can_continue_sleeping` | сервер | заменяет результат | `CanContinueSleepingEvent`: проверка для спящей живой сущности, может ли она продолжать спать; выполняется каждый тик для серверного игрока, которого проверки смерти и здоровья ещё не разбудили, и для спящего жителя деревни или торговца-скитальца. |
| `cauldron_level_changed` | сервер | может отменить | На авторитетном сервере котёл на `pos` вот-вот изменит уровень заполнения с `from_level` на `to_level` по причине `reason`. |
| `chunk_load` | сервер | слушает | Чанк `chunk` (`[x, z]`, в чанках) измерения `dimension` появился в мире (событие NeoForge `ChunkEvent.Load`): `new` имеет значение true, если он только что сгенерирован, и false, если прочитан из сохранения. |
| `chunk_sent` | сервер | слушает | Пакет чанка `chunk` ушёл к `player` (событие NeoForge `ChunkWatchEvent.Sent`, в конце `PlayerChunkSender.sendChunk`), после `chunk_watch`; теперь мод может отправить свои данные для этого чанка. |
| `chunk_ticket_level_updated` | сервер | слушает | Уровень тикета чанка `chunk` изменился с `old` на `new` (событие NeoForge `ChunkTicketLevelUpdatedEvent`, из `ChunkMap.updateChunkScheduling`): игрок, принудительный чанк, портал или арена боя с драконом начали или перестали удерживать его активным или тикающим. |
| `chunk_unload` | сервер | слушает | Чанк `chunk` измерения `dimension` покинул мир (событие NeoForge `ChunkEvent.Unload`), а также при закрытии мира из-за пустоты. |
| `chunk_unwatch` | сервер | слушает | `player` прекратил наблюдение за чанком `chunk` (событие NeoForge `ChunkWatchEvent.UnWatch`, `ChunkMap.dropChunk`): он покинул их зону видимости, или они покидают мир (через портал, телепортацию или возрождение) либо сервер. |
| `chunk_watch` | сервер | слушает | `player` начал наблюдение за чанком `chunk` (событие NeoForge `ChunkWatchEvent.Watch`): чанк попал в их зону видимости и отправляется на их клиент; один раз до `chunk_unwatch`. |
| `client_chunk_load` | клиент | слушает | Чанк `chunk` (`[x, z]`, в чанках) измерения `dimension` попал на уровень этого клиента с сервера (событие NeoForge `ChunkEvent.Load` на клиенте, где `isNewChunk` всегда false): один раз на пакет чанка, принимаемый клиентом, после того как столбец и его блок-сущности попали на уровень, чтобы слушатель мог читать его блоки через `ClientHost`. |
| `client_chunk_unload` | клиент | слушает | Чанк `chunk` (`[x, z]`, в чанках) измерения `dimension` покидает уровень этого клиента (событие NeoForge `ChunkEvent.Unload` на клиенте): один раз на резидентную колонку, когда её блоки ещё на месте, прямо перед освобождением. |
| `client_level_load` | клиент | слушает | Клиент создал новый мир для `dimension` (событие NeoForge `LevelEvent.Load` из конструктора `ClientLevel`): при подключении к серверу, включая однопользовательский режим, и при смене измерения. |
| `client_level_unload` | клиент | слушает | Клиент выгрузил мир измерения `dimension` (`LevelEvent.Unload` из `Minecraft.setLevel` и `disconnect`): заменён миром другого измерения или при выходе с сервера. |
| `client_tick_post` | клиент | слушает | Один раз за каждый тик клиента, на паузе или нет, а также на экране заголовка, после тика окна (`ClientTickEvent.Post` в NeoForge): только `Game` окна, а не собственный тик сервера (`server_tick` — это тот самый момент). |
| `client_tick_pre` | клиент | слушает | Один раз за каждый тик клиента, на паузе или нет, а также на экране заголовка, до тика окна (`ClientTickEvent.Pre` в NeoForge): только `Game` окна, а не собственный тик сервера (`server_tick` — это тот самый момент). |
| `entities_load` | сервер | слушает | Сущности из `chunk` завершили загрузку в серверный мир. |
| `entities_unload` | сервер | слушает | Сущности в `chunk` покинули активный серверный мир и поставлены в очередь на сохранение. |
| `food_level_changed` | сервер | может отменить | Уровень сытости серверного игрока `player` (`name`) вот-вот изменится с `from` на `to` (Bukkit `FoodLevelChangeEvent`): еда, голод и насыщение, истощение, отнимающее очко, голодание. |
| `level_load` | сервер | слушает | Мир измерения `dimension` открыт на сервере (NeoForge `LevelEvent.Load`, публикуется в `MinecraftServer.createLevels`): до загрузки любого из его чанков (`chunk_load`), поэтому здесь мод настраивает данные, которые сохраняет для мира. |
| `level_save` | сервер | слушает | Мир измерения `dimension` сохранён (NeoForge `LevelEvent.Save`, в `ServerLevel.save`): его данные мира, чанки и сущности — автосохранением, `/save-all`, паузой, закрытием мира или остановкой сервера; каждый мир в сохранении по очереди. |
| `level_tick_post` | обе | слушает | После успешного прохода тика мира на любой логической стороне; никогда после неудачного тика. |
| `level_tick_pre` | обе | слушает | Перед проходом тика мира на любой логической стороне. |
| `level_unload` | сервер | слушает | Мир измерения `dimension` закрыт (NeoForge `LevelEvent.Unload`, публикуется после `level.close()` в `MinecraftServer.stopServer`) после последнего сохранения: удалите данные, которые мод хранил для него. |
| `play_level_sound_at_position` | обе | может отменить | Звук собирается заиграть в `position` в `level` (NeoForge `PlayLevelSoundEvent.AtPosition`, в `Level.playSeededSound`): любой серверный звук, не привязанный к сущности, включая собственные звуки мобов (`Entity.playSound` играет в позиции сущности). |
| `server_tick` | сервер | слушает | Фазы тика авторитетного сервера: `ServerTickStart` до тика любого мира (NeoForge `ServerTickEvent.Pre`), `ServerTickEnd` после последнего (`.Post`). |
| `server_tick_rate_changed` | сервер | слушает | `/tick rate` изменил частоту тиков сервера, или `/tick sprint` завершился (`MinecraftServer.onTickRateChanged`, до изменения задержки автосохранения). |
| `sleep_finished_time` | сервер | может отменить | В мире `dimension` достаточно игроков проспали ночь, и часы вот-вот перейдут к утру (NeoForge `SleepFinishedTimeEvent`, ванильный `ServerLevel.tick` при переводе часов в `wake_up_from_sleep`). |
| `thunder_change` | сервер | может отменить | Сервер изменил своё грозовое состояние через `/weather`. |
| `tick_target_millis` | клиент | слушает | Сколько миллисекунд длится клиентский тик (`Minecraft.getTickTargetMillis`, целевое значение, на которое `DeltaTracker.Timer` окна делит время кадра): `millis` приходит как в ваниле (50 при 20 тиках в секунду, больше, когда `/tick rate` сервера медленнее); установите его, чтобы ускорить или замедлить клиентские фиксированные часы шага, «таймер». |
| `time_skip` | сервер | может отменить | Часы основного мира вот-вот перескочат из-за сна или `/time`. |
| `validate_tickets` | сервер | слушает | Мир `dimension` загрузил сохранённые тикеты чанков от `controller` (NeoForge `LoadingValidationCallback.validateTickets`), до первого тика и после `level_load`, один раз для каждого открытого мира и для каждого контроллера, зарегистрированного с валидатором, имеющим тикеты в мире. |
| `vanilla_game_event` | сервер | может отменить | Ванильное игровое событие вот-вот дойдёт до слушателей (NeoForge `VanillaGameEvent`, ванильное `Level.gameEvent`): каждое событие в `dimension`. |
| `vehicle_create` | сервер | слушает | Лодка или вагонетка создана в `pos`, после присоединения к миру. |
| `vehicle_damage` | сервер | может отменить | Транспорт вот-вот получит урон от атаки игрока. |
| `vehicle_destroy` | сервер | слушает | Лодка или вагонетка вот-вот уничтожены в `pos`. |
| `vehicle_moved` | сервер | слушает | Лодка или вагонетка (`kind`, `minecraft:oak_boat`) переместились с последнего тика (`VehicleMoveEvent`): `from` и `to` — её позиции, событие получено после тика, который её переместил. |
| `viewed_level_sound` | клиент | слушает | Звук мира, который сервер открыл для просмотра (arismoko/horizon#3716), дошёл до этого клиента, и клиент спрашивает, где в собственном мире игрока его проигрывать. |
| `weather_change` | сервер | может отменить | Сервер изменил параметры погоды. |
| `world_border_bounds_change_finish` | сервер | слушает | Отложенное изменение размера границы мира на сервере завершилось после применения конечного размера. |
| `world_border_bounds_changed` | сервер | слушает | Пределы границы мира на сервере изменились — либо сразу, либо при запросе отложенного изменения размера. |
| `world_border_center_changed` | сервер | слушает | Центр границы мира на сервере изменился. |
| `world_drawn` | клиент | слушает | Мир отрисован (см. `WorldDrawn`). |
| `world_lifecycle` | сервер | слушает | Фазы мира, только для чтения; `error_text` содержит собственный текст ошибки открытия движка, иначе пуст. |
| `world_saved` | сервер | слушает | Сервер завершил цикл сохранения мира. |
| `world_view_drawn` | клиент | слушает | Представление мира отрисовано (см. `WorldViewDrawn`): один раз на каждое представление, взятое в этом кадре, после мира кадра и перед `world_drawn`; представления, видимые внутри представления (его `parent`, на один `nesting_depth` глубже), идут перед ним. |
| `world_views` | клиент | собирает ответы | Запрашивается один раз при построении каждого кадра (не для панорамы заглавного экрана): добавьте мировые представления (см. `WorldView`), которые ваш мод хочет отрисовать в этом кадре, каждое родительское представление перед видимыми в нём представлениями. |

#### Команды, чат, сеть

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `broadcast_message` | сервер | может отменить | Серверная строка рассылки вот-вот будет записана в журнал и отправлена. |
| `chat_recipient` | сервер | может отменить | Публичная строка чата от `sender` (`sender_name`) собирается достигнуть помещённого игрока `recipient`: чат игрока после пропуска `server_chat` или `/say`/`/me` (с подписью или без). |
| `chat_suggestion_applied` | клиент | слушает | `chat_suggestion_applied(original, start, end, suggestion, input)` сообщает о дополнении чата после Tab или клика. |
| `client_chat` | клиент | может отменить | Событие NeoForge `ClientChatEvent`: клиент собирается отправить `message`. |
| `client_chat_received` | клиент | может отменить | Событие NeoForge `ClientChatReceivedEvent`: строка чата с сервера собирается отобразиться, после пропуска Chat Delay. |
| `client_disconnecting` | клиент | слушает | Перед фактическим разрывом соединения, но не при смене измерения. |
| `client_information_updated` | сервер | слушает | Событие NeoForge `ClientInformationUpdatedEvent`: клиент серверного игрока отправил новые сведения о клиенте (язык, дальность прорисовки, чат, части скина, основная рука, фильтрация, списки, частицы) во время игры. |
| `client_packet` | клиент | слушает | Пакет фазы игры с сервера дошёл до игры окна и декодирован непосредственно перед обработкой (`Connection.genericsFtw` передаёт его в `ClientPacketListener`): каждый пакет фазы игры, включая пользовательские нагрузки, в том порядке, в котором их отправил сервер. |
| `client_packet_send` | клиент | может отменить | Пакет фазы игры, который игра окна собирается отправить серверу (`Connection.send`): каждый пакет фазы игры, отправляемый окном, в том порядке, в котором он отправляется, включая пакет выбранного слота, отправляемый перед атакой или использованием. |
| `command` | клиент | может отменить | Команда, которую локальный игрок ввёл или выбрал кликом в чате, без слэша, до того как её увидят сервер и команды Horizon: установите `ran`, когда мод её выполнил. |
| `custom_click_action` | сервер | может отменить | `CustomClickActionEvent` NeoForge: клиент серверного игрока отправил действие клика `custom` (кнопку диалога или событие клика текстового компонента): его идентификатор и необязательные данные. |
| `customize_chat_overlay` | клиент | слушает | Игровой чат собирается быть отрисован (`CustomizeGuiOverlayEvent.Chat` NeoForge, в `Gui.renderChat`): `x` и `y` передаются как левый нижний угол чата в пикселях GUI (0 и высота GUI минус 40, ванильное расположение); измените их, чтобы отрисовать чат в другом месте. |
| `resource_pack_status` | сервер | слушает | Клиент игрока на сервере ответил на ресурспак в игре (ванильный `ServerboundResourcePackPacket`): `status` — число `Action`: 0 успешно загружен, 1 отклонён, 2 сбой загрузки, 3 принят, 4 скачан, 5 недопустимый URL, 6 сбой перезагрузки, 7 отброшен. |
| `run_mod_command` | сервер | может отменить | Выполнить разобранную мод-команду после проверок прав и разрешения аргументов. |
| `server_about_to_start` | сервер | слушает | Сервер собирается открыть свои миры: ничего из мира ещё не загружено (NeoForge `ServerAboutToStartEvent`, «вызывается до начала загрузки чего-либо сервером»). |
| `server_chat` | сервер | может отменить | NeoForge `ServerChatEvent`: сообщение игрока в чате, прошедшее через его цепочку и фильтр чата, вот-вот будет разослано. |
| `server_list_ping` | сервер | может отменить | Paper `PaperServerListPingEvent`: клиент запросил у сервера его статус (пинг в списке серверов, запрос статуса в состоянии протокола статуса), и ответ вот-вот будет отправлен. |
| `server_started` | сервер | слушает | Сервер открыл свои миры и вот-вот запустит цикл тиков (NeoForge `ServerStartedEvent`): все миры загружены, ни один игрок не присоединился, и первый тик ещё не прошёл. |
| `server_starting` | сервер | слушает | Вызывается один раз сразу после `server_about_to_start`, до начала приёма игры, на встроенном и выделенном серверах. |
| `server_stopped` | сервер | слушает | Вызывается один раз в конце остановки сервера, после `server_stopping`, последней попытки сохранения и освобождения хранилищ. |
| `server_stopping` | сервер | слушает | Сервер начинает остановку (NeoForge `ServerStoppingEvent`), в том числе после сбоя запуска или цикла тиков: начало ванильного `MinecraftServer.stopServer`, миры ещё загружены, игроки ещё подключены и данные ещё не сохранены, поэтому мод может сохранить свои данные или уведомить их. |
| `tab_complete` | сервер | собирает ответы | Гость попросил дополнить `command`; записи `Suggestion` из API могут добавляться в `suggestions` или отфильтровываться из него. |
| `teleport_command` | сервер | может отменить | Серверная сущность вот-вот будет телепортирована командой `/tp` или `/teleport` (NeoForge `EntityTeleportEvent.TeleportCommand`, в `TeleportCommand.performTeleport`, один раз для каждой телепортируемой сущности): `dimension` — мир, в который она прибывает, `entity` — её сетевой id, `from` — текущая позиция её ног, `to` — куда команда её отправляет. |

#### Клиент: экран, интерфейс, ввод

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `client_pause_change_post` | клиент | слушает | `ClientPauseChangeEvent.Post` в NeoForge: состояние паузы изменилось. |
| `client_pause_change_pre` | клиент | может отменить | `ClientPauseChangeEvent.Pre` в NeoForge: отменить предлагаемое изменение паузы. |
| `component_hover` | клиент | может отменить | `GuiGraphicsExtractor.componentHoverEffect`: наведение на текстовый компонент под курсором, который вот-вот будет нарисован, в чате, книге или диалоге. |
| `customize_boss_event_progress` | клиент | может отменить | Одна полоса босса собирается быть отрисована (`CustomizeGuiOverlayEvent.BossEventProgress` NeoForge, в `BossHealthOverlay`): `bar` — это полоса, `x` и `y` передаются как её левый верхний угол в пикселях GUI (91 левее центра экрана; 12 для первой полосы), а `increment` — как отступ до `y` следующей полосы (19). |
| `debug_entry_lines` | клиент | собирает ответы | Строки одной включённой записи отладки F3, которую зарегистрировал мод (`register_debug_entries`); запрашивается каждый кадр, когда оверлей её рисует. |
| `dimension_transition_screen_draw` | клиент | слушает | Отрисуйте полностью выбранный экран загрузки с его контекстом отрисовки GUI. |
| `framerate_limit` | клиент | слушает | Частота кадров, к которой это окно вот-вот будет ограничено: Max Framerate после ограничения из-за бездействия и режима батареи (260 — без ограничений). |
| `gui_layer_draw` | клиент | слушает | Отрисуйте слой HUD, зарегистрированный модом (`register_gui_layers`), в каждом кадре отображения HUD, между `gui_layer_pre` и `gui_layer_post` этого слоя; не вызывается, когда `gui_layer_pre` мода скрыл его. |
| `gui_layer_post` | клиент | слушает | После одного из ванильных слоёв HUD (или после слоя, скрытого модом), с id `gui_layer_pre`: отрисовывайте здесь, чтобы разместить что-то над слоем (`RenderGuiLayerEvent.Post`). |
| `gui_layer_pre` | клиент | может отменить | Перед отрисовкой одного из ванильных слоёв HUD (`RenderGuiLayerEvent.Pre`) рисуйте здесь, чтобы разместить что-то под слоем, или задайте `canceled`, чтобы скрыть его. |
| `health_hud` | клиент | заменяет результат | Строки HUD здоровья игрока, когда HUD рисует их один раз за кадр (`HealthHud::layout`, не под F1): `inputs` содержит состояние здоровья, еды, воздуха и транспорта, считываемое раскладкой, а `wi`/`hi` — размер GUI экрана. |
| `input_key` | клиент | слушает | Событие клавиши после обработки, `key` окна или сокета управления (одно нажатие, затем одно отпускание для однократного нажатия), включая повторы и перехваченные сочетания клавиш (`InputEvent.Key`). |
| `interaction_key_mapping_triggered` | клиент | может отменить | Маппинг взаимодействия вот-вот подействует на цель прицела этого тика. |
| `key_mapping_pre` | клиент | может отменить | Физическая клавиша или кнопка мыши, привязанная к игровому действию `mapping` (`key.jump`, `key.attack`...), меняет состояние во время игры до того, как игра применит её. |
| `left_click_empty` | клиент | слушает | Локальный игрок левой кнопкой нажимает на пустое пространство при любом стеке в основной руке. |
| `minimap` | клиент | заменяет результат | Миникарта, которую показывает нативный мод: установите `minimap`, если её ещё нет. |
| `mouse_button_post` | клиент | слушает | Принятое сырое событие кнопки завершило диспетчеризацию, включая ранний возврат обработчика экрана. |
| `mouse_button_pre` | клиент | может отменить | Сырой ввод кнопки мыши на любом экране, до диспетчеризации; отмена пропускает диспетчеризацию и Post. |
| `mouse_scrolling` | клиент | может отменить | Игровой скроллинг после накопления, до обработки хотбара или режима наблюдателя. |
| `movement_input` | клиент | слушает | Ввод движения локального игрока после считывания и перед применением. |
| `panel_event` | клиент | слушает | Игрок выполнил `event` на панели мода `mod_id` (`native_panel_items`): реагируйте, если она ваша. |
| `right_click_empty` | клиент | слушает | Локальный игрок нажимает правой кнопкой по пустому месту с пустой `hand`. |
| `screen_char_typed_post` | клиент | может отменить | Напечатанный символ, который ни одно текстовое поле не обработало (`ScreenEvent.CharacterTyped.Post`, возврат false ванильного `Screen.charTyped`), один раз для каждого печатного символа, пропущенного `screen_char_typed_pre`: `c` и `modifiers` как там. |
| `screen_char_typed_pre` | клиент | может отменить | На открытом экране был набран символ (`ScreenEvent.CharacterTyped.Pre`, ванильный `Screen.charTyped`): `screen` — это сам экран, `c` — символ, `modifiers` — биты GLFW (shift 1, control 2, alt 4, super 8), а `field` — копия текстового поля в фокусе. |
| `screen_closing` | клиент | слушает | Заменяется `screen`, который был открыт (`ScreenEvent.Closing`), после того как `screen_opening` его разрешил. |
| `screen_exclusion_areas` | клиент | собирает ответы | Хук `native_screen_exclusion` запрашивает: мод добавляет в `areas` прямоугольники, которые он рисует поверх `screen` (боковая панель, столбец кнопок), в GUI-пикселях, `[x, y, width, height]`. |
| `screen_init_post` | клиент | слушает | Замечено, что экран открылся или изменил прямоугольник, не чаще одного раза за кадр (`ScreenEvent.Init.Post`). |
| `screen_key_pressed_post` | клиент | может отменить | Нажатие клавиши, которое ни один экран не обработал (`ScreenEvent.KeyPressed.Post`, возврат false ванильного `Screen.keyPressed`): `key`, `repeat` и `modifiers` как в `screen_key_pressed_pre`. |
| `screen_key_pressed_pre` | клиент | может отменить | Клавиша была нажата над открытым экраном (`ScreenEvent.KeyPressed.Pre`, ванильный `Screen.keyPressed`): `screen` — это сам экран, `key` — ванильное имя клавиши (`key.keyboard.e`) — клавиша без имени ничего не вызывает — `repeat`, является ли это автоповтором зажатой клавиши с клавиатуры (дополнение Horizon; NeoForge объединяет повторы с событием), и `modifiers` — биты GLFW (shift 1, control 2, alt 4, super 8). |
| `screen_key_released_post` | клиент | может отменить | Отпускание клавиши, которое ни один экран не обработал (`ScreenEvent.KeyReleased.Post`, возврат false ванильного `Screen.keyReleased`): `key` и `modifiers` как в `screen_key_released_pre`. |
| `screen_key_released_pre` | клиент | может отменить | Перед тем как открытый экран обработает отпускание именованной клавиши (та же фильтрация по экрану, что и для нажатий). |
| `screen_mouse_button_pressed_post` | клиент | слушает | Кнопка мыши была нажата над открытым экраном, и экран её обработал (`ScreenEvent.MouseButtonPressed.Post`, возврат ванильного `Screen.mouseClicked`): `screen`, `button` и `modifiers` как в `screen_mouse_button_pressed_pre`. |
| `screen_mouse_button_pressed_pre` | клиент | может отменить | Кнопка мыши была нажата над открытым экраном (`ScreenEvent.MouseButtonPressed.Pre`, ванильный `Screen.mouseClicked`): `screen` — это сам экран, `screen.mouse` — где находится курсор (поэтому отдельной позиции нет), `button` — индекс кнопки по GLFW (0 левая, 1 правая, 2 средняя, 3 назад, 4 вперёд; кнопка с номером выше этих имеет собственный номер платформы, который различается между системами), а `modifiers` — биты GLFW (shift 1, control 2, alt 4, super 8). |
| `screen_mouse_button_released_post` | клиент | слушает | Кнопка мыши была отпущена над открытым экраном, и экран её обработал (`ScreenEvent.MouseButtonReleased.Post`, возврат ванильного `Screen.mouseReleased`), с полями `screen_mouse_button_released_pre`. |
| `screen_mouse_button_released_pre` | клиент | может отменить | Кнопка мыши была отпущена над открытым экраном (`ScreenEvent.MouseButtonReleased.Pre`, ванильный `Screen.mouseReleased`) с теми же `screen`, `button` и `modifiers`, что в `screen_mouse_button_pressed_pre`, и вызывается в той же точке. |
| `screen_mouse_dragged_post` | клиент | слушает | Перемещение курсора с нажатой кнопкой прошло через обработчики перетаскивания открытого экрана (`ScreenEvent.MouseDragged.Post`, ванильный `Screen.mouseDragged`): `button`, `delta` (в GUI-пикселях, `screen.mouse` — новое положение) и `modifiers` как в `screen_mouse_dragged_pre`. |
| `screen_mouse_dragged_pre` | клиент | может отменить | Перед тем как реальное перемещение с нажатой кнопкой достигает обработчиков перетаскивания экрана. |
| `screen_mouse_scrolled_post` | клиент | слушает | Колесо мыши повернулось над открытым экраном, и обработчики прокрутки экрана выполнились (`ScreenEvent.MouseScrolled.Post`, ванильный `Screen.mouseScrolled`): `scroll` и `modifiers` как в `screen_mouse_scrolled_pre`. |
| `screen_mouse_scrolled_pre` | клиент | может отменить | Колесо мыши повернулось над открытым экраном (`ScreenEvent.MouseScrolled.Pre`, ванильный `Screen.mouseScrolled`): `screen` — это сам экран, `screen.mouse` — где находится курсор (поэтому отдельной позиции нет), `scroll` — перемещение колеса по x и y, масштабированное собственным обработчиком прокрутки игры (строка, умноженная на чувствительность колеса: при включённом Discrete Scrolling одно деление равно ±1, только знак; пиксели тачпада делятся на 30), а `modifiers` — биты GLFW (shift 1, control 2, alt 4, super 8). |
| `screen_opening` | клиент | заменяет результат | `Gui.setScreen` собирается заменить экран `current` на `new` (`ScreenEvent.Opening`; экран `playing` отсутствует: ванильный null). |
| `screen_preedit_post` | клиент | слушает | После обработчика предтекстового ввода экрана (`ScreenEvent.Preedit.Post`) и только когда ни одно текстовое поле не приняло набор (обработчик вернул false: ванильное сфокусированное поле принимает его, поэтому это предтекстовый ввод, не нужный ни одному полю) и `screen_preedit_pre` не отменил его. |
| `screen_preedit_pre` | клиент | может отменить | Перед обработчиком предтекстового ввода экрана (`ScreenEvent.Preedit.Pre`, ванильный `GuiEventListener.preeditUpdated`): метод ввода набирает `full_text` (пусто, когда набор завершён), с кареткой на позиции байта `caret_position`, разделённый на `blocks`, среди которых `focused_block` — тот, который преобразуется. |
| `screen_room` | клиент | собирает ответы | Пространство для собственной панели мода справа от окна открытого экрана контейнера, в GUI-пикселях: `right` начинается с 0; увеличьте его на ширину, которую ваша панель занимает рядом с окном, включая промежуток, и в остальных случаях оставьте как есть. |
| `screenshot` | клиент | может отменить | Скриншот F2 считан обратно, перед записью любого PNG (`ScreenshotEvent`). |
| `toast_add` | клиент | может отменить | Клиентское уведомление-тост вот-вот попадёт в свою очередь (`ToastAddEvent`). |
| `window_resize` | клиент | слушает | После изменения размера окна или применения нового масштаба GUI, полноэкранного режима или настройки шрифта Unicode. |

#### Клиент: рендер, звук, частицы

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `arm_pose_transform` | клиент | слушает | Применить зарегистрированную позу (преобразователь `HumanoidModel.ArmPose`). |
| `bake_model_loader` | клиент | собирает ответы | Модель, `loader` которой зарегистрировал мод (`register_model_loaders`), запекается: добавьте её квады в `quads` для загрузчика, принадлежащего вашему моду. |
| `camera_distance` | клиент | слушает | Итоговое масштабированное расстояние камеры третьего лица, до обрезки по коллизии с визуальной формой. |
| `camera_pose` | клиент | слушает | Камера, с которой в этом кадре выполняется отрисовка, `partial` тиков после последнего тика, после её размещения ванилью (`Camera.update`): её положение, а также yaw и pitch в градусах. |
| `camera_shake` | клиент | слушает | Трясение камеры, запрашиваемое нативными модами в этом кадре, `partial` тиков после последнего тика (от 0 до 1): добавьте в `shake` свои pitch, yaw и roll в градусах и смещение в блоках вдоль правого, верхнего и заднего направлений камеры, чтобы трясения нескольких модов складывались. |
| `camera_submersion` | клиент | слушает | Тип тумана погружения камеры (`Camera.getFluidInCamera`), проверяемый перед настройкой тумана и эффектов жидкости для камеры. |
| `camera_type_cycle` | клиент | слушает | `key.togglePerspective` (F5) перебирает типы камеры окна (`Minecraft.handleKeybinds` вызывает `Options.setCameraType(getCameraType().cycle())`): `previous` — тип, который оставляют, а `next` передаётся как следующий ванильный тип (от первого лица, от третьего лица сзади, от третьего лица спереди, затем снова от первого лица); установите `next` на другой, чтобы пропустить тип или сохранить один. |
| `client_resources_reloaded` | клиент | слушает | Уведомление потока рендера после успешного применения перезагрузки ресурспака. |
| `compute_fog_color` | клиент | слушает | Вычисленный RGB тумана мира после обычных эффектов и до загрузки. |
| `compute_fov` | клиент | слушает | Поле зрения мира после обычного вычисления и приближения, перед проекцией/Nausea. |
| `compute_fov_modifier` | клиент | слушает | Масштаб FOV игрока, один раз за тик клиента при наличии игрока, после ванильных эффектов и до приближения и сглаживания. |
| `emissive` | клиент | собирает ответы | Не более 32 пар уже загруженных базовых и прозрачных оверлейных спрайтов. |
| `flip_frame` | клиент | слушает | Один раз на каждый представленный кадр, в мире или вне его, после `render_frame_post` этого кадра (NeoForge `FlipFrameEvent`): место для ротации пофреймовых данных GPU. |
| `get_arm_pose` | клиент | заменяет результат | NeoForge `IClientItemExtensions.getArmPose`: поза руки, держащей `stack` в `hand`, при отрисовке гуманоидной модели (локальных и удалённых игроков и гуманоидных мобов). |
| `hand_drawn` | клиент | слушает | Рисуется рука от первого лица (см. `HandDrawn`); на каждом кадре запускается `world_drawn` и без руки тоже. |
| `lightning_strike` | сервер | может отменить | Готовится создание серверной молнии. |
| `map_decorations` | клиент | собирает ответы | Значки, которые мод помещает на карту `map` каждый раз при её отрисовке: удерживаемая карта, карта в рамке для предметов или на экране картографического стола (пиксели карты принадлежат ей; в самой карте ничего не сохраняется). |
| `model_baking_completed` | клиент | слушает | Все модели загрузки ресурсов запечены и опубликованы (`ModelEvent.BakingCompleted`): модели блоков и предметов, а также отдельные модели, которые запросили моды (`standalone_models`), в хранилище, которое теперь читает `model`. |
| `music` | клиент | слушает | Выбрать музыку для текущего клиента один раз за клиентский тик: `music` и `volume` содержат ванильный выбор (`Minecraft.getSituationalMusic` и `getMusicVolume`: трек биома, креативный или подводный, трек босса или меню; None для тишины). |
| `music_layers` | клиент | собирает ответы | Треки, которые нужно воспроизводить поверх музыки этого клиента (или вместо неё), спрашивается раз в тик клиента: добавьте по одному `MusicLayer` (звуковое событие, зацикленность и громкость от 0 до 1) для каждого трека. |
| `play_sound` | клиент | слушает | `starting` истинно при запусках и перезапусках циклов, ложно при обновлении активного канала. |
| `play_sound_source` | клиент | слушает | Непоточный звук запущен на канале `channel` (`PlaySoundSourceEvent`) после того, как движок задал его высоту, громкость, затухание, цикличность и позицию: `sound` — то, что воспроизводится, после изменений `play_sound`. |
| `play_streaming_source` | клиент | слушает | Поточный звук (музыка, запись) запущен на канале `channel` (`PlayStreamingSourceEvent`): `sound` — то, что воспроизводится, после изменений `play_sound`. |
| `shader_params` | клиент | слушает | Значения, которые нативные моды передают своим шейдер-пакам в этом кадре; `partial` — доли тика с последнего тика (от 0 до 1): в WGSL пак читает их как `frame.mods` (восемь `vec4<f32>`). |
| `sound_engine_load` | клиент | слушает | Звуковой движок открыл вывод (`SoundEngineLoadEvent`): один раз при запуске окна, после каждой перегрузки ресурсов (с `context` нового поколения ресурсов) и при смене выходного устройства. |
| `sprite_source` | клиент | собирает ответы | Элемент определения атласа, чей `type` зарегистрирован модом (`register_sprite_sources`, ванильный `SpriteSource.run`), вызываемый на месте этого элемента в `sources` определения, при каждой загрузке или перегрузке атласа, в ресурсном загрузчике. |
| `standalone_models` | клиент | собирает ответы | Модели без имён состояний блоков, зарегистрированные один раз на клиенте до загрузки ресурсов. |
| `submit_custom_geometry` | клиент | слушает | Отправьте неосвещённую геометрию после отправки частиц, до непрозрачного рендеринга (NeoForge 26.3 `SubmitCustomGeometryEvent`, ванильный `LevelRenderer.submitFeatures`). |
| `texture_atlas_stitched` | клиент | слушает | Атлас был сшит и опубликован (`TextureAtlasStitchedEvent`): `atlas` указывает его имя. |
| `visuals` | клиент | слушает | Локальные в мировых координатах источники окантовочного света и цветного света кадра мира. |
| `zoom` | клиент | заменяет результат | Масштаб, запрошенный нативным модом, применяемый во время игры (приоритет у собственного вида подзорной трубы): установить `zoom`, если он ещё не задан. |

#### Прочее

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `client_mining_predict` | клиент | может отменить | Отклонить предсказанную клиентскую копку после сопоставления взаимодействия и проверок соответствия ванильным правилам. |

### Части блоков (`BlockPart`)

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `adjacent_block_path_type` | сервер | заменяет результат | Необязательный вклад соседнего блока `event.state` в `event.pos` (`IBlockExtension.getAdjacentBlockPathType`). |
| `affect_neighbors_after_removal` | сервер | собирает ответы | `Block.affectNeighborsAfterRemoval` для блока мода: авторитетный сервер удалил блок `state` из `pos` (`moved`: его толкнул поршень) и уже выполнил собственные обновления своего шаблонного класса. |
| `analog_output` | сервер | собирает ответы | `BlockState.getAnalogOutputSignal`: сигнал, считываемый компаратором с блока `state` в `pos`; `face` — сторона, которая касается компаратора (или проводника перед ним). |
| `beacon_color_multiplier` | обе | собирает ответы | Цвет, который блок `state` в `pos` придаёт лучу маяка в `beacon_pos` (`IBlockExtension.getBeaconColorMultiplier`); запрашивается по мере построения секций луча маяка по одному блоку. |
| `block_jump_factor` | обе | слушает | Считывается коэффициент прыжка состояния блока, к которому прикреплена эта часть (`Block.getJumpFactor`, который `Entity.getBlockJumpFactor` читает для игрока, куба, панды, кролика и любого другого живого прыгуна); ванильные значения: 0.5 для мёда и 1.0 для остальных. |
| `block_path_type` | сервер | заменяет результат | Необязательная классификация `event.state` в `event.pos` (`IBlockExtension.getBlockPathType`). |
| `block_render_visibility` | клиент | собирает ответы | Скрывает состояние блока от построения меши: установите `event.hidden`, и состояние ничего не рисует, как если бы его блоки были воздухом. |
| `block_signal` | сервер | слушает | Сигнал редстоуна, который блок `state` в `pos` выдаёт на своей стороне `face`, то есть на стороне, соприкасающейся с читающим блоком: `BlockState.getDirectSignal` при `direct` (что проходит через соприкасающийся блок, как у рычага), иначе `getSignal`. |
| `block_speed_factor` | обе | слушает | Считывается коэффициент скорости состояния блока, к которому прикреплена эта часть (`Block.getSpeedFactor`, который `Entity.getBlockSpeedFactor` читает для игрока, мобов, лодок и вагонеток там, где ваниль его считывает); ванильные значения: 0.4 для мёда и песка душ и 1.0 для остальных. |
| `bounce_on` | сервер | собирает ответы | Сущность отскочила от состояния блока, к которому прикреплена эта часть (`Block.bounceOn`), после падения на `event.fall_distance` блоков. |
| `bounce_restitution` | обе | собирает ответы | Сущность приземлилась на состояние блока, к которому прикреплена эта часть, и спрашивает его упругость (`IBlockExtension.getBounceRestitution`, которую `Entity.getBlockBounciness` спрашивает у блока, на который приземляется сущность): множитель, на который умножается её вертикальная скорость при ударе о блок сверху. |
| `can_be_connected_to` | сервер | заменяет результат | На авторитетном сервере: позволяет ли блок `state` в `pos` соседнему забору, стене, железной решётке или панели `connector` подключиться к нему (`IBlockExtension.canBeConnectedTo`): отвечает принимающий блок, а не присоединяющийся. |
| `can_be_hydrated` | сервер | заменяет результат | Увлажняет ли `fluid` в `fluid_pos` грядку, к которой прикреплена эта часть, то есть блок `state` в `pos` (`IBlockExtension.canBeHydrated`): установите `hydrates` в `Some(true)` или `Some(false)` или оставьте `None` для ванильного правила — жидкость из `#minecraft:water`. |
| `can_drop_from_explosion` | сервер | заменяет результат | На авторитетном сервере: выпадает ли блок `state` в `pos`, разрушенный взрывом `explosion` (`IBlockExtension.canDropFromExplosion`), когда `BlockBehaviour.onExplosionHit` запрашивает `Block.dropFromExplosion`. |
| `can_entity_destroy` | сервер | заменяет результат | На авторитетном сервере: может ли `entity` уничтожить блок `state` в `pos` (`IBlockExtension.canEntityDestroy`); спрашивается там, где ваниль спрашивает своё по-блочное правило: разрушение блока визером (`WitherBoss.canDestroy`: не `#wither_immune`), взрыв опасного черепа визера (сущность — череп, можно прочитать во время взрыва; только у блока, который он может уничтожить, сопротивление ограничено до 0.8), проверка `checkWalls` дракона (блок из `#dragon_transparent` пропускается до запроса к части, а блок из `#dragon_immune` остаётся, даже если ответ положительный), разрушение двери зомби (спрашивается при запуске цели) и выбор цели у `RemoveBlockGoal` черепашьих яиц. |
| `can_harvest_block` | обе | собирает ответы | Собирает ли инструмент игрока `player` блок `state` в `pos` (`IBlockExtension.canHarvestBlock`, то, что `Player.hasCorrectToolForDrops` спрашивает у блока). |
| `can_stick_to` | сервер | собирает ответы | Прилипает ли состояние блока, к которому прикреплена эта часть, к `other`, состоянию рядом (`IBlockExtension.canStickTo`): установите `sticks`, чтобы решить. |
| `can_survive` | сервер | заменяет результат | На авторитетном сервере: `BlockState.canSurvive` для `state` в `pos` (`BlockBehaviour.canSurvive`), до ответа блока: при размещении (`BlockItem.canPlace`), из обновления формы блока, чей шаблон проверяет это (растение, факел, табличка: false разрушает блок с выпадением), и везде, где ещё спрашивает ваниль. |
| `can_survive_v2` | сервер | заменяет результат | `can_survive` снова, для генерации мира сервера: когда функция размещает `state` в `pos`, её части отвечают на `survives` здесь, как они отвечают на `survives` в живой игре. |
| `can_sustain_plant` | сервер | заменяет результат | На авторитетном сервере: `IBlockExtension.canSustainPlant` для почвы `ground` в `ground_pos` и растения `state` над ней; спрашивается до того, как растение ответит на собственный `mayPlaceOn` (`may_place_on`), везде, где ванильное правило растений читает землю: при размещении растения, из его обновления формы и везде, где ещё спрашивает ваниль. |
| `can_sustain_plant_v2` | сервер | заменяет результат | `can_sustain_plant` снова, для генерации мира сервера: когда функция размещает растение `state` на почве `ground` в `ground_pos`, части почвы отвечают на `sustains` здесь. |
| `clone_item_stack` | сервер | заменяет результат | `BlockBehaviour.getCloneItemStack`, вызывается из `ServerGamePacketListenerImpl.handlePickItemFromBlock`. |
| `connects_to` | сервер | собирает ответы | Присоединяется ли ваш забор, стена, железная решётка или стеклянная панель к стороне: установите `connected`, чтобы указать, соединяется ли блок `state` в `pos` с соседним состоянием `neighbour`, где `direction` указывает от блока к этому соседу, а `face_solid` — является ли сторона соседа, обращённая к блоку, прочной. |
| `contents_changed` | сервер | слушает | Только серверное, неотменяемое уведомление базового блока при Container.setChanged / BlockEntity.setChanged после записи и до обновлений dirty/comparator. |
| `dispense_method` | сервер | заменяет результат | `DispenserBlock.getDispenseMethod` перед `dispense_item` предмета: блок, на который смотрит раздатчик, выбирает действие раздачи для любого предмета, включая ванильные предметы, на авторитетном сервере (не для дропперов). |
| `enchant_power_bonus` | обе | собирает ответы | Сила зачарования, которую блок `state` в `pos` даёт столу зачарований в `table_pos` (`IBlockExtension.getEnchantPowerBonus`); запрашивается для каждого смещения книжных полок (`EnchantingTableBlock.BOOKSHELF_OFFSETS`), если блок между этим смещением и столом входит в `#minecraft:enchantment_power_transmitter`. |
| `entity_inside` | сервер | собирает ответы | Блочный обход сущности пересёк состояние блока, к которому присоединена эта часть (`Block.entityInside`), и часть получает уведомление во время обхода, где vanilla выполняет собственный `entityInside` блока. |
| `exp_drop` | сервер | заменяет результат | Опыт до зачарований (`IBlockExtension.getExpDrop`). |
| `explosion_resistance` | сервер | заменяет результат | На авторитетном сервере сопротивление взрыву блока `state` в `pos` против `explosion` (`IBlockExtension.getExplosionResistance`) запрашивается, когда луч взрыва входит в блок (`ExplosionDamageCalculator.getBlockExplosionResistance`). |
| `fall_on` | сервер | собирает ответы | Сущность приземлилась на состояние блока, к которому прикреплена эта часть (`Block.fallOn`), после падения на `event.distance` блоков. |
| `falling_tick` | сервер | слушает | Падающий блок `state` прошёл тик и всё ещё в воздухе (`IFallableExtension.fallingTick`): `entity` — это падающий блок, а `pos` — блок, в котором он находится (`Entity.blockPosition`) после перемещения. |
| `fire_odds` | сервер | заменяет результат | На авторитетном сервере поведение огня для блока `state` в `pos` (`FireBlock.setFlammable(block, igniteOdds, burnOdds)`) запрашивается со стороны `face`, обращённой к огню или воздуху, который может занять огонь: для своих блоков заполните `(ignite, burn)` в `odds`, и ванильная таблица не читается. |
| `friction` | обе | слушает | Сила трения состояния блока, к которому прикреплена эта часть (`IBlockExtension.getFriction`), считывается для сущности, находящейся на нём; значения ванили: 0.6 для большинства блоков, 0.98 для льда, плотного льда и инея, 0.989 для синего льда и 0.8 для слизи. |
| `has_analog_output` | сервер | собирает ответы | `BlockState.hasAnalogOutputSignal`: задайте `output` для вашего блока, который считывается компаратором (насколько он заполнен или продвинут, как уровень компостера), чтобы они запрашивали `analog_output`. |
| `ignited_by_lava` | сервер | заменяет результат | На авторитетном сервере зажигает ли лава огонь рядом с блоком `state` в `pos` (`LavaFluid.isFlammable`, единственный читатель `BlockStateBase.ignitedByLava`), с спрашиваемой стороны `face`, обращённой к воздуху, где огонь появится, при случайном тике лавы, ищущем, что поджечь. |
| `is_bonemeal_success` | сервер | собирает ответы | `BonemealableBlock.isBonemealSuccess` для блока, ответившего на `is_valid_bonemeal_target`, после этого ответа: задайте `success`, чтобы вырастить его в этот раз. |
| `is_fertile` | сервер | собирает ответы | Считается ли почва, к которой прикреплена эта часть (состояние блока `state` в `pos` под растущей культурой), влажной (`IBlockExtension.isFertile`): значение `fertile` определяет ответ. |
| `is_fire_source` | сервер | заменяет результат | На авторитетном сервере поддерживает ли огонь блок `state` в `pos`, находящийся под огнём, чтобы тот горел (`FireBlock.tick` проверяет тег `infiniburn` измерения), спрашивается каждый тик огня над ним: `Some(true)` делает огонь бесконечным, как незерак в обычном мире (дождь не гасит, и он не прогорает), а `Some(false)` отвергает блок, указанный в теге. |
| `is_ladder` | обе | слушает | Поднимается ли живая сущность по состоянию блока, к которому прикреплена эта часть (`IBlockExtension.isLadder`, запрашивается `LivingEntity.onClimbable` у блока, в котором стоит сущность). |
| `is_pathfindable` | сервер | собирает ответы | Изменяет, может ли путь `PathComputationType` (`Land`, `Water` или `Air`) проходить через состояние блока. |
| `is_signal_source` | сервер | слушает | `BlockState.isSignalSource`: задайте `source` для вашего блока, который даёт сигнал, чтобы редстоуновая проволока рядом подключилась к нему. |
| `is_slime_block` | сервер | собирает ответы | Считается ли состояние блока, к которому прикреплена эта часть, блоком слизи для поршня (`IBlockExtension.isSlimeBlock`, который `PistonStructureResolver` спрашивает везде, где он отличает слизь от мёда): установите `slime`, чтобы решить. |
| `is_sticky_block` | сервер | собирает ответы | Является ли состояние блока, к которому прикреплена эта часть, липким для поршня (`IBlockExtension.isStickyBlock`): установите `sticky`, чтобы решить. |
| `is_valid_bonemeal_target` | обе | заменяет результат | Блок, для которого в Horizon нет известного `BonemealableBlock`, вот-вот примет костную муку: ванильный `BonemealableBlock.isValidBonemealTarget`, `state` — собственное состояние блока в `pos`. |
| `item_after_vertical_collision` | обе | может отменить | `native_item_after_vertical_collision` передаёт активный стак брошенного предмета после обработки вертикального столкновения, до эффектов блока, слияния и подбора. |
| `light_emission` | обе | заменяет результат | Свет, который излучает блок `state` в `pos` в `dimension` (`IBlockExtension.getLightEmission`) для блока, объявляющего `dynamic_light: true`. |
| `makes_open_trapdoor_above_climbable` | обе | слушает | Можно ли лазить по открытому люку над состоянием блока, к которому прикреплена эта часть, как по лестнице (`IBlockExtension.makesOpenTrapdoorAboveClimbable`; `LivingEntity.onClimbable` проверяет это у блока под открытым люком, в котором стоит живая сущность). |
| `may_place_on` | сервер | заменяет результат | На авторитетном сервере: `VegetationBlock.mayPlaceOn` для растения `state` на `ground` в `ground_pos`, до ответа растения — блоки, на которых может стоять модовое растение (с шаблоном ванильного растения, например культура). |
| `may_place_on_v2` | сервер | заменяет результат | `may_place_on` снова, для генерации мира на сервере: пока генератор ставит растение `state` на `ground` в `ground_pos`, его части отвечают `may` здесь, а части почвы отвечают `can_sustain_plant_v2` перед этим. |
| `minecart_pass` | сервер | слушает | Вагонетка проехала по состоянию блока рельса, к которому прикреплена эта часть (момент, когда `AbstractMinecart` вызывает `activateMinecart` на активирующем рельсе, для каждого рельса; `IBaseRailExtension.onMinecartPass` из Forge, которого в NeoForge 26.3 уже нет, поэтому у этого хука нет аналога). |
| `neighbor_changed` | сервер | слушает | Авторитетный сервер вот-вот вызовет `neighborChanged`. |
| `on_block_exploded` | сервер | слушает | Взрыв разрушает этот блок, перед удалением и пока его живая блок-сущность доступна для чтения. |
| `on_block_state_change` | обе | слушает | Запись через `Level.setBlock` прошла успешно, и блок в `pos` изменился с `old` на `new` (`IBlockExtension.onBlockStateChange`); сообщение происходит в конце записи, где ваниль выполняет `updatePOIOnBlockStateChange`: после удаления блок-сущности, `on_place`, обновления клиента, обновления соседей и обновлений формы соседей. |
| `on_caught_fire` | сервер | слушает | На авторитетном сервере сосед огня прошёл свой бросок горения. |
| `on_destroyed_by_player` | сервер | заменяет результат | Игрок удаляет блок `state` в `pos` (`IBlockExtension.onDestroyedByPlayer`, где `ServerPlayerGameMode.destroyBlock` вызывает `playerWillDestroy` и удаляет блок); параметр `tool` — стак в руке игрока (`None` для голой руки), а `will_harvest` — выпадет ли добыча при поломке, насколько позволяет инструмент (никогда в креативе и false для инструмента, который блок отвергает). |
| `on_destroyed_by_push_reaction` | сервер | слушает | На авторитетном сервере движение поршня разрушает блок `state` в `pos` (`IBlockExtension.onDestroyedByPushReaction`), где `direction` — направление движения блоков, а `fluid` — состояние жидкости в ячейке. |
| `on_neighbor_change` | сервер | слушает | Соседняя блок-сущность изменилась, была создана или удалена. |
| `on_tree_grow` | сервер | заменяет результат | Размещатель ствола собирается поставить блок под стволом в `pos`, где стоит почвенный `state`, к которому прикреплена эта часть (`IBlockExtension.onTreeGrow`, вызывается из `TrunkPlacer.placeBelowTrunkBlock`). |
| `perform_bonemeal` | сервер | слушает | `BonemealableBlock.performBonemeal` для блока, который подтвердил `is_valid_bonemeal_target` и у которого `is_bonemeal_success` вернул true: выращивайте его здесь, `state` — его состояние в `pos`. |
| `piston_carries_block_entity` | обе | заменяет результат | Несёт ли поршень ваш блок `state` в `pos` вместе с его блок-сущностью: вопрос ставится на последнем шаге `PistonBaseBlock.isPushable`, после реакции блока на толкание, для блока, который в остальном толкаем, когда структура поршня захватывает его. |
| `piston_push_reaction` | обе | заменяет результат | Разрешите реакцию состояния на поршень при загрузке и перезагрузке на обеих сторонах. |
| `play_hit_sound` | клиент | может отменить | Вызывается перед звуком удара при добыче, включая первый удар, каждый четвёртый тик разрушения и маршрут события мира. |
| `rail_can_make_slopes` | сервер | собирает ответы | Может ли рельс `state` в `pos` подниматься к соседнему приподнятому рельсу (`IBaseRailBlockExtension.canMakeSlopes`). |
| `rail_direction` | обе | собирает ответы | Направление пути, которое рельс `state` в `pos` задаёт для `cart` (`IBaseRailBlockExtension.getRailDirection`), поэтому стрелочный путь или односторонний рельс может поворачивать вагонетку иначе, чем её сохранённая форма. |
| `rail_flexible` | сервер | собирает ответы | Может ли рельс `state` в `pos` образовывать повороты (`IBaseRailBlockExtension.isFlexibleRail`). |
| `rail_valid_shape` | сервер | собирает ответы | Может ли рельс `state` принять `shape` (`IBaseRailBlockExtension.isValidRailShape`); вопрос задаётся при соединении с соседями (`RailState`) до записи формы. |
| `random_tick` | сервер | может отменить | У авторитетного сервера `ServerLevel.tickChunk` готовится случайный тик блока `state` в `pos` (`BlockState.randomTick`): только блоки, тикающие случайно, а у мода — так же, как у его ванильного блока-шаблона. |
| `remove_block` | сервер | заменяет результат | Выберите состояние, остающееся после удаления вашего блока. |
| `respawn_position` | сервер | собирает ответы | Где встаёт игрок, чья точка возрождения — блок `state` в `pos`, при возрождении там (`IBlockExtension.getRespawnPosition`), или что там нельзя возродиться. |
| `rotate` | сервер | собирает ответы | Шаблон структуры собирается поставить блок `state` в `pos` с поворотом на `rotation` (`IBlockExtension.rotate`, где ваниль выполняет `state.mirror(mirror).rotate(rotation)`): `state` уже отражён, а начальное значение `rotated` равно ответу самой игры (таблица ванили или шаблона блока мода). |
| `should_check_weak_power` | сервер | собирает ответы | `IBlockExtension.shouldCheckWeakPower`: задайте `check`, чтобы решить, считывается ли косвенное питание через этот блок. |
| `should_display_fluid_overlay` | клиент | собирает ответы | Использовать ли для грани жидкости рядом с состоянием блока спрайт перекрытия жидкости (у воды — `water_overlay`) вместо спрайта течения (`IBlockExtension.shouldDisplayFluidOverlay`): задайте `event.overlay`. |
| `should_hide_adjacent_fluid_face` | клиент | собирает ответы | Скрывать ли состоянию блока грань, которую прилегающая к нему жидкость рисует в его сторону (`IBlockExtension.shouldHideAdjacentFluidFace`): задайте `event.hidden`, и грань жидкости в сторону блока не рисуется. |
| `should_redstone_wire_connect_to` | сервер | собирает ответы | `BlockBehaviour.shouldRedstoneWireConnectTo`: задайте `connected`, чтобы указать, соединяется ли редстоуновый провод с блоком `state` в `pos`. |
| `smoke_source` | сервер | заменяет результат | На авторитетном сервере `CampfireBlock.isSmokeSource` для блока `state` под костром при размещении костра или изменении блока под ним: `Some(true)` для ваших блоков превращает костёр в сигнальный, чей дым поднимается выше, как над стоком сена. |
| `sound_type` | обе | слушает | Тип звука состояния блока, к которому прикреплена эта часть (`IBlockExtension.getSoundType`), читается, чтобы воспроизвести один из его звуков. |
| `state_at_viewpoint` | клиент | собирает ответы | Состояние блока, которое видит камера в `viewpoint` у блока `state` в `pos` (`IBlockExtension.getStateAtViewpoint`), чтобы частичный блок жидкости, резервуар или заполненный водой блок мода показывали подводный туман, только пока глаз действительно находится внутри жидкости. |
| `state_for_placement` | сервер | заменяет результат | Блок вот-вот примет состояние, в котором он размещается (`Block.getStateForPlacement(BlockPlaceContext)`); `state` — его собственное состояние по умолчанию. |
| `tick_block` | сервер | может отменить | `ServerLevel.tickBlock` авторитетного сервера вот-вот выполнит назначенный тик блока `state` в `pos` (`BlockState.tick`): блок всё ещё тот, для которого был запланирован тик. |
| `tool_modified_state` | обе | может отменить | Состояние, в которое переходит блок `state` от действия инструмента (`IBlockExtension.getToolModifiedState`: снятие коры, соскабливание или удаление воска топором; уплотнение или гашение лопатой; вспахивание мотыгой), запрашиваемое у блока, где используется инструмент (`tool.pos`). |
| `update_shape` | сервер | заменяет результат | Авторитетный сервер собирается вычислить, каким состоянием станет блок `state` в `pos`, так как соседний блок в `direction` теперь `neighbour` (`BlockBehaviour.updateShape`). |
| `use_item_on` | сервер | заменяет результат | `BlockBehaviour.useItemOn`: поведение самого блока с удерживаемым предметом на сервере для каждого игрока. |
| `use_without_item` | сервер | заменяет результат | `BlockBehaviour.useWithoutItem`: метод пустой руки захваченного состояния, после того как useItemOn его запросил, на сервере и только в основной руке. |
| `weak_changes` | сервер | собирает ответы | Получает ли этот блок изменения блок-сущности через соседний проводник. |

### Части блок-сущностей (`BlockEntityPart`)

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `block_entity` | обе | заменяет результат | Начальные сохраняемые данные для объявленной блок-сущности (`EntityBlock.newBlockEntity`); вызывается внутри записи, создающей её (`LevelChunk.setBlockState`), на сервере и на клиенте одного экземпляра. |
| `block_entity_client_tick` | клиент | слушает | Клиентский тик этой клиентской копии блок-сущности мода в `pos` (`ClientLevel.tickBlockEntities`), когда её `id` ваш и состояние блока тикает на клиенте (`block_entity_client_ticks`): один раз за клиентский тик, пока мир работает, в ванильном порядке тикера среди остальных, рисуется ли копия или нет. |
| `block_entity_client_ticks` | клиент | слушает | Тикает ли блок-сущность блока `block` на клиенте (`EntityBlock.getTicker` вызывается с клиентским миром): установите `ticks`, когда она ваша, и для неё `block_entity_client_tick` выполняется один раз за клиентский тик. |
| `block_entity_collect_components` | сервер | заменяет результат | Компоненты предмета, которые блок-сущность в `pos` хранит в своих данных, когда её `id` ваш (`BlockEntity.collectImplicitComponents`): установите `components` по ним, прочитав только из `entity` (еда в горшке, содержимое шкафа). |
| `block_entity_data_received` | клиент | слушает | Синхронизированный `tag` был объединён с блок-сущностью этого клиента. |
| `block_entity_on_load` | обе | слушает | Блок-сущность мода `entity` добавлена в её мир с уже применёнными сохранёнными данными перед её первым тиком (`IBlockEntityExtension.onLoad`). |
| `block_entity_render_bounding_box` | клиент | собирает ответы | Где рисуется блок-сущность мода в `pos`, для отсечения (`getRenderBoundingBox` NeoForge): `bounds` — min x, y, z, затем max x, y, z, в блоках от нижнего северо-западного угла блока. |
| `block_entity_submit` | клиент | слушает | Рисует блок-сущность мода в `pos`, блока `block` (`mymod:machine`), на этом клиенте: раз в кадр, пока она в поле зрения (ванильный `BlockEntityRenderer.submit`: в пределах её дистанции видимости, 64 блока, если `block_entity_view_distance` не указал иначе, её чанк проявлен, а её границы, `block_entity_render_bounding_box`, в усечённой пирамиде), когда её `id` принадлежит вам. |
| `block_entity_tick` | сервер | слушает | Серверный тик блок-сущности в `pos` в `level`, в ванильном порядке среди других (`Level.tickBlockEntities`), когда её `id` принадлежит вам. |
| `block_entity_ticks` | сервер | слушает | Тикает ли блок-сущность блока `block` (`EntityBlock.getTicker`): установите `ticks`, когда она ваша. |
| `block_entity_update_tag` | сервер | заменяет результат | Что клиенты получают от блок-сущности в `pos`, когда её `id` ваш (`BlockEntity.getUpdateTag`): установите `tag` на неё, читая только из `entity`. |
| `block_entity_view_distance` | клиент | собирает ответы | На каком расстоянии от камеры рисуется блок-сущность блока `block`, в блоках (ванильный `BlockEntityRenderer.getViewDistance`): 64, если ни одна часть не ответила. |
| `menu_button` | сервер | слушает | Игрок нажал кнопку контейнера мода (`clickMenuButton`) на сервере. |
| `menu_quick_move` | сервер | может отменить | Игрок на сервере нажал Shift на слоте `slot` меню блок-сущности мода в `pos` (`AbstractContainerMenu.quickMoveStack`, метод, который переопределяет Java-меню), до собственной маршрутизации игры: переместите стак туда, куда хочет ваш блок. |
| `on_chunk_unloaded` | обе | слушает | Чанк, содержащий эту блок-сущность мода, выгружается (`IBlockEntityExtension.onChunkUnloaded`): вызывается один раз для каждой его блок-сущности мода на каждой стороне, содержащей её копию, до отбрасывания копии. |
| `pre_remove_side_effects` | сервер | собирает ответы | `BlockEntity.preRemoveSideEffects` модовой блок-сущности: авторитетный сервер удаляет `entity` из `pos`, где стоял блок `state`, и её обновления могут иметь побочные эффекты (не под флагом 256, `UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS`). |
| `screen` | обе | заменяет результат | На этом клиенте открывается меню для блока `block` из мода: задайте `screen`, если этот блок ваш, чтобы расположить элементы его интерфейса (`screen_draw` его рисует). |
| `screen_click` | клиент | заменяет результат | Левое или правое (`right`) нажатие в позиции `at` (в GUI-пикселях от левого верхнего угла экрана, может быть и за его пределами) на экране блока `block` из мода, не по слоту: присвойте `button` идентификатор кнопки меню, если нажатие приходится на вашу кнопку. |
| `screen_draw` | клиент | слушает | Рисуйте экран блока `block` в `origin` (его левый верхний угол, GUI-пиксели), поверх простой панели с рамками слотов и под предметами и подписями, из слотов данных меню `data` (`data` блок-сущности на сервере): слоты данных передаются по сети как 16-битные, как в ванили (значения больше диапазона заворачиваются), и `data` пуст до поступления первых серверных значений, поэтому считывайте его с помощью `get`. |
| `screen_tooltip` | клиент | собирает ответы | Подсказка в точке `at` (как в `screen_click`) на экране блока `block` из мода, не над предметом: добавьте её строки, когда она находится над чем-то вашим. |

### Части предметов (`ItemPart`)

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `apply_enchantments` | сервер | слушает | NeoForge `IItemExtension.applyEnchantments`: стол зачарования применяет `enchantments`, список, выбранный им для нажатой строки в виде (id, level), к `stack` — предмету в его входном слоте (`EnchantmentMenu.clickMenuButton`). |
| `can_equip` | обе | слушает | NeoForge `IItemExtension.canEquip`: может ли `stack` попасть в слот экипировки `slot` живой сущности `entity` типа `kind` (`minecraft:player`)? |
| `can_fit_inside_container_items` | обе | слушает | NeoForge `IItemExtension.canFitInsideContainerItems`: может ли `stack` поместиться внутри предмета, который хранит предметы? |
| `can_grindstone_repair` | обе | слушает | NeoForge `IItemExtension.canGrindstoneRepair`: чинит ли точильный камень `stack` так же, как он чинит предмет с прочностью? |
| `can_perform_action` | обе | слушает | NeoForge `IItemExtension.canPerformAction`: может ли `stack` выполнить способность предмета `ability` — простое регистрозависимое имя, например `shield_block`, `shears_harvest` или `shears_carve` (собственные способности мода тоже используют простые имена, например `my_mod_chop`)? |
| `contained_fluid` | клиент | слушает | Жидкость, которую удерживает стак собственного контейнерного предмета мода, как её отображает модель предмета `neoforge:fluid_container` (`FluidUtil.getFirstStackContained`): `fluid` приходит со значением `None` (не содержит жидкости); установите его в ключ жидкости, полученный один раз при регистрации с помощью `r.fluid_key`. |
| `crafting_remainder` | сервер | слушает | `IItemExtension.getCraftingRemainder(ItemStack)` в NeoForge: что оставляет `stack`, когда он используется в крафте, как топливо печи или при зельеварении. |
| `creator_mod_id` | обе | собирает ответы | `IItemExtension.getCreatorModId`: мод, создавший `stack`; его показывает просмотрщик рецептов или подсказка, а не мод, зарегистрировавший предмет. |
| `damage_item` | сервер | может отменить | `IItemExtension.damageItem`: `stack` собирается получить износ (`ItemStack.hurtAndBreak`: инструмент, который копает или бьёт, броня, получающая удар, щит, удилище, элитры, инструмент, используемый раздатчиком, броня моба), после влияния Unbreaking, как передаёт NeoForge. |
| `dispense_item` | сервер | заменяет результат | `DispenserBlock.getDispenseMethod`: выбирает поведение для любого предмета, включая ванильные предметы, на авторитетном сервере (не для дропперов). |
| `empty_contents` | сервер | заменяет результат | `IDispensibleContainerItemExtension.emptyContents`: `stack` (ведро жидкости мода, банка или канистра) собирается опорожниться в `pos` игроком `player` (действующим игроком), кликнувшим по `hit` (блок и сторона), или раздатчиком (`player` и `hit` — `None`). |
| `get_equip_sound` | сервер | слушает | NeoForge `IItemExtension.getEquipSound`: звуковое событие, которое `stack` воспроизводит при экипировке в слоте `slot` (`head`, `chest`, `legs`, `feet`, `offhand`, `mainhand` или собственном названии слота мода, например `curios:ring`). |
| `highlight_tip` | клиент | слушает | NeoForge `IItemExtension.getHighlightTip`: название, отображаемое над хотбаром, когда выбран `stack`. |
| `hurt_enemy` | сервер | слушает | Удар игрока попал по мобе `entity` типа `kind` (`minecraft:zombie`) с `stack` в руке: там, где вызывается `Item.hurtEnemy`, до зачарований после атаки и износа оружия. |
| `inventory_tick` | сервер | слушает | Серверный тик для `stack` — стака, который игрок держит в `slot`: там, где `Inventory.tick` вызывает `Item.inventoryTick`, для каждого стака, каждый тик, включая `/tick freeze` (останавливает мир, а не игрока). |
| `is_combine_repairable` | обе | слушает | NeoForge `IItemExtension.isCombineRepairable`: можно ли объединить два предмета из `stack` в сетке крафта, чтобы починить его (`RepairItemRecipe.canCombine`)? |
| `is_not_replaceable_by_pick_action` | сервер | слушает | NeoForge `IItemExtension.isNotReplaceableByPickAction`: должен ли выбор блока (клавиша pick-block) оставить `stack` на его текущем месте в хотбаре, в `slot` (от 0 до 8)? |
| `is_piglin_currency` | обе | слушает | NeoForge `IItemExtension.isPiglinCurrency`: является ли `stack` валютой для бартера с пиглином (`PiglinAi.isBarterCurrency`)? |
| `is_primary_item_for` | сервер | слушает | NeoForge `IItemExtension.isPrimaryItemFor`: является ли предмет `stack` основным для `enchantment` (id, например `minecraft:sharpness`), чтобы выбор зачарования мог его предложить? |
| `item_bar` | клиент | слушает | The small bar `GuiGraphicsExtractor.itemDecorations` draws under an item, wherever the game shows one: `bar` arrives holding vanilla's value (the damage bar, or a bundle's fullness already applied) and may be replaced (width `0..=13`, colour `rgb`) or set to `None` to hide it. |
| `item_decorations` | клиент | слушает | Добавить пользовательские украшения предмета после его стандартной полоски, времени восстановления и количества. |
| `item_interact_mob` | обе | заменяет результат | `ItemStack.interactLivingEntity`: после успешного прохождения обычных взаимодействий с сущностями, для живого нативного моба и непустого стака в `hand`. |
| `makes_piglins_neutral` | сервер | слушает | Хук NeoForge `IItemExtension.makesPiglinsNeutral`: заставляет ли `stack`, надетый на `wearer`, спокойных пиглинов не выбирать его целью, как это делает золотая броня (`PiglinAi.isWearingSafeArmor`)? |
| `on_block_placed_by` | сервер | слушает | `BlockItemExtension.onBlockPlacedBy`: блок этого предмета был размещён, его финальное состояние блока и данные блок-сущности стака применены, а `Block.setPlacedBy` выполнен (включая блоки-компаньоны). |
| `on_dropped_by_player` | сервер | может отменить | `IItemExtension.onDroppedByPlayer`: `player` (дескриптор игрока, выполняющего действие) нажал клавишу выброса для `stack` — всего выбранного стака до его изменения; `all` равно true при выбросе всего стака и false для одного предмета (`ServerPlayer.drop(boolean)`). |
| `on_entity_item_update` | сервер | может отменить | `IItemExtension.onEntityItemUpdate`: каждый тик выброшенной сущности предмета, содержащей `stack`, после `entity_tick_pre` и до собственного тика сущности (`ItemEntity.tick`: её жидкости, движение, задержка подбора, объединение и срок жизни). |
| `on_entity_swing` | обе | может отменить | `IItemExtension.onEntitySwing`: игрок на месте (`entity`, его `Entity.getId`, если известен) вот-вот взмахнёт `stack` в `hand`: там, где начинается `LivingEntity.swing`, для атаки, использования или взмаха, запрошенного сервером. |
| `on_item_use_first` | сервер | заменяет результат | `IItemExtension.onItemUseFirst`: первое решение удерживаемого предмета при клике по блоку, на сервере для каждого игрока, после того как `use_item_on_block` (`ItemBeforeBlock`) пропустил его, и до собственного `useItemOn` блока. |
| `on_left_click_entity` | сервер | может отменить | `IItemExtension.onLeftClickEntity`: `player` (UUID действующего игрока), держащий `stack` в основной руке, вот-вот атакует сущность `entity` (`Entity.getId`) типа `kind` (`minecraft:cow`, `minecraft:player`, `minecraft:item_frame`): там, где начинается `Player.attack`, после того как глобальный `attack_entity` пропустил удар, и до любого урона, отбрасывания, эффекта зачарования, `hurt_enemy` или износа оружия. |
| `on_use_tick` | обе | слушает | `Item.onUseTick` для `stack`, используемого `entity`, с остатком `remaining` тиков, всегда больше 0: что собственный предмет мода делает каждый тик использования (готовка, зарядка, истощение). |
| `release_using` | обе | слушает | `Item.releaseUsing` для `stack`: `entity` отпускает его, когда из `use_duration` остаётся `time_left` тиков, поэтому он использовался `duration - time_left` (лукоподобный предмет стреляет с учётом этого заряда). |
| `should_cause_block_break_reset` | клиент | слушает | NeoForge `IItemExtension.shouldCauseBlockBreakReset`: должно ли ломание сброситься из-за смены основной руки? |
| `should_cause_reequip_animation` | клиент | слушает | NeoForge `IItemExtension.shouldCauseReequipAnimation`: должна ли рука от первого лица опуститься и снова подняться, чтобы показать `stack`, текущий стак в руке? |
| `tooltip` | клиент | собирает ответы | `Item.appendHoverText`: собственные строки предмета во всплывающей подсказке, после названия и перед строками его компонентов (зачарования, описание, атрибуты). |
| `tooltip_component` | клиент | заменяет результат | Клиентский компонент изображения для подсказки этого стака. |
| `tooltip_component_draw` | клиент | слушает | Отрисуйте измеренный компонент в начале координат, внутри фона подсказки. |
| `use_animation` | клиент | слушает | `Item.getUseAnimation` для `stack` во время использования, для рендера от первого и третьего лица. |
| `use_duration` | обе | слушает | `Item.getUseDuration` для `stack`, в тиках: `duration` приходит с ванильным значением (время использования, 72000 для щита, копья, лука, 0 для предмета, который не используется длительное время). |
| `use_item` | обе | заменяет результат | Игрок использует `stack` (правой кнопкой) через `self.hand`: в воздухе или по блоку, когда `native_use_on` пропустил действие; здесь начинается `Item.use`. |
| `use_on` | обе | заменяет результат | Игрок использует `stack` (правой кнопкой) через `self.hand` на блоке в `pos`, `face` — нажатая грань (`[0, 1, 0]` — верхняя), когда сам блок позволил клику пройти (сундук всё равно откроется): здесь начинается `ItemStack.useOn`. |
| `xp_repair_ratio` | сервер | слушает | `IItemExtension.getXpRepairRatio`: очки прочности, восстанавливаемые `stack` за единицу опыта Починкой, где `ExperienceOrb.repairPlayerItems` чинит выбранный повреждённый предмет с Починкой, один раз на каждый чинимый предмет. |

### Части сущностей (`EntityPart`)

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `general_entity_tick` | сервер | слушает | Авторитетный тик одной общей строки сущности типа `kind` (`mymod:robot`) с ID `entity` на фазе сущностей серверного мира: `entity.tick()` из `ServerLevel.tickNonPassenger` для строки, которой владеет реестр типов (arismoko/horizon#792 этап 4). |

### Части меню (`MenuPart`)

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `menu_slot_rules` | обе | слушает | Размещение слотов и лимиты стака для зарегистрированных ванильных и модовых меню; ездовые существа используют `horizon:mount_inventory`, а инвентарь игрока и творческий выбор не имеют зарегистрированного типа. |
| `menu_still_valid` | сервер | слушает | Открыто ли меню блока игрока типа модового меню (`AbstractContainerMenu.stillValid`). |
| `menu_transfer` | сервер | заменяет результат | Кнопка «+» просмотрщика рецептов на модовом меню (у `IRecipeTransferHandler` из JEI): переместить один набор ингредиентов `recipe` (сколько позволяют предметы с `max`) из инвентаря `player` в открытое меню типа модового меню, либо при `dry_run` только сказать, что было бы сделано. |

### Части жидкостей (`FluidPart`)

| Хук | Сторона | Контракт | Что делает |
| --- | --- | --- | --- |
| `fluid_rules` | сервер | слушает | Как мировая жидкость мода течёт в `pos`, начиная с того, что она объявила (геттеры NeoForge `BaseFlowingFluid` и `FluidType`: `getTickDelay`, `getDropOff`, `getSlopeFindDistance`, `canConvertToSource`). |

## Методы Host

`Host` — то, что хук может спросить или сделать в игре. Методы `ServerHost` доступны только
на сервере, `ClientHost` — только на клиенте, `Host` — на обеих сторонах.

### `advancements`

| Метод | Где | Что делает |
| --- | --- | --- |
| `advancement` | `ServerHost` | `ServerAdvancementManager.get(id)`: загруженное достижение `id` из набора, созданного последней загрузкой датапака или `/reload` (загружается при первом использовании). |

### `block_entities`

| Метод | Где | Что делает |
| --- | --- | --- |
| `block_entity` | `Host` | Найти блок-сущность в загруженном чанке открытого мира, или на клиенте в отображаемом мире. |
| `block_entity_identity` | `Host` | Дескриптор, который даёт `BlockEntitiesHost::block_entity`, а также для блок-сущности вне мира для её собственного тика, чья идентичность и вложения пока остаются на месте: то, с чем её состояние части (`BlockEntityPartState`) проверяет дескриптор. |
| `block_entity_sync` | `ServerHost` | Повторно отправляет клиентам блок по `pos` и тег обновления его блок-сущности (`block_entity_update_tag`), как это делает `Level.sendBlockUpdated`: вместе с другими изменениями блоков в мире, игрокам, которые видят этот чанк, один раз, как бы часто его ни вызывали до этого. |

### `block_states`

| Метод | Где | Что делает |
| --- | --- | --- |
| `lit_block_name` | `Host` | Прочитать загруженный излучающий свет блок. |
| `fluid_key` | `Host` | Разрешить зарегистрированный идентификатор жидкости. |
| `block_state_id` | `Host` | Состояние, названное `name`, разобранное так, как `/setblock` разбирает блок (`BlockStateParser.parseForBlock`): `minecraft:oak_stairs[facing=east]`, `stone` для `minecraft:stone`, пропущенные свойства — по умолчанию. |
| `block_state_string` | `Host` | Состояние `state` в виде, который `/setblock` принимает обратно (`BlockStateParser.serialize`): его блок, затем каждое свойство по порядку имён, `minecraft:oak_stairs[facing=east,half=bottom,...]`, или только блок, если у него нет свойств. |
| `block_state_default` | `Host` | Состояние по умолчанию разрешённого ключа `block`: `Block.defaultBlockState`, которое принимает блок, когда ничего другого не указано. |
| `item_crafting_remainder` | `Host` | `Item.getCraftingRemainder`: Horizon сейчас хранит идентификатор предмета остатка, поэтому результат — один обычный стак. |
| `block_item` | `Host` | `Block.asItem`: идентичность предмета блока, включая воздух для блока без предмета; None, если недоступна любая из разрешённых идентичностей. |
| `item_block` | `Host` | Блок, который размещает `item` (`BlockItem.getBlock`, как его читает `Block.byItem`): `minecraft:wheat_seeds` размещает `minecraft:wheat`, а предмет стоячего или настенного блока размещает его стоячий блок, а не вариант на стене. |
| `set_block_property` | `ServerHost` | Устанавливает свойство `key` блока в `pos` в `value`, как тик печи устанавливает её `lit` (`Level.setBlock` с `UPDATE_ALL`: соседи и клиенты получают уведомление); true, если блок изменился, false, если у блока нет такого свойства или значения, или свойство уже имеет `value` (как `Level.setBlock` для неизменённого состояния). |
| `set_block_property_with` | `ServerHost` | `native_set_block_property` с `flags` из `Level.setBlock`, как тик варочной стойки устанавливает свои бутылки с `flags::UPDATE_CLIENTS`. |

### `boss_bars`

| Метод | Где | Что делает |
| --- | --- | --- |
| `boss_bar` | `ServerHost` | Существующая пользовательская полоса босса по идентификатору (`minecraft:` без пространства имён) или None. |
| `boss_bars` | `ServerHost` | Все пользовательские полосы боссов в порядке, который показывает `/bossbar list`. |
| `create_boss_bar` | `ServerHost` | `/bossbar add`: новая полоса, видимая, белая, `progress`, с 0 из 100, без добавленных игроков. |

### `client_input`

| Метод | Где | Что делает |
| --- | --- | --- |
| `claim_items` | `ClientHost` | Правые клики этими предметами (не более 64 идентификаторов) принадлежат моду `mod_id`; последующий вызов заменяет список. |
| `poll` | `ClientHost` | События мода `mod_id` с последнего опроса (в очереди не более 256). |
| `cursor` | `ClientHost` | Где находится мышь, в пикселях интерфейса. |
| `mod_key` | `ClientHost` | Сохранённая клавиша и модификаторы для привязанной к владельцу регистрации или их объявленное значение по умолчанию. |
| `is_down` | `ClientHost` | Удерживается ли зарегистрированная раскладка `id` мода `mod_id` (`KeyMapping.isDown`): её клавиша или кнопка мыши нажата, а контекст конфликта и модификаторы активны. |
| `consume_click` | `ClientHost` | Отнимает один клик у зарегистрированной раскладки `id` мода `mod_id` (`KeyMapping.consumeClick`): true, если он у неё был. |
| `playing` | `ClientHost` | Идёт игра, поверх мира ничего нет: применяются клавиши и масштаб. |
| `mapping` | `ClientHost` | Ванильная раскладка (`key.forward`, `key.jump`, `key.attack`…): удерживается ли она (`KeyMapping.isDown`) и имя её клавиши, как оно показано в «Управлении». |
| `set_mapping_down` | `ClientHost` | Удерживает ванильную игровую раскладку `key` нажатой или отпускает (`KeyMapping.setDown`): удерживаемое состояние, соответствующее её клавише, без клика, добавляемого нажатием. |
| `click_mapping` | `ClientHost` | Добавляет один клик ванильной игровой раскладке `key` (`KeyMapping.click`), как нажатие её клавиши: атака, использование или выбор блока, начатые кликом, выполняются на последующем клиентском тике, с учётом проверок `startAttack` и `startUseItem` (задержка промаха, занятые руки, предмет, цель под перекрестием) и пакетов, которые отправляет собственный клик игрока; у клавиш инвентаря — выбор слота хотбара, выброс, инвентарь или смена руки. |
| `millis` | `ClientHost` | Миллисекунды на внутренних часах игры (`Util.getMillis`), отсчитываемые от начала игры: по ним мигает курсор поля ввода и отличают двойной клик от двух кликов. |
| `modifiers` | `ClientHost` | Текущие модификаторы клавиш окна в той же кодировке GLFW, что и для хуков ввода. |
| `clipboard_text` | `ClientHost` | Текст в буфере обмена (`KeyboardHandler.getClipboard`), пустой, если его нет, во время обработки игрой события клавиши, мыши или вводимого текста: вставка в собственное поле хуком клавиши или экрана мода. |
| `set_clipboard_text` | `ClientHost` | Помещает `text` в буфер обмена (`KeyboardHandler.setClipboard`) на тех же условиях, на которых `native_clipboard_text` его читает. |

### `client_narrator`

| Метод | Где | Что делает |
| --- | --- | --- |
| `narrate` | `ClientHost` | Заставляет диктора произнести `text` как системное сообщение через тот же `GameNarrator`, который используется собственной озвучкой игры: `Narration::Now` — это `saySystemNow` (прерывает то, что произносится), `Narration::Queued` — это `saySystemQueued`. |

### `client_options`

| Метод | Где | Что делает |
| --- | --- | --- |
| `input_options` | `ClientHost` | Снимок текущих предпочтений ввода. |
| `graphics_options` | `ClientHost` | Снимок текущих предпочтений графики. |
| `gui_options` | `ClientHost` | Снимок текущих предпочтений интерфейса и доступности. |
| `gui_scale` | `ClientHost` | Действующий масштаб интерфейса (`Window.getGuiScale`), не меньше 1; отсутствует на сервере или если нет окна. |
| `chat_options` | `ClientHost` | Снимок текущих предпочтений чата. |
| `audio_options` | `ClientHost` | Снимок текущих предпочтений звука. |

### `client_render`

| Метод | Где | Что делает |
| --- | --- | --- |
| `instance_group_write` | `ClientHost` | Атомарно перезаписывать сохранённые копии моделей; используйте `InstanceGroupRef::write`. |
| `instance_group_set_len` | `ClientHost` | Задаёт рисуемую длину; используйте `InstanceGroupRef::set_len`. |
| `instance_group_set_transform` | `ClientHost` | Замените позу группы; используйте `InstanceGroupRef::set_group_transform`. |
| `instance_group_destroy` | `ClientHost` | Освободите удержанные копии модели; используйте `InstanceGroupRef::destroy`. |
| `refresh_render_visibility` | `ClientHost` | Снова запрашивает `block_render_visibility` для каждого состояния и, если ответы отличаются от текущих, перестраивает сетку каждой загруженной секции в соответствии с ними. |
| `refresh_biome_colors` | `ClientHost` | Снова запрашивает `biome_colors` для каждого биома и, если цвета отличаются от текущих, перестраивает каждую загруженную секцию и дальнюю местность с учётом этих цветов (`LevelRenderer.allChanged`, как при изменении Biome Blend). |
| `mod_image` | `ClientHost` | Собственное RGBA-изображение мода, загруженное в слот атласа HUD `MOD_IMAGE_SPRITE` и рисуемое через `native_sprite`: собственный исходный прямоугольник обрезает и растягивает его. |
| `block_tint_color` | `ClientHost` | Для клиентского мода, самостоятельно рисующего грани блока (копия блока, рисующая грани своего материала, из `block_sprites`): цвет 0xRRGGBB, на который умножается грань `state` с индексом окрашивания `tint_index` (`FaceSprite::tint_index`) в `pos`, так же как сборщик меша чанка окрашивает её там (`BlockColors.getColor`, включая смешение биомов: трава, листва и вода следуют биому вокруг `pos`). |
| `block_sprites` | `ClientHost` | Публикуемая операция чтения ассетов/данных: доступна в Setup, Reloading и World. |
| `texture_sprites` | `ClientHost` | Публикуемая операция чтения ассетов/данных: доступна в Setup, Reloading и World. |
| `gui_sprite` | `ClientHost` | Публикуемая операция чтения ассетов/данных: доступна в Setup, Reloading и World. |
| `block_sprites_version` | `ClientHost` | Публикуемая операция чтения ассетов/данных: доступна в Setup, Reloading и World. |

### `client_text`

| Метод | Где | Что делает |
| --- | --- | --- |
| `text_string` | `ClientHost` | `Component.getString()`: преобразует компонент в обычные символы, включая переводы на текущий язык, аргументы, имена ключей и соседние компоненты. |
| `text_visual_order` | `ClientHost` | `Component.getVisualOrderText()`: стилизованные символы на текущем языке в визуальном порядке, включая формирование и зеркальное отражение двунаправленного текста. |
| `text_width` | `ClientHost` | `Font.width(FormattedText)`: ширина компонента в неизмасштабированных пикселях интерфейса с учётом текущего языка, стилей и шрифтов ресурспака. |
| `text_split` | `ClientHost` | `Font.split(FormattedText, width)`: переносит компонент по `width` неизмасштабированных пикселей интерфейса. |

### `client_view`

| Метод | Где | Что делает |
| --- | --- | --- |
| `block_collision` | `Host` | Коллизийные ящики в локальных координатах блока, полученные с пустым контекстом столкновений. |
| `hud_mode` | `ClientHost` | Считывает фактическую видимость HUD независимо от отображения рук от первого лица. |
| `native_corner_shown` | `ClientHost` | Указывает, оставляет ли отладочный экран F3 `corner` свободным в этом кадре для окна, размещаемого `render_gui_post`: false для углов, которые он занимает (`DebugScreenOverlay`), для обоих верхних углов, пока он открыт (его левая и правая колонки), и для нижнего угла, пока один из его графиков рисуется там. |
| `set_hud_mode` | `ClientHost` | Выбирает видимость HUD и рук, включая состояние после ванильной обработки ввода F1. |
| `camera_type` | `ClientHost` | Тип камеры окна (`Options.getCameraType`), включая последний отложенный запрос. |
| `set_camera_type` | `ClientHost` | Запрашивает тип камеры окна без переключения, в том числе из колбэков общего хоста. |
| `is_paused` | `ClientHost` | Применённое состояние паузы клиента, а не открытость экрана паузы. |
| `clock` | `ClientHost` | Часы мира, отображаемого этим окном, для клиентских модов: игровое время и время суток в том виде, как сервер отправил их в последний раз (`ClientboundSetTimePacket`), продвигаемые на каждый тик между пакетами; на начальном экране отсутствуют. |
| `raycast` | `ClientHost` | Первый блок вдоль луча обзора в пределах `distance` (не более 1024), включая далёкую местность за загруженными чанками. |
| `view_hit` | `ClientHost` | Первый блок вдоль луча обзора в пределах `RAYCAST_MAX` и расстояние до него. |
| `camera` | `ClientHost` | Камера последнего извлечённого кадра мира, если он существует. |
| `project` | `ClientHost` | Проекция точки мира `at` на экран этого кадра (`GameRenderer.projectPointToScreen`): нормализованные экранные координаты x и y от -1 до 1 поперёк экрана (x вправо, y вверх) и глубина z (инвертированная: чем ближе, тем больше). |
| `local_item_use` | `ClientHost` | Использование предмета локальным игроком в том виде, в каком его видит клиент: рука, использованные тики и оставшиеся тики (`isUsingItem`, `getUsedItemHand`, `getTicksUsingItem`, `getUseItemRemainingTicks`). |
| `set_local_rotation` | `ClientHost` | Поворачивает взгляд локального игрока на `yaw` и `pitch` (в градусах), как их сохраняют `setYRot` и `setXRot`: yaw как задано (без оборачивания; сервер оборачивает), pitch по `% 360`, затем ограничивается до -90..90. |
| `camera_entity` | `ClientHost` | Сущность, за которой следует обзор (`Minecraft.getCameraEntity`): сущность, к которой сервер прикрепил камеру, иначе локальный игрок. |
| `client_surface_heights` | `ClientHost` | Высота каждого столбца по `heightmap` в мире, отображаемом этим окном (`LevelReader.getHeight(type, x, z)` для клиентского мира): на единицу выше самого высокого блока, учитываемого на этой карте, из отправленных сервером блоков; отсутствует для столбца, чанк которого этот клиент не держит. |
| `client_far_surface` | `ClientHost` | Для клиентского мода, рисующего карту за загруженными чанками: поверхность каждого столбца в том виде, в котором её хранит дальняя местность этого окна (настройка LOD Distance), независимо от загрузки чанка. |
| `set_camera_entity` | `ClientHost` | Направляет обзор на `entity` (`Minecraft.setCameraEntity`), как пакет камеры от сервера; собственная ссылка локального игрока возвращает обзор обратно. |
| `crosshair_target` | `ClientHost` | Обычный выбор блока или сущности (Minecraft.hitResult), использующий игровой луч прицеливания, радиус действия и перекрытие. |
| `targeting` | `ClientHost` | Текущие параметры прицеливания. |
| `set_targeting` | `ClientHost` | Задаёт параметры прицеливания окна для последующих выборов. |

### `config`

| Метод | Где | Что делает |
| --- | --- | --- |
| `dist` | `Host` | Физический процесс, а не логическая сторона этого хука. |
| `game_version` | `Host` | Точная версия Minecraft, поддерживаемая этим процессом. |
| `server_data_version` | `Host` | `DataVersion` движка, записываемый в данные мира и чанки. |
| `horizon_version` | `Host` | Версия Horizon, выводимая в отчётах об ошибках. |
| `on` | `Host` | Включил ли игрок мод `mod_id` в Options > Mods. |
| `setting` | `Host` | Настройка мода `key` (по одному файлу на ключ в `native_data_dir`). |
| `set_setting` | `Host` | Сохраняет зарегистрированную настройку мода `key`. |
| `config_files` | `Host` | Файлы вызывающего мода в папке `config/` игры, рядом с `options.txt` (рядом с `server.properties` на выделенном сервере): его папка `config/<mod id>/` и файлы рядом с ней с именами `<mod id>.<ext>` или `<mod id>-<rest>`, так как моды Fabric и NeoForge хранят свои. |
| `data_files` | `Host` | `horizon-mod-data/<mod id>/` в папке игры, не сканируется как настройки или конфиг. |
| `dimension` | `Host` | Текущий мир (`minecraft:overworld`) для хука, которому не указали, где он выполняется. |
| `server_has_time` | `Host` | Есть ли у текущего тика сервера время до начала следующего (`MinecraftServer.haveTime`, `ServerTickEvent.hasTime` и `LevelTickEvent.hasTime` NeoForge): мод, выполняющий необязательную работу в `server_tick` или `level_tick_pre`/`level_tick_post`, пропускает её, если это false. |
| `instance_dir` | `Host` | Папка экземпляра (игры), общая для его игроков: в ней находятся `saves/`, `resourcepacks/`, `shaderpacks/` и `horizon-mods/`, а экраны главного меню создают миры и читают паки там. |
| `game_tests_enabled` | `ServerHost` | Опция при запуске для тестового контента. |
| `tick_stats` | `ServerHost` | Частота тиков сервера (20, если `/tick rate` не изменила её) и среднее время тиков в миллисекундах за последнее время, как сообщает `/tick query`. |
| `tick_times` | `ServerHost` | Длительность последних 100 тиков сервера в наносекундах, как их укладывает `MinecraftServer.getTickTimesNanos`: тик с номером `n` находится по индексу `n % 100`, 0 там, где ещё ничего не записано. |
| `mob_cost_sampling` | `ServerHost` | Замеряет собственный тик каждого моба (включая всадников внутри него) с этого момента или перестаёт, суммируя по мирам и типам мобов для `take_mob_costs`, как профайлер при выборке: когда включено, два чтения часов и обновление map на моба; когда выключено, одно чтение thread-local. |
| `take_mob_costs` | `ServerHost` | Затраты тиков мобов, замеренные после прошлого вызова (`mob_cost_sampling`), от самых дорогих; очищает их. |
| `count_packets` | `ServerHost` | Начинает или прекращает считать play-пакеты каждого игрока по id и направлению с этого момента для `player_take_packet_counts`. |
| `server_view_distance` | `ServerHost` | Дальность прорисовки сервера в чанках (`PlayerList.getViewDistance`, `view-distance`): насколько далеко игрокам максимально отправляются чанки. |
| `server_simulation_distance` | `ServerHost` | Дальность симуляции сервера в чанках (`PlayerList.getSimulationDistance`, `simulation-distance`): насколько далеко вокруг каждого игрока тикают чанки. |
| `server_set_view_distance` | `ServerHost` | Меняет дальность прорисовки сервера на лету, как делают `PlayerList.setViewDistance` (и `serversettings/view_distance/set` управляющего протокола): каждому игроку отправляется `ClientboundSetChunkCacheRadiusPacket`, а за ним и чанки, каждый в пределах запрошенного его клиентом. |
| `server_set_simulation_distance` | `ServerHost` | Меняет дальность симуляции сервера на лету, как делает `PlayerList.setSimulationDistance`: каждому игроку отправляется `ClientboundSetSimulationDistancePacket`, а чанки вокруг игроков тикают на новой дистанции с следующего тика. |
| `set_tick_rate` | `ServerHost` | Меняет частоту тиков сервера, как делает `/tick rate` (`ServerTickRateManager.setTickRate`): игра тикает `rate` раз в секунду начиная со следующего тика, уже идущее ожидание автосохранения сокращается, если новая частота требует меньше, и срабатывает `server_tick_rate_changed`. |
| `world_dir` | `ServerHost` | Папка открытого мира (`world/`, рядом с `level.dat`): серверный мод хранит свои данные в отдельной папке там, чтобы резервные копии мира включали их. |
| `world_data_files` | `ServerHost` | `horizon-mod-data/<mod id>/`, копируется, резервируется и удаляется вместе с этим сохранением. |
| `game_rules` | `ServerHost` | Все текущие правила в едином общесерверном хранилище, по id с пространством имён. |
| `game_rule` | `ServerHost` | Читает общесерверное правило начиная с первого серверного хука. |
| `reload_resources` | `ServerHost` | Сохранённые серверные слушатели последней успешной перезагрузки. |
| `translate` | `ClientHost` | Текст выбранного языка для ключа перевода (поиск `Component.translatable`), `None`, если ни один языковой слой его не определяет. |
| `setting_bool` | `ClientHost` | Переключатель мода-вызывателя `key` (`Registrar::register_setting`): выбор игрока или, если нет, его значение по умолчанию. |
| `setting_choice` | `ClientHost` | Индекс опции, которую выбрал игрок для настройки выбора мода-вызывателя `key`, или индекс по умолчанию. |
| `setting_f64` | `ClientHost` | Диапазон мода-вызывателя `key`: значение игрока или, если нет, по умолчанию; сохранённое значение вне диапазона читается как по умолчанию. |
| `setting_color` | `ClientHost` | Цвет мода-вызывателя `key`, `0xRRGGBB`: выбор игрока или, если нет, по умолчанию. |
| `official_extra_settings` | `ClientHost` | Старые настройки мода-вызывателя из `horizon-extras.txt`, если это официальный мод, перенявший встроенное расширение (`fps_overlay`, `keystrokes`, `minimap`, `zoom`): каждый ключ расширения, для которого в файле есть строка, со значением, которое расширение использовало и записало (`("fpsCorner", "top_left")`). |

### `displays`

| Метод | Где | Что делает |
| --- | --- | --- |
| `display` | `ServerHost` | Сужает сущность до display, либо None для другого типа, устаревшего handle или клиента. |

### `draw`

| Метод | Где | Что делает |
| --- | --- | --- |
| `fluid_name` | `ClientHost` | Название жидкости `fluid` так, как она названа в языке (`fluid_type.<mod>.<name>`, у ванильных по её блоку), иначе её path словами. |
| `bake_layer` | `ClientHost` | Слой модели сущности `layer`, запечённый в её дерево `ModelPart`, каждая часть в своей позе, как запекает `EntityModelSet.bakeLayer`: зарегистрированный нативным модом (`register_layer_definitions`) или ванильный, по имени, которое задаёт ему `entity-models.json` (`minecraft:zombie#main`, `minecraft:player#main`). |
| `size` | `Host` | Размер GUI в пикселях GUI, сначала ширина, затем высота. |
| `item_model` | `Host` | Иконка предмета `item`, рисуемая моделью `model` (`minecraft:paper` как `mymod:wheel_icon`, `item_model` контент-мода), как рисует `native_item`; false для неизвестного предмета. |
| `quad` | `Host` | Четырёхугольник с углами `corners` в сплошном цвете `argb`. |
| `fill` | `Host` | Прямоугольник в пикселях GUI, `argb` (0x78000000 — цвет рамки расширений). |
| `fill_gradient` | `Host` | `GuiGraphics.fillGradient`: прямоугольник `rect` (`[x, y, w, h]` в пикселях GUI, как принимает `fill`(Self::fill)) с градиентом от ARGB `top` в `y` до `bottom` в `y + h`. |
| `sprite` | `Host` | Четырёхугольник с углами `corners`, отображающий спрайт `name` из 2D-атласа игры (`block/fire_1`, `misc/underwater`), где `uv` — текстурные координаты каждого угла в единицах спрайта (от 0 до 1 — спрайт один раз; далее он повторяется), каждый тексель умножается на `argb`. |
| `item` | `Host` | Рисует `stack` под позой `draw` так, как предмет рисуется в мире (`ItemModelResolver.updateForTopItem` и `ItemStackRenderState.submit`), по центру в начале координат позы, в трансформации `display` его модели: `Fixed` как у костра или рамки для предмета, `Ground` как у выпавшего предмета. |
| `text` | `Host` | Рисует `text` под позой `draw`, как ванильный `Font.drawInBatch` рисует компонент в мире (строку таблички, строку текстового дисплея): в плоскости x-y позы, один юнит равен пикселю шрифта, y вниз, левый край в `at`, направление к +z. |
| `item_stack` | `Host` | Рисование стака не ограничивает количество в стеке и максимальный урон. |
| `gui_paint` | `Host` | Вызовы извлечения GUI-графики проходят через атлас этого окна, включая лица игроков. |
| `gui_nine_slice` | `Host` | Рендеринг виджетов по ванильному nine-slice. |
| `gui_tooltip` | `Host` | Ванильный тултип у курсора, в GUI-пикселях. |
| `text_width` | `Host` | The width of `text` in GUI pixels, as `native_text` draws it on `draw` (`Font.width`). |
| `fluid` | `Host` | Жидкость `fluid` (`minecraft:water`, `mymod:oil`) рисуется как квадрат с верхним левым углом в `at`, шириной `size` GUI-пикселей (16 в слоте): спрайт, названный её видом (`data/<mod>/horizon/fluid/<name>.json`), тонированный её цветом, иначе плоский квадрат цвета, как у ванильных воды и лавы. |
| `draw_scene` | `Host` | Рисует `blocks` — сцену из block states в собственных координатах (не мира) — в рамку `rect` (x0, y0, x1, y1, GUI-пикселей) `batch` экрана, как задано `view`: сцены Ponder в Create. |
| `push_clip` | `Host` | `GuiGraphics.enableScissor`: всё, что рисует этот вызов хука с этого момента до парного `pop_clip`(Self::pop_clip), обрезается по `rect` (`[x, y, w, h]` в GUI-пикселях, как принимает `fill`(Self::fill)); сохраняется видимая часть спрайта, глифа или предмета. |
| `pop_clip` | `Host` | `GuiGraphics.disableScissor`: закрывает последнюю открытую область отсечения `push_clip`(Self::push_clip), так что следующее рисуемое обрезается только окружающими областями отсечения. |
| `entity` | `Host` | Рисует живую сущность `entity`, стоящую в `view.rect`, при `view.size` GUI-пикселей на блок и повёрнутую как задано `view.look`: как ванильные `InventoryScreen.renderEntityInInventoryFollowsMouse` (`GuiEntityLook::At` у курсора) и `renderEntityInInventory` (`GuiEntityLook::Turned`), т. е. так, как её показывают страница моба, иконка сущности или моб из data model. |
| `draw_menu_background` | `Host` | `Screen.extractMenuBackground`: рисует мозаичный фон меню (32x32) по всей `rect` (`[x, y, w, h]` в GUI-пикселях). |
| `tiled_sprite` | `Host` | Повторяет спрайт `sprite` по всей области `rect` (`[x, y, w, h]` в пикселях GUI), каждый тайл — квадрат со стороной `size` пикселей GUI, с тонировкой `argb`. |
| `gui_draw_item_with` | `Host` | Рисует стак предмета с пользовательским текстом количества/украшения (или None) в `at` в GUI-единицах. |
| `widget_button` | `Host` | Кнопка так, как её рисуют игровые экраны (`AbstractButton`): `label` по центру спрайта `widget/button` (`button_highlighted`, когда `highlighted`; `button_disabled` и серый, когда не `active`), прокрутка при отсутствии места и учёт для диктора и фокуса контроллера. |
| `widget_text_field` | `Host` | Текстовое поле так, как его рисуют игровые экраны (`EditBox`): текст от первого показываемого `field.scroll` символа, прокрутка, чтобы курсор оставался видимым, подсвеченное выделение, каретка, подсказка, спрайт `widget/text_field`, если есть рамка, и учёт для диктора и фокуса контроллера. |
| `push` | `Host` | `PoseStack.pushPose`: `pop` вернётся к текущей позе. |
| `pop` | `Host` | `PoseStack.popPose`; собственная поза блока никогда не выгружается со стека. |
| `translate` | `Host` | Смещает следующее рисуемое на `by`, в блоках. |
| `rotate_about` | `Host` | Поворачивает следующее рисуемое на `degrees` вокруг `axis` (против часовой стрелки, если смотреть с положительного конца оси, как в `Axis.rotationDegrees`) относительно `pivot`, в блоках от начала позы: `[0.5; 3]` — центр блока. |
| `scale` | `Host` | Масштабирует следующее рисуемое в `by` раз относительно начала позы. |
| `keep` | `Host` | Сохраняет то, что нарисовал этот вызов, не вызывая `block_entity_submit` и `block_entity_render_bounding_box`, пока не изменятся состояние, блок-сущность, свет или ресурспаки. |
| `partial` | `Host` | Частичный тик кадра, от 0 до 1 (`BlockEntityRenderState`). |
| `game_time` | `Host` | Игровое время мира, для анимаций. |
| `animation_time` | `Host` | Такт кадра на таймере анимации этого клиента, в тиках с частичным тиком: им поворачиваются части, использующие `spin`. |
| `light` | `Host` | Свет, которым рисуют вспомогательные функции: собственный для блока (`LevelRenderer.getLightCoords`), 16-кратный свет неба в битах 0..8 и 16-кратный свет блока в битах 8..16. |
| `set_light` | `Host` | Освещает следующее рисуемое `light` (как его читает `native_draw_light`). |
| `quad` | `Host` | Четырёхугольник с углами `corners` в сплошном цвете `argb`. |
| `cube` | `Host` | Рисует куб от `from` до `to` (тексели 16 на блок, как в элементе модели блока) под позой `draw`, с указанными гранями по `Direction` (down, up, north, south, west, east), запечёнными как у элемента модели (`FaceBakery`) и умноженными на 0xRRGGBB `tint`. |
| `block` | `Host` | Рисует модель блока `block` (`mymod:cog`, любой блок с моделью) в состоянии с `properties` (`[("axis", "x")]`, остальное по умолчанию) под позой `draw`, как ванильный `submitBlockModel` рисует блок на месте сущности: запечённая модель блока с его оттенками по умолчанию. |
| `item` | `Host` | Рисует `stack` под позой `draw` так, как предмет рисуется в мире (`ItemModelResolver.updateForTopItem` и `ItemStackRenderState.submit`), по центру в начале координат позы, в трансформации `display` его модели: `Fixed` как у костра или рамки для предмета, `Ground` как у выпавшего предмета. |
| `spin` | `Host` | Следующее рисуемое вращается само: на `degrees_per_tick` градусов каждый тик таймера анимации (`animation_time`; вал с `rpm` поворачивается на `rpm * 0.3`) вокруг `axis` относительно `pivot`, оба значения в блоках от угла блока независимо от позы, против часовой стрелки, если смотреть с положительного конца оси. |
| `still` | `Host` | Следующее рисуемое снова стоит на месте, после `spin`. |
| `model` | `Host` | Рисует разрешённую самостоятельную модель под позой `draw`: без поворота, от нижнего северо-западного угла блока, освещённую `draw.light`. |
| `terrain_model` | `Host` | Рисует разрешённую самостоятельную модель под позой `draw`, размещая её как `Self::model`, с материалами ландшафта: спрайты блоков (анимированные, mipmap по настройкам), задние грани отсечены, цвет и глубина пишутся как непрозрачный ландшафт, а свет `draw.light` увеличен собственной эмиссией каждого квада. |
| `text` | `Host` | Рисует `text` под позой `draw`, как ванильный `Font.drawInBatch` рисует компонент в мире (строку таблички, строку текстового дисплея): в плоскости x-y позы, один юнит равен пикселю шрифта, y вниз, левый край в `at`, направление к +z. |
| `text_width` | `Host` | The width of `text` in font pixels, as `text` draws it, styles included (`Font.width`), to lay a line out before drawing it. |
| `model_part` | `Host` | Рисует часть модели `part` и части под ней под позой `draw`, как это делает ванильный `ModelPart.render`: каждая часть смещается на своё место (в текселях, 16 на блок), поворачивается вокруг z, затем y, затем x и масштабируется, её грани текстурируются спрайтом `sprite` (`mymod:entity/golem`, `assets/mymod/textures/entity/golem.png` контентного мода или ванильным `entity/zombie/zombie`) и умножаются на 0xRRGGBB `tint`. |
| `fluid_box` | `Host` | Рисует `stack` как неподвижную жидкость, заполняющую `bounds` (`[x0, y0, z0, x1, y1, z1]` в блоках под позой `draw`), как содержимое бака: `minecraft:water` или `minecraft:lava` без компонентов. |
| `quad` | `Host` | Четырёхугольник с углами `corners` в сплошном цвете `argb`. |
| `sprite` | `Host` | Четырёхугольник с углами `corners`, отображающий спрайт `name` из 2D-атласа игры (`block/fire_1`, `misc/underwater`), где `uv` — текстурные координаты каждого угла в единицах спрайта (от 0 до 1 — спрайт один раз; далее он повторяется), каждый тексель умножается на `argb`. |
| `draw_instance_group` | `Host` | Рисует retained-группу один раз на каждый вид, с отсечением по виду и правилами маршрутизации batch. |
| `lines` | `Host` | Атомарно отправляет неосвещённые ARGB-линии в мире. |
| `quads` | `Host` | Атомарно отправляет неосвещённые квады в мире с альфа-смешиванием, используя белый тексель атласа. |
| `pipeline_quads` | `Host` | Атомарно отправляет квады мира, рисуемые пайплайном мода `pipeline`: шейдер получает текстурные координаты и цвет каждого квада, а режимы смешивания и глубины применяются. |
| `usage` | `Host` | Количество в очереди и фиксированные ёмкости, общие для обоих режимов глубины и всех слушателей. |
| `draw_model_instances` | `Host` | Отрисовывает каждую копию из `instances` разрешённой модели `model`. |

### `effects`

| Метод | Где | Что делает |
| --- | --- | --- |
| `muffle_sounds` | `ClientHost` | Приглушает игровые звуки на время, как при звоне в ушах после близкого взрыва: `amount` (от 0 до 1) убирает часть громкости каждого звука, а `lowpass` (от 0 до 1) глушит его, от неискажённого звука до фильтра нижних частот 300 Гц. |
| `flame_particle` | `ClientHost` | Малая локальная ванильная частица; вызывающему коду следует ограничивать частоту эмиссии. |
| `play_local_sound` | `ClientHost` | Проигрывает `sound` только этому игроку (событие `SoundPlay`: `minecraft:ui.button.click` или собственный звук мода из его `assets/<id>/sounds.json`): ничего не отправляется на сервер, а другие игроки не слышат звук. |
| `particle_count` | `ClientHost` | Количество живых частиц в клиентском мире (`ParticleEngine.countParticles`): 0 после запроса очистки, а также на сервере. |
| `clear_particles` | `ClientHost` | `ParticleEngine.clearParticles`: каждая частица и эмиттер удаляются перед отрисовкой следующего кадра или перед следующим тиком частиц — что наступит раньше. |
| `add_particle` | `ClientHost` | ClientLevel.addParticle: добавляет одну локальную частицу через обычный провайдер с учётом ограничений по расстоянию до камеры, настройкам и количеству. |
| `create_fireworks` | `ClientHost` | `ClientLevel.createFireworks`: отрисовывает взрывы фейерверка в мире этого клиента так, как их отрисовывает разрывающаяся ракета (`FireworkParticles.Starter`): форма каждого взрыва, цвета, цвета перехода, след и мерцание, по одному каждые два тика. |
| `sound_events` | `ClientHost` | Все звуковые события, доступные ресурсам клиента (`SoundManager.getAvailableSounds`). |
| `audio_devices` | `ClientHost` | Выходные устройства, на которых может играть звук (`SoundEngine.getAvailableSoundDevices`). |
| `audio_device` | `ClientHost` | Выходное устройство, указанное в настройках (`Options.soundDevice`). |

### `entities`

| Метод | Где | Что делает |
| --- | --- | --- |
| `entity` | `Host` | Сущность с идентификатором `id` (идентификатор, который несут события) как дескриптор для методов `entity_*`: None, если мир не открыт, или если нет такой сущности в текущем мире или среди игроков сервера. |
| `entity_residency` | `Host` | Где сейчас находится сущность `entity`, по состоянию её части (`EntityPartState`): в мире этого хоста, вне списков её мира на её тике, в другом мире или отсутствует (удалена или ожидает удаления, выгружена, перемещена под новым идентификатором, либо это дескриптор другого открытого мира). |
| `level` | `Host` | Мир `dimension` (`minecraft:the_nether`) как дескриптор: на клиенте — показываемый им мир, как бы сервер его ни называл, и каждый мир, открытый сервером для просмотра (`horizon:levels`), пока этот просмотр открыт. |
| `entity_by_uuid` | `ServerHost` | Находит загруженную сущность по UUID в текущем мире сервера для этого хука. |
| `summon` | `ServerHost` | Призывает сущность типа `kind` (`minecraft:item_display`) в точке `pos` в текущем мире, `dimension`, из `nbt` (её NBT-компаунд, как принимает `/summon <kind> <pos> <nbt>`; включая пассажиров): её идентификатор. |
| `set_experience_orb` | `ServerHost` | Записывает `value` сферы опыта `id` (очки опыта, выдаваемые при подборе), `count` (объединённые в неё сферы, каждая стоимостью `value`) и `age` (прожитые тики; сфера исчезает на 6000); `None` оставляется без изменений. |
| `experience_orbs` | `ServerHost` | Сферы опыта в мире сервера `dimension`, бокс которых пересекает бокс `min`..`max`, в порядке мира: то, что мод читает перед отменой сферы в `entity_join_level`, чтобы выдать её очки опыта через `PlayerRef::give_experience_points`. |
| `spawn_falling_block` | `ServerHost` | Запускает падающий блок с состоянием `state` из `pos` (его нижний центр) со скоростью `velocity` (блоков за тик) в текущем мире, `dimension`: он падает, приземляется и ставит себя, как песок, или выпадает как предмет, если не может. |

### `fluids`

| Метод | Где | Что делает |
| --- | --- | --- |
| `cauldron_content_for_block` | `Host` | Содержимое, зарегистрированное для `block`, или ничего, если его нет или таблица не построена. |
| `cauldron_content_for_fluid` | `Host` | Содержимое, зарегистрированное для `fluid`, или ничего, если его нет или таблица не построена. |
| `cauldron_level` | `Host` | Текущий уровень `state`, или ничего, если нет зарегистрированного содержимого или построенной таблицы. |

### `item_entities`

| Метод | Где | Что делает |
| --- | --- | --- |
| `item_entity` | `Host` | Приводит сущность к выпавшему предмету или None, если это другой тип, устаревший дескриптор, клиент или предмет, временно извлечённый для тика. |

### `item_particles`

| Метод | Где | Что делает |
| --- | --- | --- |
| `item_particle_options` | `ClientHost` | Разрешает ванильный `ItemParticleOption` для этого стака, используя контекст отображения на земле и затравку 0, в точности как частицы еды в игре. |

### `items`

| Метод | Где | Что делает |
| --- | --- | --- |
| `max_stack_size` | `Host` | Максимальное количество предмета `item`, которое вмещает один слот (`Item.getDefaultMaxStackSize`), или `None` для неизвестного предмета: для контейнера мода, который сам заполняет или объединяет стаки. |
| `max_damage` | `Host` | Базовая прочность разрешённого предмета, или ничего, если его метаданные недоступны. |
| `item_key` | `Host` | Однократно разрешает предмет при загрузке, включая ванильные предметы, до его идентичности, локальной для процесса. |
| `item_default_component` | `Host` | Значение по умолчанию компонента `component` предмета `item` в его прототипе (`Item.components().get`): `minecraft:max_damage` у железного меча равно `250`, с учётом изменений датапаков и нативных модов. |
| `item_burn_time` | `Host` | Время горения `item` как топлива для готовки в тиках (`FuelValues.burnDuration` из его компонента `cooking_fuel`, разрешённый контекстный int-провайдер датапака), None, если он не является топливом. |
| `item_compost_chance` | `Host` | Шанс предмета `item` поднять уровень компостера (разрешённый поставщик слоёв из его компонента `compostable`), None, если он не компостируется. |
| `enchantments_compatible` | `Host` | Могут ли зачарования `a` и `b` находиться на одном предмете (`Enchantment.areCompatible`) согласно определениям зачарований мира. |
| `entity_type_in_tag` | `Host` | `EntityType.is(TagKey)`: входит ли тип сущности `entity_type` (`minecraft:player`) в тег типов сущностей `tag` (`minecraft:cannot_be_pushed_onto_boats`). |
| `item_from_snbt` | `Host` | Стак, который представляет SNBT-компаунд предмета, читаемый так, как `ItemStack.CODEC` читает сохранённый предмет: `{id:"minecraft:diamond_sword",count:1,components:{"minecraft:damage":5}}` (`count` равен 1, если отсутствует, а `stone` — это `minecraft:stone`). |
| `dispense_default` | `ServerHost` | Выбрасывает один предмет так, как это делает DefaultDispenseItemBehavior. |
| `scratch_stack` | `ServerHost` | Удерживает `stack` вне мира в открытой транзакции `tx` (`ItemAccess.forStack`): `ItemPlace::Scratch` для работы с возможностями предмета. |
| `scratch_take` | `ServerHost` | Забирает стак из черновой позиции `place` в `tx` и завершает работу с позицией: ничего, если она пуста, больше не удерживается или не является черновой позицией. |
| `scratch_slots` | `ServerHost` | Удерживает `stack` вне мира в двухслотовой копии, чей предмет может измениться, в открытой транзакции `tx`: `ItemPlace::ScratchSlots` — позиция, в которой работает выдаваемый контейнер жидкости (`DispenseFluidContainer`), с `stack` в его первом слоте. |
| `scratch_take_slots` | `ServerHost` | Забирает оба слота из двухслотовой позиции `place` в `tx` и завершает работу с позицией: то, что держит первый слот, и то, что поймал второй. |
| `dispense_with_remainder` | `ServerHost` | `DefaultDispenseItemBehavior.consumeWithRemainder`: используется один предмет из `stack`, а `remainder` заменяет его; для одиночного `stack` — становясь им, иначе — помещается в раздатчик в `pos` (`DispenserBlockEntity.insertItem`) или, если не помещается, выбрасывается из его передней стороны с щелчком и дымом. |
| `give` | `ClientHost` | Только в творческом режиме: выдаёт игроку `count` предметов `item` (`minecraft:diamond`) так, как при взятии из творческого инвентаря (`ServerboundSetCreativeModeSlot`): сначала в выбранный слот, затем в хотбар, затем в инвентарь. |
| `hover_name` | `ClientHost` | `ItemStack.getHoverName()` как обычный текст: пользовательское название, иначе непустое название написанной книги, иначе название предмета на языке клиента (содержимое зелья, владелец головы и компонент `item_name`, как в ваниле). |
| `enchantments` | `ClientHost` | Зарегистрированные зачарования в порядке реестра. |
| `enchantment_min_cost` | `ClientHost` | `Enchantment.getMinCost(level)`: необходимая сила зачарования, или None, если id неизвестен. |
| `enchantment_supports` | `ClientHost` | Можно ли наложить зачарование `id` на `item` (`Enchantment.isSupportedItem`). |

### `keys`

| Метод | Где | Что делает |
| --- | --- | --- |
| `biome_key` | `Host` | до установки данных мира, или для неизвестного id. |
| `game_event_key` | `Host` |  |
| `block_key` | `Host` | Ключ блока с именем `id`, когда имя приходит только во время выполнения (данные, конфиг, команда, сетевое сообщение). |
| `effect_key` | `Host` | Ключ эффекта с именем `id`: полное имя в реестре, `minecraft:speed` или `mymod:rage`. |
| `registry_entry` | `Host` | Получает каноническую запись в активном реестре текущей стороны. |
| `registry_entries` | `Host` | Идентификаторы записей в текущей таблице, каждый один раз. |
| `registry_entry_in_tag` | `Host` | Принадлежность к объединённым тегам текущей стороны. |
| `registry_entry_key_slot` | `Host` | Получает слот активного типизированного ключа, принадлежащий загрузке, и никогда — его числовой id протокола. |
| `block_keys` | `Host` | Все ключи блоков в порядке регистрации реестра: сначала ванильные, затем модовые блоки в том порядке, в котором они были зарегистрированы. |
| `item_keys` | `Host` | Все ключи предметов в порядке регистрации реестра: сначала ванильные, затем модовые предметы в том порядке, в котором они были зарегистрированы. |
| `entity_type_keys` | `Host` | Все встроенные ключи типов сущностей в порядке реестра (`BuiltInRegistries.ENTITY_TYPE`): сначала ванильные, затем модовые типы в том порядке, в котором они были зарегистрированы. |
| `menu_keys` | `Host` | Все встроенные ключи типов меню в порядке реестра (`BuiltInRegistries.MENU`): сначала ванильные, затем модовые типы в том порядке, в котором они были зарегистрированы. |
| `enchantment_keys` | `Host` | Все ключи зачарований в глобальном отсортированном порядке загруженного реестра, включая записи ванили и датапаков модов вместе (`RegistryDataLoader` сортирует по id). |
| `registry_protocol_id` | `Host` | Сетевой id `name` в `registry` (`Registry.getId`): число, которое пакет передаёт для него, для мода, который собирает или читает сырые пакеты (`minecraft:particle_type` и `minecraft:flame`). |

### `levels`

| Метод | Где | Что делает |
| --- | --- | --- |
| `is_loaded` | `Host` | Установлен ли мод с id `id` из `horizon-mod.toml`: нативный, wasm или пакет контента (в Fabric — `FabricLoader.isModLoaded`, в NeoForge — `ModList.isLoaded`). |
| `chunk` | `Host` | Находит загруженный чанк в открытом серверном мире. |

### `living`

| Метод | Где | Что делает |
| --- | --- | --- |
| `living` | `Host` | Сущность как живая сущность (`LivingEntity` в ваниле: моб, игрок, стойка для брони или дракон), или None для другого типа либо устаревшего дескриптора. |

### `math`

| Метод | Где | Что делает |
| --- | --- | --- |
| `math_profile` | `Host` | Фиксированный выбор нативного движка для семи платформозависимых операций Java Math. |

### `menus`

| Метод | Где | Что делает |
| --- | --- | --- |
| `open_menu` | `ServerHost` | Открывает для игрока меню сундука из `rows` строк (от 1 до 6) с названием `title`, показывая `items` (по слотам, дополняя пустыми), вместо ранее открытого меню: id окна, каждый раз новый (`ServerPlayer.openMenu`). |
| `menu` | `ServerHost` | Меню, открытое `open_menu`, которое игрок держит открытым как окно `window` (id окна, возвращаемый `open_menu`, `menu_clicked` и `menu_closed`). |
| `close_menu` | `ServerHost` | Закрывает нативное меню игрока (следом приходит `menu_closed`). |
| `menu_set_items` | `ServerHost` | Заменяет всё содержимое открытого меню `open_menu` с окном `window` на `items` (по слотам, дополняя пустыми, лишние отбрасывая, как их принимает `open_menu`) и отправляет сразу одним пакетом `ClientboundContainerSetContentPacket` (`AbstractContainerMenu.sendAllDataToRemote`) без повторного открытия меню: название, курсор и экран сохраняются. |
| `local_player` | `ClientHost` | Локальный игрок в клиентском мире после входа в игру. |

### `merchants`

| Метод | Где | Что делает |
| --- | --- | --- |
| `merchant` | `ServerHost` | Сужает сущность до жителя или странствующего торговца, или None для другого типа, устаревшего дескриптора либо на клиенте. |

### `minecarts`

| Метод | Где | Что делает |
| --- | --- | --- |
| `minecart` | `ServerHost` | Сужает сущность до вагонетки любого вида; None для другого типа сущности, устаревшего дескриптора, на клиенте или для сущности, удерживаемой вне хранилища. |

### `mobs`

| Метод | Где | Что делает |
| --- | --- | --- |
| `mob` | `Host` | Сущность как ванильная Mob: моб или эндер-дракон. |
| `kill_mob` | `ServerHost` | Убивает моба `id`, как `/kill`: он падает и выбрасывает добычу. |
| `dragon` | `ServerHost` | Сущность как эндер-дракон; None для другого типа, устаревшего дескриптора либо на клиенте. |

### `mod_enchantments`

| Метод | Где | Что делает |
| --- | --- | --- |
| `enchantment_level_for` | `ServerHost` | и указывает имя `mod_slot` вызывающей стороны, чтобы мод мог задать уровень для каждого носителя. |

### `mod_state`

| Метод | Где | Что делает |
| --- | --- | --- |
| `attachment_query` | `Host` | Захватывает снимок (SNAPSHOT) присутствия и значений вложений текущего измерения. |
| `attachments` | `Host` | Вложения носителей, принадлежащих серверу; `TYPE.get(host, holder)` читает одно. |
| `attachment_validate_write` | `ServerHost` | Проверяет ранее зафиксированное присутствие относительно текущего измерения непосредственно перед обычной журнальной записью вложения. |
| `transaction_open` | `ServerHost` | Открывает транзакцию, вложенную в открытую, если та есть (`Transaction.open`), и возвращает печать, завершающую её. |
| `transaction_close` | `ServerHost` | Закрывает транзакцию, завершаемую `seal`, с фиксацией или отменой, сначала отменяя любые открытые внутри неё и оставшиеся незакрытыми. |
| `transaction_lifecycle` | `ServerHost` | Состояние системы транзакций (`Transaction.getLifecycle`): нет открытых, одна открыта, вложенная закрывается или самая внешняя закрывается, пока игра сообщает миру о фиксации. |
| `transaction_current` | `ServerHost` | Самая внутренняя открытая транзакция для записи (`Transaction.getCurrentOpenedTransaction`); отсутствует, если нет открытых. |
| `transaction_depth` | `ServerHost` | Насколько глубоко вложен `tx` (`TransactionContext.depth`): 0 для самой внешней, отсутствует, если транзакция не открыта. |
| `schedule_task` | `ServerHost` | Запрашивает вызов `Hooks::scheduled_task` с `token` через `delay_ticks` серверных тиков (`MinecraftServer.tell(new TickTask(getTickCount() + delay, ..))`; 0 и 1 оба означают следующий тик, а остановленный сервер не продвигает его). |
| `cancel_task` | `ServerHost` | Отменяет задачу, запланированную вызвавшим модом: true, если она всё ещё ожидала. |

### `network`

| Метод | Где | Что делает |
| --- | --- | --- |
| `player_send_packet` | `ServerHost` | Отправляет `packet` клиенту этого игрока как есть (`ServerPlayer.connection.send`), как плагин Paper отправляет пакет через ProtocolLib или PacketEvents: команду, цель, счёт, запись таб-листа, которую сервер не хранит. |
| `open_chat` | `ClientHost` | Открывает экран чата с введённым `text`, чтобы игрок дописал и отправил его, как при нажатии T или /; ничего не отправляется. |
| `chat_line` | `ClientHost` | Добавляет `line` в интерфейс чата и историю этого клиента (ванильное `ChatComponent.addClientSystemMessage`): отображается только здесь, поэтому на сервер ничего не отправляется. |
| `send_chat` | `ClientHost` | Строка чата или команда мода (`/` в начале), отправляется так же, как `send_chat` отправляет их (ClientPacketListener.sendChat / sendCommand), на любой сервер: без истории чата и без клиентского события чата (ванильный `ClientChatEvent` приходит только из ChatScreen). |
| `send_packet` | `ClientHost` | Отправляет ванильный игровой пакет на любой сервер, включая посторонние данные плагинов. |
| `disconnect` | `ClientHost` | Покидает сервер или мир в одиночной игре (NeoForge `IClientCommonPacketListenerExtension.disconnect(Component)`), в конце этого кадра, а не внутри вызывающего хука. |

### `payload_tokens`

| Метод | Где | Что делает |
| --- | --- | --- |
| `check_payload_token` | `Host` | Является ли `token` выданным регистратором этой загрузки: замороженное объявление в его слоте содержит его id, протокол, направление и схему, а владеющий им мод включён. |

### `payloads`

| Метод | Где | Что делает |
| --- | --- | --- |
| `player_send_payload` | `Host` | Добавляет в очередь точные байты прикладного уровня для соединения этого живого игрока. |
| `send_payload_to_server` | `ClientHost` | Добавляет в очередь точные байты прикладного уровня на подключённый сервер, используя замороженное объявление для его текущего протокола. |

### `players`

| Метод | Где | Что делает |
| --- | --- | --- |
| `item_cooldown_group` | `Host` | `ItemCooldowns.getCooldownGroup(stack)`: группа, под которой записан откат стака `stack`, группа, названная его компонентом `use_cooldown` (собственным или значением по умолчанию его предмета), иначе id его предмета. |
| `player` | `Host` | Находит реального онлайн-игрока по UUID, никогда не подставного. |
| `fake_player` | `ServerHost` | Создаёт или получает игрока без подключения для текущего мира. |
| `server_max_players` | `ServerHost` | Максимальное количество игроков, которые могут быть онлайн одновременно (`PlayerList.getMaxPlayers`, `max-players`), как его показывает список серверов; вход сверх этого отклоняется. |
| `server_set_max_players` | `ServerHost` | Устанавливает лимит игроков со следующего входа и пинга списка серверов (в Paper `Server.setMaxPlayers`); уже подключённые игроки остаются, даже если их больше нового лимита. |
| `server_define_terrain_view` | `ServerHost` | Определяет или заменяет вид, принадлежащий вызывающему моду. |
| `player_set_terrain_view` | `ServerHost` | Выбирает вид вызывающего мода для этого подключённого игрока и измерения или снимает выбор через `None`. |
| `profile_by_uuid` | `ServerHost` | Профиль игрока с этим UUID, как знает его сервер, без запросов к Mojang: полный профиль онлайн-игрока (свойства, которые даёт список игроков, то есть установленные через `player_set_profile_property`), иначе имя из кэша пользователей (`usercache.json` выделенного сервера), без свойств. |
| `profile_by_name` | `ServerHost` | Профиль игрока с этим именем, в любом регистре, как знает его `profile_by_uuid`: онлайн-игрока, иначе не истёкшая запись кэша пользователей. |
| `push_player` | `ClientHost` | Для клиентского мода: толчок, добавляемый к скорости локального игрока, в блоках за тик, как отдача от взрыва (игрок окна движется самостоятельно, как в ванильном клиенте). |
| `select_hotbar_slot` | `ClientHost` | Для клиентского мода: выбирает слот хотбара локального игрока, `slot` (от 0 до 8), как `Inventory.setSelectedSlot` для числовой клавиши. |
| `set_collision_exclusions` | `ClientHost` | Для клиентского мода: исключения столкновений локального игрока, клиентская часть `EntityRef::set_collision_exclusions`, с теми же областями, правилами и отказами (клиент без собственного игрока отвечает `Stale`). |
| `cross_passage` | `ClientHost` | Перемещает игрока окна через `passage` (arismoko/horizon#3775), о котором ему сообщил его сервер, как это делает клиент Immersive Portals: его ноги (там, где они были тик назад, чтобы кадры между двумя тиками не дёргались), скорость и направление взгляда оказываются на другой стороне, повёрнутыми вместе с passage, а серверу сообщаются passage и то, где были ноги. |
| `last_passage_refusal` | `ClientHost` | Отказ сервера от последнего перехода окна (`Self::cross_passage`) и passage, названный им, до следующего перехода; None, если отказа не было или ответ ещё не пришёл. |

### `projectiles`

| Метод | Где | Что делает |
| --- | --- | --- |
| `spawn_projectile_from_rotation` | `ServerHost` | снаряд: снежок, яйцо, жемчуг, зелье, флакон опыта, ветряной заряд или зарегистрированный модом тип метаемого объекта. |
| `projectile` | `ServerHost` | Сужает сущность до снаряда: сущность из списка снарядов, выпущенный игроком трезубец или ракета фейерверка. |
| `fishing_hook` | `ServerHost` | Сужает сущность до заброшенного рыболовного крючка. |

### `recipe_properties`

| Метод | Где | Что делает |
| --- | --- | --- |
| `mod_recipe_property_contains` | `Host` | Принимает ли созданный модом набор свойств рецептов id предмета. |

### `recipes`

| Метод | Где | Что делает |
| --- | --- | --- |
| `recipes_of_type` | `Host` | Публикуемое чтение ресурспака/датапака: доступно на этапах Setup, Reloading и World. |
| `recipe_ingredient` | `Host` | Публикуемое чтение ресурспака/датапака: доступно на этапах Setup, Reloading и World. |
| `recipe_result` | `Host` | Публикуемое чтение ресурспака/датапака: доступно на этапах Setup, Reloading и World. |
| `known_recipes` | `Host` | `book_id` рецептов, которые сейчас есть в книге рецептов игрока (на сервере — в книге сервера); пусто вне мира. |
| `recipe_book_category` | `Host` | Публикуемое чтение ресурспака/датапака: доступно на этапах Setup, Reloading и World. |
| `panel_items` | `ClientHost` | Показывает панель рядом с экранами контейнеров с этими предметами (не более 8192), как собственные слоты игры, разложенные по страницам, с полем поиска; клики игрока, поисковые запросы и нажатия клавиш над предметами возвращаются как `panel_event`. |
| `panel_style` | `ClientHost` | Как теперь выглядит панель предметов мода `mod_id` (`panel_items`): `PanelStyle::RecipeBook` (по умолчанию: стрелки страниц книги рецептов и обычное поле поиска), `PanelStyle::Viewer` (квадратные кнопки страниц на панели над предметами и поле поиска просмотрщика, как их рисует панель Recipes в Horizon) или `PanelStyle::Classic` (список JEI в Minecraft 1.12: прижатый справа под всегда отображаемыми и переносимыми кнопками страниц, поле поиска внизу экрана с кнопкой настроек, открывающей страницу мода в разделе Настройки > Моды). |
| `place_recipe` | `ClientHost` | Размещает рецепт в сетке открытого экрана так, как это делает клик в книге рецептов (`ServerboundPlaceRecipePacket`): сетка инвентаря или верстака. |
| `transfer_recipe` | `ClientHost` | Перемещает доступные ингредиенты рецепта из просмотрщика в открытую сетку создания, без требования, чтобы рецепт был изучен. |
| `transfer_recipe_id` | `ClientHost` | Перенос рецепта по его id через «+» просмотрщика в открытое меню: модовое меню механизма, а также сетка создания. |
| `transfer_recipe_check` | `ClientHost` | Что сейчас сделает `transfer_recipe(book_id)`, вычисленное сервером без перемещения чего-либо: учитываются инвентарь игрока, сетка и все контейнеры, которые мод предоставляет сетке рецептов (сундуки станции создания, которые клиент не видит). |
| `recipe_book_ids` | `ClientHost` | Значения `book_id` для рецепта `recipe_id` (`minecraft:stick`), которые используют идентификаторы `transfer_recipe`, `transfer_recipe_check`, `place_recipe` и `known_recipes`: по одному для каждого способа отображения рецепта в книге рецептов, в порядке книги. |

### `resources`

| Метод | Где | Что делает |
| --- | --- | --- |
| `resource` | `Host` | Победившие байты ресурса `id` в стеке `pack_type` так, как на них отвечает `ResourceManager.getResource`: побеждает самый верхний выбранный ресурспак, содержащий файл. |

### `screens`

| Метод | Где | Что делает |
| --- | --- | --- |
| `preset_editor` | `ClientHost` | Редактор пресетов мира, открытый поверх «Создания мира», или отсутствует. |
| `preset_editor_dimensions` | `ClientHost` | Копия рабочих dimension stems открытого редактора пресетов. |
| `set_preset_editor_dimensions` | `ClientHost` | Замените рабочие dimension stems открытого редактора. |
| `screen_pos` | `ClientHost` | Блок открытого модового экрана (`screen`), как сервер назвал его, когда блок открыл свой тип меню; None, если модовый экран не открыт. 401. |
| `screen_extra_data` | `ClientHost` | Что мод блока записал для открытого модового экрана (`menu_extra_data` на сервере), пусто, если он ничего не записал; None, если модовый экран не открыт. |
| `screen_exclusion` | `ClientHost` | Где открытый экран рисует за пределами своего `rect`, и где моды рисуют поверх него: панель книги рецептов, ряды вкладок креативного режима, активные эффекты рядом с инвентарём, затем всё, что добавляет `screen_exclusion_areas`. |
| `screen_hovered_item` | `ClientHost` | Идентификатор предмета в слоте под указателем на открытом экране контейнера (`AbstractContainerScreen.getHoveredSlot().getItem()`), None, если указатель не над слотом или слот пуст. |
| `apply_screen_blur` | `ClientHost` | Запрашивает размытие фона меню из ванили для этого кадра с защитой по кадру, чтобы размытие применялось не более одного раза за кадр. |
| `menu_blur_radius` | `ClientHost` | Настроенный пользователем радиус размытия фона меню (`menuBackgroundBlurriness` в options.txt, обычно 0.0–10.0; по умолчанию 5.0). 0.0 означает, что размытие выключено. |
| `mod_screen_open` | `ClientHost` | Открывает собственный экран мода `mod_id` поверх мира: хуки экрана срабатывают для него так же, как для любого другого, поэтому мод рисует его в `screen_render_post` и обрабатывает в нём клики, прокрутку, клавиши и ввод; курсор освобождается, а Escape закрывает экран. |
| `mod_screen_close` | `ClientHost` | Закрывает собственный экран мода `mod_id`, если он открыт. |
| `close_screen` | `ClientHost` | Закрывает открытый экран так, как это делает его клавиша Escape: экран контейнера закрывается как закрытие контейнера игрока (`ServerboundContainerClosePacket` с идентификатором меню, затем клиентское меню закрывается), меню паузы закрывается и возвращает в игру, а другой экран выполняет своё собственное закрытие. |
| `request_client_quit` | `ClientHost` | Ставит в очередь обычное действие «Выход из игры» основного окна после текущей рассылки хуков. |
| `open_inventory` | `ClientHost` | Открывает инвентарь игрока так же, как это делает клавиша инвентаря: локально, без пакета (меню игрока всегда открыто на сервере). |
| `sign_editor` | `ClientHost` | Табличка, для которой открыт редактор табличек, её позиция блока и сторона; None, если редактор табличек не открыт или вне мира. |
| `mod_screen` | `ClientHost` | Мод, собственный экран которого открыт, если такой есть (Escape или замена другим экраном закрывает его). |
| `screen_title` | `ClientHost` | Название открытого меню (название блока или данное ему название) в виде простого текста. |
| `window_focused` | `ClientHost` | Последнее состояние фокуса окна, сообщённое оконной системой. |
| `observed_screen_id` | `ClientHost` | Тот же идентификатор экрана, который передаётся хукам экрана. |
| `is_key_down` | `ClientHost` | Удерживается ли указанная клавиша в данный момент. |
| `request_screenshot` | `ClientHost` | Делает скриншот так же, как клавиша скриншота (F2) (`Screenshot.grab`): следующий кадр считывается в размере, сохраняемом F2 (разрешение игры, умноженное на качество скриншота), хук `screenshot` выполняется по нему как для F2, поэтому слушатель скриншота каждого мода его видит, а файл записывается в папку `screenshots` игрока с ванильным именем файла и строкой чата F2. |

### `services`

| Метод | Где | Что делает |
| --- | --- | --- |
| `commands` | `ServerHost` | Сервис команд. |
| `levels` | `ServerHost` | Сервис миров. |
| `messages` | `ServerHost` | Сервис сообщений. |
| `players` | `ServerHost` | Сервис игроков. |
| `profiles` | `ServerHost` | Сервис профилей. |
| `runtime_levels` | `ServerHost` | Сервис runtime_levels. |
| `instance_groups` | `ClientHost` | Постоянные группы моделей. |
| `client_session` | `ClientHost` | Сервис client_session. |
| `render_assets` | `ClientHost` | Сервис render_assets. |
| `client_server` | `Host` | Подключённый сервер, а не устаревшая цель переподключения. |
| `client_language` | `Host` | Применённый код языка, а не ожидающий выбор в настройках. |
| `client_latency` | `Host` | Задержка локального игрока в миллисекундах так, как её показывает список игроков (`PlayerInfo.getLatency`): сглаженное среднее keep-alive на сервере, обновляемое собственной рассылкой сервера. 0 до первого измерения сервера, как в ванили; None вне подключённого клиентского мира или до того, как сервер покажет этого игрока. |
| `client_tab_list` | `Host` | Список игроков так, как его показывает оверлей Tab: отображаемое имя каждого показанного игрока, имя профиля, UUID, игровой режим и задержка в порядке отображения, максимум 80 показываемых игроков, с заголовком и подвалом. |
| `client_frame_stats` | `Host` | Существующие локальные счётчики кадров; None вне подключённого клиентского мира. |
| `refresh_commands` | `Host` | Мод изменил команды, которые он регистрирует (`register_commands`): в начале следующего серверного тика каждый мод снова опрашивается, команды, которые могут выполнять игроки, перестраиваются, а каждому игроку на сервере заново отправляется его дерево команд после `player_commands_sent`. |
| `feedback` | `Host` | Отзыв успешного выполнения команды, направленный в её исходный источник по ванильным правилам молчания, отправки отзыва и вывода блока/вагонетки. |
| `run_command` | `Host` | Выполняет `line` (без косой черты) так, как это делает консоль сервера: с уровнем разрешений 4, из точки спавна основного мира и без вывода. |
| `run_command_with_output` | `Host` | `run_command` с сохранённым обратным выводом: та же строка, источник и путь запуска (один ведущий слэш здесь отбрасывается), отвечая, была ли она выполнена без ошибок (`success`, что возвращает `run_command`), и что консоль бы вывела для неё как обычный текст (`feedback`, как нужно моду, отвечающему удалённой консоли; Paper `Server.dispatchCommand` с перехватывающим `CommandSender`). |
| `list_levels` | `Host` | Загруженные миры сервера, в ванильном порядке измерений. |
| `text_allowed` | `Host` | Пропускает ли фильтр чата сервера (`server.toml` `[chat_filter]`) `text`: true без фильтра, None в хуке игрока (где сервер недоступен). |
| `broadcast` | `Host` | Системная строка в чате каждого игрока и в логе сервера. |
| `server_players` | `Host` | Игроки на этом сервере, в порядке подключения; в одиночном режиме это включает владельца мира, который подключается к его серверу как любой игрок. |
| `player_by_name` | `Host` | Найти подключённого игрока по имени учётной записи, без учёта регистра. |
| `acting_player` | `Host` | Игрок, чьё действие или команду слушает хук, при подставлении его состояния (`run_mod_command`, `right_click_block`, `native_use_on`, `break_block`...): None вне такого хука. |
| `resolve_name` | `Host` | Разрешает имя учётной записи Minecraft (`Notch`) через резолвер профилей игры: профиль сервера сессий с подписанным свойством `textures`, используемым скин-модом для замены скина игрока. |
| `resolve_id` | `Host` | Разрешает UUID учётной записи через резолвер профилей игры, так же как `resolve_name` разрешает по имени. |
| `open` | `Host` | Открыть мир `id` (`mymod:instance/ab`) в пространстве имён вызывающего мода, как описывает `level`. |
| `unload` | `Host` | Сохранить и закрыть мир `id`, открытый вызывающим модом, сохранив его папку, когда миры этого серверного тика оттикуют: тогда срабатывает `level_unload`. |
| `delete` | `Host` | Удалить мир `id`, принадлежащий вызывающему моду, вместе с папкой, когда миры этого серверного тика оттикуют. |
| `is_open` | `Host` | Открыт ли мир `id`: false, пока не применён поставленный в очередь `open`, и true, пока не применён поставленный в очередь `unload` или `delete`. |
| `model` | `Host` | Публикуемое чтение ресурсов/данных: доступно в Setup, Reloading и World. |
| `render_pipeline` | `Host` | Публикуемое чтение ресурсов/данных: доступно в Setup, Reloading и World. |
| `animation` | `Host` | Публикуемое чтение ресурсов/данных: доступно в Setup, Reloading и World. |
| `create_group` | `Host` | Выделить до 16,384 копий, изначально нулевой длины. |

### `tags`

| Метод | Где | Что делает |
| --- | --- | --- |
| `tags` | `Host` | Служба объединённых тегов, использующая разрешённые при загрузке ключи предметов и тегов. |
| `unresolved_fluid_id` | `Host` | Сохранённое имя неразрешённой жидкости, выданной движком. |
| `block_in_tag` | `Host` | Находится ли разрешённый блок в объединённом теге блоков мира, а на клиенте — в теге, присланном его сервером. |
| `state_outline` | `Host` | Контур `state`, как `BlockState.getShape` отвечает на него с пустым getter'ом в начале координат (`EmptyBlockGetter.INSTANCE`, `BlockPos.ZERO`): его боксы, каждый `[min x, y, z, max x, y, z]` в собственном пространстве блока, без случайного смещения и без контекста мира (состояние, читающее соседей или блок-сущность, возвращает то, что даёт только его состояние). |
| `state_fluid_state` | `Host` | Жидкостное состояние, которое `state` удерживает (`BlockState.getFluidState`): вода для пропитанного водой состояния, пустая жидкость для большинства. |
| `fluid_in_tag` | `Host` | Находится ли `fluid` в теге жидкостей `tag` (`FluidState.is(TagKey)`, `minecraft:water` или `water`) по объединённым тегам жидкостей, а на клиенте — по тем, что прислал его сервер. |
| `fluid_is_same` | `Host` | Являются ли `fluid` и `other` одной жидкостью (`Fluid.isSame`): источник и его текущая форма являются, поэтому `minecraft:water` и `minecraft:flowing_water` одинаковы, а вода и лава — нет. |
| `fluid_bucket` | `Host` | Ведро, забирающее `fluid` (`Fluid.getBucket`), `minecraft:water_bucket` для воды и текущей воды. |
| `fluid_pickup_sound` | `Host` | Звуковое событие, которое ведро проигрывает при заборе `fluid` (`Fluid.getPickupSound`), `minecraft:item.bucket.fill_lava` для лавы и текущей лавы. |
| `fluid_empty_sound` | `Host` | Звуковое событие, которое ведро проигрывает при опустошении `fluid` (`BucketItem.playEmptySound`, звук `BUCKET_EMPTY` у `FluidType` NeoForge), `minecraft:item.bucket.empty_lava` для лавы и текущей лавы и `minecraft:item.bucket.empty` для воды и жидкости мира мода. |

### `text`

| Метод | Где | Что делает |
| --- | --- | --- |
| `text_from_json` | `Host` | Читает текстовый компонент так, как его принимают `/tellraw` и `ComponentSerialization.CODEC`: одно значение JSON или SNBT (`"plain"`, `{"text":"Hi","color":"gold"}`, `["a",{"text":"b"}]`, `translate`, `extra`, события нажатия и наведения), после которого ничего, кроме пробелов (Paper `GsonComponentSerializer.deserialize`). |
| `text_from_legacy` | `Host` | Читает устаревшие коды форматирования (`§a`, `§l`, `§r`...) так, как их рисует шрифт (`Style.applyLegacyFormat`; строки `ChatColor` Bukkit): код цвета сбрасывает оформление, `§r` сбрасывает всё, неизвестный код отбрасывается. |
| `text_to_nbt` | `Host` | Текст так, как его сохраняют компоненты предметов и блок-сущности (`ComponentSerialization` в NBT): содержимое `minecraft:custom_name` и каждой строки `minecraft:lore`. |

### `world_edit`

| Метод | Где | Что делает |
| --- | --- | --- |
| `edit_blocks` | `ServerHost` | Для серверного мода: наборы изменений, выполняемые за раз, сразу в чанки авторитетного мира `dimension` (который тикается, как у `native_set_block`), с поведением `fill ... strict` для блок-сущностей (блок, заменяемый другим, теряет свою блок-сущность, а новый блок получает ту, которую дал бы `native_set_block`, включая блок-сущность мода); соседям не сообщается, а изменённые чанки полностью отправляются игрокам. |
| `upgrade_block_entity_nbt` | `ServerHost` | Данные блок-сущности, сохранённые при `data_version` (схемы, файла структуры, более старого мира), обновлённые так же, как загрузчик чанков обновляет блок-сущность чанка этой версии: через datafixes её типа до DataVersion игры. |
| `upgrade_entity_nbt` | `ServerHost` | Данные сущности, сохранённые при `data_version`, обновлённые так же, как загрузчик чанков сущностей обновляет сущность чанка этой версии: через datafixes её типа и её пассажиров до DataVersion игры. |
| `surface_heights` | `ServerHost` | Для серверного мода: высота каждой колонки по `heightmap` в авторитетном мире `dimension` (который тикается), и её нет для незагруженной колонки. |
| `read_columns` | `ServerHost` | Для серверного мода: `below` блоков под и `above` блоков от высоты каждой колонки по `heightmap`, как палитра полных значений `BlockState` и индексов, с оттенками биомов (воздух вне колонки и в незагруженных колонках). |
| `terrain_surface` | `ServerHost` | Для серверного мода: что генератор создаёт для каждой из `columns` мира `dimension` (который тикается) до размещения его особенностей, загружен чанк или нет: читает генератор и не создаёт чанк. |
| `structure_template` | `ServerHost` | Для серверного мода: шаблон структуры `id` (`mymod:hut`, искомый так же, как ищет его `StructureTemplateManager.get`: сохранённые структуры мира, затем структуры игры и модов), как `/place template id` разместил бы его в `at` с `rotation` и `mirror`, не размещая ничего. |

### `world_request`

| Метод | Где | Что делает |
| --- | --- | --- |
| `request_world` | `ClientHost` | Спрашивает игрока, открыть ли `saves/<folder>` так, как это делает кнопка Play на титульном экране: запрос ставится в очередь, а следующий кадр титула показывает вопрос с именем мода и сохранения и кнопками «Да» и «Нет». |
