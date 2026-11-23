#!/usr/bin/env python3
"""The modding reference (docs/magnetite/modding-reference.md and .en.md) from the mod API's sources.

Every hook of the catalog (`crates/horizon-mod-vocab/src/hooks/*.rs`) and every public Host method
(`crates/horizon-mod-vocab/src/host/*.rs`), with its side, contract and the first sentence of its
doc. The descriptions are translated once and kept in `docs/magnetite/modding-reference.ru.json`
(name -> Russian line); a name with no translation shows its English line, so the page is always
complete and only new entries need translating.

    python tools/modding_reference.py extract > entries.json   # what the sources declare
    python tools/modding_reference.py render                   # writes both pages
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HOOKS = ROOT / "crates/horizon-mod-vocab/src/hooks"
HOST = ROOT / "crates/horizon-mod-vocab/src/host"
DOCS = ROOT / "docs/magnetite"
RU = DOCS / "modding-reference.ru.json"


def first_sentence(lines):
    """The doc's first sentence, one line, without the `Side: ...` marker the host docs start with."""
    text = " ".join(l.strip() for l in lines if not l.strip().startswith("Side:")).strip()
    # The end of the first sentence: a full stop before a space and a capital, a backtick or the end.
    m = re.search(r"^(.+?[.!?])(\s+[A-Z(`\[]|\s*$)", text)
    return (m.group(1) if m else text).strip()


def hooks():
    out = []
    for path in sorted(HOOKS.glob("*.rs")):
        if path.name == "mod.rs":
            continue
        lines = path.read_text(encoding="utf-8").splitlines()
        docs, attr, in_hooks, i = [], None, False, 0
        while i < len(lines):
            line = lines[i].strip()
            if line.startswith("pub trait "):
                in_hooks = line.startswith("pub trait Hooks")
                docs, attr = [], None
            elif not in_hooks:
                pass
            elif line.startswith("///"):
                docs.append(line[3:].strip())
            elif line.startswith('#[notes = r##"'):
                while '"##]' not in lines[i]:
                    i += 1
            elif line.startswith("#[hook("):
                attr = line
                while not attr.rstrip().endswith(")]"):
                    i += 1
                    attr += " " + lines[i].strip()
            elif line.startswith("fn ") and attr:
                name = re.match(r"fn (\w+)", line).group(1)
                field = lambda key: (re.search(key + r'\s*=\s*"([^"]*)"', attr) or [None, None])[1]
                out.append({
                    "kind": "hook",
                    "name": name,
                    "side": field("side"),
                    "contract": field("contract"),
                    "scope": field("scope") or "global",
                    "doc": first_sentence(docs),
                })
                docs, attr = [], None
            i += 1
    return out


def host():
    out = []
    for path in sorted(HOST.glob("*.rs")):
        lines = path.read_text(encoding="utf-8").splitlines()
        docs, trait, hidden, i = [], None, False, 0
        while i < len(lines):
            line = lines[i].strip()
            if line.startswith("#[doc(hidden)]") and trait is None:
                hidden = True
            elif line.startswith("pub trait "):
                trait = re.match(r"pub trait (\w+)", line).group(1)
                docs = []
            elif lines[i] == "}" and trait:
                trait, hidden, docs = None, False, []
            elif trait and line.startswith("///"):
                docs.append(line[3:].strip())
            elif trait and re.match(r"fn \w+", line):
                name = re.match(r"fn (\w+)", line).group(1)
                if not hidden and not name.startswith("__"):
                    on = ("ServerHost" if trait.endswith("ServerHost")
                          else "ClientHost" if trait.endswith("ClientHost") else "Host")
                    out.append({
                        "kind": "host",
                        "name": name,
                        "area": path.stem,
                        "on": on,
                        "doc": first_sentence(docs),
                    })
                docs = []
            elif trait and not line.startswith("#[") and not line.startswith("//"):
                # A signature's continuation lines, or anything else, end no doc on their own.
                pass
            i += 1
    return out


# Global hooks by what their name is about, first match wins: (Russian title, English title, pattern).
THEMES = [
    ("Запуск, настройки и конфиги", "Startup, settings and config", r"^(client_setup|client_stop|game_shutting|config_|setting_|mod_mismatch|game_rule|difficulty|permissions|whitelist|scheduled_task|console_line|component_types|default_components|extend_poi|map_decoration_types)"),
    ("Регистрация и загрузка данных", "Registration and data loading", r"^(register|add_|gather|modify_|build_|create_|load|data_|tags|reload|datapack|recipe|loot|registry)"),
    ("Бой, урон и опыт", "Combat, damage and experience", r"(attack|hurt|critical|sweep|arrow|target_hit|potion|effect|xp|exp_|experience|creeper|tnt|ender|dragon|raid|siege|wind_burst)"),
    ("Игроки", "Players", r"player"),
    ("Сущности и мобы", "Entities and mobs", r"(entity|mob|living|spawn|projectile|villager|animal|baby|tame|mount|shear)"),
    ("Блоки", "Blocks", r"(block|piston|crop|sapling|fluid|bonemeal|farmland|portal|explosion|fire|note|bell|beacon|leaves|sponge|sculk|vault|moisture|neighbor|sign_|hopper|dispenser|bucket|hanging|grindstone|ground)"),
    ("Предметы, инвентарь, крафт", "Items, inventory and crafting", r"(item|stack|craft|smelt|enchant|anvil|brew|furnace|trade|container|menu|tooltip|equip|ingredient|smithing|armor|can_eat|consume|creative_slot|station)"),
    ("Мир, чанки, тики", "World, chunks and ticks", r"(level|world|chunk|tick|weather|thunder|time|sleep|structure|biome|feature|section|entities_load|entities_unload|vanilla_game_event|vehicle)"),
    ("Команды, чат, сеть", "Commands, chat and network", r"(command|chat|message|payload|packet|network|connection|login|server|disconnect|tab_complete|custom_click|client_information|resource_pack_status)"),
    ("Клиент: экран, интерфейс, ввод", "Client: screens, interface and input", r"(screen|gui|hud|overlay|input|key|mouse|touch|toast|narrat|title|click|panel|pause|hover|debug|minimap|boss_event|window|framerate)"),
    ("Клиент: рендер, звук, частицы", "Client: rendering, sound and particles", r"(render|model|texture|sprite|shader|particle|sound|music|streaming|camera|fov|zoom|fog|light|emissive|color|colour|arm_pose|hand_drawn|geometry|visuals|resources_reloaded|flip_frame|map_decorations)"),
]

# Each language's page: where it goes, its words, and the guide it links to.
LANGS = {
    "ru": {
        "page": DOCS / "modding-reference.md",
        "sides": {"server": "сервер", "client": "клиент", "both": "обе"},
        "contracts": {"listen": "слушает", "cancel": "может отменить",
                      "collect": "собирает ответы", "replace": "заменяет результат"},
        "scopes": {
            "global": "Глобальные хуки (`Hooks`)",
            "block": "Части блоков (`BlockPart`)",
            "block_entity": "Части блок-сущностей (`BlockEntityPart`)",
            "item": "Части предметов (`ItemPart`)",
            "entity": "Части сущностей (`EntityPart`)",
            "menu": "Части меню (`MenuPart`)",
            "fluid": "Части жидкостей (`FluidPart`)",
        },
        "other": "Прочее",
        "scope": "Область",
        "hook_head": "| Хук | Сторона | Контракт | Что делает |",
        "host_head": "| Метод | Где | Что делает |",
        "head": [
            "# Справочник функций для модов",
            "",
            "Собран из исходников мод-API скриптом `tools/modding_reference.py` (не править руками).",
            "Здесь все хуки каталога и все методы `Host`: имя, сторона, контракт и одна строка о том,",
            "что делает функция. Полное описание с параметрами — в rustdoc крейта `horizon-mod-api`",
            "(`cargo doc -p horizon-mod-api --no-deps --open`). Как всё это устроено — в",
            "[гайде по моддингу](modding.md). English: [modding-reference.en.md](modding-reference.en.md).",
            "",
            "Хуков: **{hooks}**, методов Host: **{host}**.",
            "",
            "**Контракт хука:** «слушает» — узнаёт о событии; «может отменить» — может запретить",
            "действие; «собирает ответы» — ответы всех модов складываются; «заменяет результат» —",
            "первый ответивший мод заменяет ванильный ответ.",
            "",
            "## Хуки",
            "",
        ],
        "host": [
            "## Методы Host",
            "",
            "`Host` — то, что хук может спросить или сделать в игре. Методы `ServerHost` доступны только",
            "на сервере, `ClientHost` — только на клиенте, `Host` — на обеих сторонах.",
            "",
        ],
    },
    "en": {
        "page": DOCS / "modding-reference.en.md",
        "sides": {"server": "server", "client": "client", "both": "both"},
        "contracts": {"listen": "listen", "cancel": "cancel", "collect": "collect",
                      "replace": "replace"},
        "scopes": {
            "global": "Global hooks (`Hooks`)",
            "block": "Block parts (`BlockPart`)",
            "block_entity": "Block entity parts (`BlockEntityPart`)",
            "item": "Item parts (`ItemPart`)",
            "entity": "Entity parts (`EntityPart`)",
            "menu": "Menu parts (`MenuPart`)",
            "fluid": "Fluid parts (`FluidPart`)",
        },
        "other": "Other",
        "scope": "Scope",
        "hook_head": "| Hook | Side | Contract | What it does |",
        "host_head": "| Method | On | What it does |",
        "head": [
            "# Mod function reference",
            "",
            "Generated from the mod API's sources by `tools/modding_reference.py` (do not edit by hand).",
            "Every catalog hook and every `Host` method: its name, side, contract and one line on what",
            "it does. The full documentation with parameters is the rustdoc of `horizon-mod-api`",
            "(`cargo doc -p horizon-mod-api --no-deps --open`). How it all fits together: the",
            "[modding guide](modding.en.md). По-русски: [modding-reference.md](modding-reference.md).",
            "",
            "Hooks: **{hooks}**, Host methods: **{host}**.",
            "",
            "**A hook's contract:** *listen*: told of the event; *cancel*: may stop the game's",
            "action; *collect*: every mod's answers are gathered; *replace*: the first mod to answer",
            "replaces vanilla's answer.",
            "",
            "## Hooks",
            "",
        ],
        "host": [
            "## Host methods",
            "",
            "`Host` is what a hook may ask of the game or do in it. `ServerHost` methods exist on the",
            "server only, `ClientHost` methods on the client only, `Host` methods on both sides.",
            "",
        ],
    },
}


def theme(name, lang):
    for ru_title, en_title, pattern in THEMES:
        if re.search(pattern, name):
            return ru_title if lang == "ru" else en_title
    return LANGS[lang]["other"]


def render(lang):
    words = LANGS[lang]
    entries = hooks() + host()
    ru = json.loads(RU.read_text(encoding="utf-8")) if RU.exists() and lang == "ru" else {}
    say = lambda e: ru.get(key(e)) or e["doc"]
    hook_rows = [e for e in entries if e["kind"] == "hook"]
    host_rows = [e for e in entries if e["kind"] == "host"]
    side = lambda e: words["sides"].get(e["side"], e["side"])
    contract = lambda e: words["contracts"].get(e["contract"], e["contract"])
    out = [line.format(hooks=len(hook_rows), host=len(host_rows)) for line in words["head"]]
    titles = [t[0] if lang == "ru" else t[1] for t in THEMES] + [words["other"]]
    for scope, title in words["scopes"].items():
        rows = [e for e in hook_rows if e["scope"] == scope]
        if not rows:
            continue
        out += [f"### {title}", ""]
        groups = {}
        for e in rows:
            groups.setdefault(theme(e["name"], lang) if scope == "global" else "", []).append(e)
        for group in titles if scope == "global" else [""]:
            if group not in groups:
                continue
            if group:
                out += [f"#### {group}", ""]
            out += [words["hook_head"], "| --- | --- | --- | --- |"]
            for e in sorted(groups[group], key=lambda e: e["name"]):
                out.append(f"| `{e['name']}` | {side(e)} | {contract(e)} | {cell(say(e))} |")
            out.append("")
    for scope in sorted({e["scope"] for e in hook_rows} - set(words["scopes"])):
        out += [f"### {words['scope']} `{scope}`", "", words["hook_head"], "| --- | --- | --- | --- |"]
        for e in sorted((e for e in hook_rows if e["scope"] == scope), key=lambda e: e["name"]):
            out.append(f"| `{e['name']}` | {side(e)} | {contract(e)} | {cell(say(e))} |")
        out.append("")
    out += words["host"]
    for area in sorted({e["area"] for e in host_rows}):
        out += [f"### `{area}`", "", words["host_head"], "| --- | --- | --- |"]
        for e in (e for e in host_rows if e["area"] == area):
            out.append(f"| `{e['name']}` | `{e['on']}` | {cell(say(e))} |")
        out.append("")
    words["page"].write_text("\n".join(out), encoding="utf-8", newline="\n")
    missing = sum(1 for e in entries if key(e) not in ru) if lang == "ru" else 0
    print(f"{words['page'].relative_to(ROOT)}: {len(hook_rows)} hooks, {len(host_rows)} host "
          f"methods, {missing} without a translation", file=sys.stderr)


def key(e):
    return f"{e['kind']}:{e.get('area', '')}:{e['name']}"


def cell(text):
    # Rustdoc's intra-doc links ([`WorldView`]) read as plain code here.
    text = re.sub(r"\[(`[^`]*`)\]", r"\1", text)
    return text.replace("|", "\\|").replace("\n", " ")


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else "render"
    if command == "extract":
        entries = hooks() + host()
        json.dump([dict(e, key=key(e)) for e in entries], sys.stdout, ensure_ascii=False, indent=1)
    elif command == "render":
        for lang in LANGS:
            render(lang)
    else:
        sys.exit(__doc__)
