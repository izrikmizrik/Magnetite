#!/usr/bin/env python3
"""Builds the tables zip a phone installs from: every file of a prepared resource folder that is
not Mojang's, plus `setup.json`, which names what the phone downloads from Mojang itself.

    python3 -I make_tables.py PREPARED FETCHED OUT_ZIP

PREPARED is `horizon-prepare --complete`'s output folder (`--output prepared`), FETCHED is
`fetch_minecraft.py`'s (`version.json`, `client.jar`, `assets/indexes/<id>.json`), and OUT_ZIP is
what `magnetite-tables` publishes.

Every file of PREPARED is classified, by content and not by name alone:

- `jar`: its path is a member of `client.jar`, its bytes are that member's, and the preparation
  copies that member (a `jar` file that an index object overrides at its own path is the object:
  the panorama, fonts, languages and built-in packs come from the index, not from the jar's
  placeholders);
- `object`: its SHA-1 is an asset-index object's whose name puts it at this very path;
- `index`: it is `asset-index.json`, the index itself (the phone downloads its own copy);
- `horizon`: everything else — the exporters' tables, the world-generation bundles, the marker.

`setup.json` is the manifest `crates/horizon-client/src/game_files.rs` reads: `minecraft`,
`client{url,sha1,size}`, `asset_index{id,url,sha1}`, `jar_files` and `tables_root`. `jar_files`
names the exact jar members present in PREPARED (a name ending in `/` would be a whole folder, the
game files installer takes either), so the phone writes precisely those and nothing else.

Where an asset object goes is decided by the installer, not here: `download_objects` writes
`inside(&staging.join("assets"), &object.name)`, and the preparation's own copy does the same
(`copy_indexed`: `target.join("assets").join(relative)`). So index key `K` is the resource file
`assets/K` — one function, `object_path`, states it in both programs' terms.

`assets/` and `data/` are the phone's to rebuild, from the jar members `jar_files` names and from
the index, so a prepared file there that is neither is refused: a table cannot shadow Mojang's
assets, and the install has no way to write one. That refusal is also what makes the install
match this folder: everything the preparation holds, it writes again — Mojang's parts from the
jar and the index, the rest from these tables. The install is a superset in one place only: the
installer downloads every object of the index, while a preparation keeps the ones the game reads
(sounds, languages, panorama, fonts, the built-in packs); the extra objects are unused bytes, not
a difference in any file this folder has.

Refused, with the list, instead of silently dropped or published:
- a prepared file under `assets/` or `data/` that the installer would not write at that path;
- a prepared file that is the jar's bytes where an index object of the same name gives other
  bytes (the installer writes the object second, so the file would not survive);
A prepared file that is Mojang's bytes under another path (the world generation bundle's
`worldgen/structure/*/**.nbt` are the jar's templates byte for byte) stays out of the archive and
goes in `setup.json`'s `copies`: the install copies it from the player's own jar or objects.

The archive is then read back and proved: no member is Mojang's bytes at any path (files of 64
bytes or less are exempt and counted — a two-byte `{}` matching a jar entry says nothing), and
the members are exactly `setup.json` plus the tables.

Logs, compiled exporter classes, the preparer's stamp, the run time cache `horizon-data.bin` and
the lock file are not resources and are left out, each counted in the run's report.
"""

import argparse
import hashlib
import json
import os
import sys
import zipfile
from pathlib import Path

# Every entry's date in the zip: the same input always makes the same bytes.
FIXED_DATE = (2026, 1, 1, 0, 0, 0)
REPO = Path(__file__).resolve().parent.parent
LOCK = REPO / "research-lock.json"
# The manifest's name inside the zip (game_files.rs, MANIFEST).
MANIFEST = "setup.json"
# The marker a preparation leaves and the game refuses to start without (game_files.rs, MARKER).
MARKER = "horizon-resources.json"
STAMP = "horizon-prepare.json"
CACHE = "horizon-data.bin"
# The folder inside the zip the tables live in (setup.json's `tables_root`).
TABLES_ROOT = "tables"
# The folder the phone writes index objects into (game_files.rs, download_objects).
ASSETS = "assets"
# What the preparation copies out of the jar (lib.rs, extract_client_resources).
JAR_FOLDERS = ("assets/", "data/")
JAR_FILES = ("pack.png",)
# Not resources: an exporter's console output, its classes, a half-written download.
JUNK_SUFFIXES = (".log", ".class", ".tmp", ".part")
# Directories holding an exporter's compiled classes rather than resources (lib.rs).
CLASS_DIRS = {"classes", "client-classes"}
# A file this small cannot be a copy of Mojang's data, whatever its hash says.
EXEMPT_BYTES = 64
# How many names an error lists before it says "and N more".
SHOWN = 20


def sha1_of(path):
    digest = hashlib.sha1()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def is_hex(text, length):
    return (
        isinstance(text, str)
        and len(text) == length
        and all(c in "0123456789abcdefABCDEF" for c in text)
    )


def read_json(path):
    with path.open(encoding="utf-8") as stream:
        return json.load(stream)


def safe_relative(name):
    """A manifest or index name as a relative path, or None (artifacts.rs, safe_relative)."""
    if not isinstance(name, str) or not name or "\\" in name or name.startswith("/"):
        return None
    parts = [p for p in name.split("/") if p and p != "."]
    if not parts or ".." in parts or (len(parts[0]) > 1 and parts[0][1] == ":"):
        return None
    return "/".join(parts)


def object_path(key):
    """Where the phone puts the asset object named `key`: `assets/<key>`.

    The installer's `download_objects` writes `inside(staging/assets, object.name)` and the
    preparation's `copy_indexed` writes `target/assets/<safe_relative(name)>`; both are
    `assets/` + the index key.
    """
    relative = safe_relative(key)
    return None if relative is None else ASSETS + "/" + relative


def jar_records(jar):
    """Every file member of `jar`: {name: sha1}, refusing a link or a damaged archive."""
    records = {}
    with zipfile.ZipFile(jar) as archive:
        for info in archive.infolist():
            if info.filename.endswith("/"):
                continue
            # A symlink's mode is in the high half of external_attr; the installer refuses one.
            if (info.external_attr >> 16) & 0o170000 == 0o120000:
                raise SystemExit("the client jar holds a link: " + info.filename)
            digest = hashlib.sha1()
            with archive.open(info) as member:
                for chunk in iter(lambda: member.read(1 << 20), b""):
                    digest.update(chunk)
            records[info.filename] = digest.hexdigest()
    return records


def copies_from_jar(name):
    """Whether the preparation copies this jar member into the resources (lib.rs)."""
    return name.startswith(JAR_FOLDERS) or name in JAR_FILES


def junk(rel):
    """Why `rel` is not a resource, or None."""
    name = rel.rsplit("/", 1)[-1]
    if name.startswith(".") or rel.startswith("."):
        return "hidden"
    if name.endswith(JUNK_SUFFIXES):
        return "build output"
    if rel.split("/", 1)[0] in CLASS_DIRS:
        return "exporter classes"
    if rel == STAMP:
        return "the preparer's stamp"
    if rel == CACHE or rel.startswith(CACHE + "."):
        return "the run time cache"
    return None


def walk(prepared):
    """Every file of `prepared`, as (`/`-separated relative path, Path), in name order."""
    found = []
    for folder, dirs, names in os.walk(prepared):
        dirs.sort()
        for name in sorted(names):
            path = Path(folder) / name
            rel = path.relative_to(prepared).as_posix()
            found.append((rel, path))
    found.sort()
    return found


def case_collisions(names):
    """Paths that differ only by case: two zip entries a case-insensitive folder merges."""
    folded = {}
    for name in names:
        folded.setdefault(name.lower(), []).append(name)
    return [paths for paths in folded.values() if len(paths) > 1]


def classify(prepared, jar, index_objects):
    """The files by kind, and the ones the installer would not reproduce there."""
    kinds = {"jar": [], "object": [], "index": [], "horizon": []}
    left_out = {}
    irreproducible = []
    sizes = {}
    for rel, path in walk(prepared):
        reason = junk(rel)
        if reason:
            left_out[reason] = left_out.get(reason, 0) + 1
            continue
        if path.is_symlink():
            raise SystemExit("the prepared folder holds a link, not a file: " + rel)
        sha1 = sha1_of(path)
        size = path.stat().st_size
        sizes[rel] = (sha1, size)
        kind = "horizon"
        if rel == "asset-index.json":
            # The phone downloads its own copy; the file must be the index this manifest pins.
            if sha1 != index_objects["sha1"]:
                irreproducible.append(
                    (rel, "not the asset index this zip pins (" + index_objects["sha1"] + ")")
                )
            kind = "index"
        elif rel.startswith(ASSETS + "/"):
            # The preparation writes `assets/` from the index alone: `extract_client_resources`
            # copies the jar's members, and an index object of the same name then overwrites
            # them (`Assets::imported` keeps the object, `copy_indexed` writes it), and
            # `--complete` copies the rest. So a file here is that index object, and anything
            # else is a file the install would not write.
            entry = index_objects["by_key"].get(rel[len(ASSETS) + 1:])
            if entry is not None and entry["hash"] == sha1 and entry["size"] == size:
                kind = "object"
            else:
                irreproducible.append(
                    (rel, "the phone writes assets/ only from the jar and the index")
                )
        elif rel.startswith("data/"):
            # `data/` is written from the jar alone (extract_client_resources, then an index
            # object over any name it repeats), so a file here is the jar's member or nothing.
            if rel in jar and copies_from_jar(rel) and jar[rel] == sha1:
                kind = "jar"
            else:
                irreproducible.append((rel, "the phone writes data/ only from the jar"))
        elif rel in jar and copies_from_jar(rel) and jar[rel] == sha1:
            kind = "jar"
        elif rel in jar and copies_from_jar(rel):
            irreproducible.append((rel, "differs from the client jar's member of that name"))
        elif rel in jar:
            # A jar member the preparation does not copy: the phone would not have it either, so
            # there is nothing to publish and nothing lost.
            left_out["a jar member the preparation leaves out"] = (
                left_out.get("a jar member the preparation leaves out", 0) + 1
            )
        kinds[kind].append(rel)
    return kinds, sizes, left_out, irreproducible


def index_objects(index, sha1):
    """The index by name and by hash."""
    objects = index.get("objects")
    if not isinstance(objects, dict) or not objects:
        raise SystemExit("the asset index has no objects")
    by_key = {}
    by_hash = {}
    for key in sorted(objects):
        info = objects[key]
        digest = info.get("hash") if isinstance(info, dict) else None
        if not is_hex(digest, 40):
            raise SystemExit("the asset index gives " + str(key) + " a bad hash")
        if object_path(key) is None:
            raise SystemExit("the asset index names an unsafe path: " + str(key))
        size = info.get("size")
        size = size if isinstance(size, int) and size >= 0 else -1
        by_key[key] = {"hash": digest.lower(), "size": size}
        by_hash.setdefault(digest.lower(), []).append(key)
    return {"sha1": sha1, "by_key": by_key, "by_hash": by_hash}


def report(problems, what):
    for rel, why in problems[:SHOWN]:
        print("  " + rel + ": " + why, file=sys.stderr)
    if len(problems) > SHOWN:
        print("  ... and {} more".format(len(problems) - SHOWN), file=sys.stderr)
    print(what, file=sys.stderr)


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("prepared", type=Path, help="horizon-prepare --complete's --output folder")
    parser.add_argument("fetched", type=Path, help="fetch_minecraft.py's folder")
    parser.add_argument("out_zip", type=Path, help="the tables zip to write")
    args = parser.parse_args()
    (prepared, fetched, out_zip) = (args.prepared, args.fetched, args.out_zip)
    if not prepared.is_dir():
        raise SystemExit("no prepared folder: " + str(prepared))
    if not fetched.is_dir():
        raise SystemExit("no fetched folder: " + str(fetched))
    if not LOCK.is_file():
        raise SystemExit("no research-lock.json beside this checkout: " + str(LOCK))
    with LOCK.open(encoding="utf-8") as stream:
        lock = json.load(stream)["minecraft"]
    version_id = lock["version"]
    accepted_index = set(h.lower() for h in lock.get("accepted_asset_index_sha1") or [])
    print("make_tables: {} from {}, Mojang's files from {}".format(out_zip, prepared, fetched))

    # The version JSON, the client jar and the index: the three things the manifest and the
    # classification are read from.
    version_path = fetched / "version.json"
    if not version_path.is_file():
        raise SystemExit("no {}: run fetch_minecraft.py first".format(version_path))
    version = read_json(version_path)
    if version.get("id") != version_id:
        raise SystemExit(
            "version.json is for {}, research-lock.json pins {}".format(
                version.get("id"), version_id
            )
        )
    client_blob = (version.get("downloads") or {}).get("client") or {}
    client_url = client_blob.get("url")
    if client_blob.get("sha1") != lock["client_sha1"] or not client_url:
        raise SystemExit("version.json's client download is not the pinned one")
    jar_path = fetched / "client.jar"
    if not jar_path.is_file():
        raise SystemExit("no {}: run fetch_minecraft.py first".format(jar_path))
    if sha1_of(jar_path) != lock["client_sha1"]:
        raise SystemExit("{} is not the pinned client jar: fetch it again".format(jar_path))
    jar = jar_records(jar_path)
    print(
        "  client.jar  {:>5} members, {:>8} MB".format(
            len(jar), round(jar_path.stat().st_size / 1e6, 1)
        )
    )

    # The preparation's own index: it is what the phone's objects come from, so it decides both
    # the classification and the manifest's `asset_index`.
    prepared_index = prepared / "asset-index.json"
    if not prepared_index.is_file():
        raise SystemExit(
            "{} has no asset-index.json: prepare with --complete".format(prepared)
        )
    index_sha1 = sha1_of(prepared_index)
    version_index = version.get("assetIndex") or {}
    index_id = version_index.get("id")
    if not index_id:
        raise SystemExit("version.json has no assetIndex id")
    pinned_index = (version_index.get("sha1") or "").lower()
    if index_sha1 not in accepted_index and index_sha1 != pinned_index:
        raise SystemExit(
            "asset-index.json's sha1 {} is neither the version JSON's ({}) nor one Horizon "
            "knows ({})".format(
                index_sha1, pinned_index or "none", ", ".join(sorted(accepted_index)) or "none"
            )
        )
    objects = index_objects(read_json(prepared_index), index_sha1)
    print(
        "  index       {} names ({} distinct contents), sha1 {}".format(
            len(objects["by_key"]), len(objects["by_hash"]), index_sha1
        )
    )

    # Every file, by what makes it.
    kinds, sizes, left_out, irreproducible = classify(prepared, jar, objects)
    if MARKER not in kinds["horizon"]:
        raise SystemExit(
            "{} has no {}: not a finished preparation".format(prepared, MARKER)
        )
    # A prepared file under assets/ or data/ the install would not write at that path: the two
    # folders would differ, so nothing is published.
    if irreproducible:
        report(
            irreproducible,
            "these files are not what the installer would write, so the phone's folder would "
            "differ from this one:",
        )
        return 1

    # No table file may hold Mojang's bytes, whatever its name: the check is by content, over
    # every member of client.jar and every object of the index, not by paths. A file that is such
    # a copy is never published; `jar_files` names members only at their own paths, so the phone
    # does not get it either: it is listed, and what it was meant to be stays missing.
    mojang_bodies = set(jar.values()) | set(objects["by_hash"])
    # A jar member by its SHA-1, for naming a copy of one; an object otherwise.
    jar_contents = {sha1: name for name, sha1 in jar.items()}
    object_contents = {sha1: keys[0] for sha1, keys in objects["by_hash"].items()}
    tables = []
    copies = []
    exempt = 0
    for rel in sorted(kinds["horizon"]):
        sha1, size = sizes[rel]
        if sha1 in mojang_bodies:
            # A file of a few bytes that matches (`{}` in a jar member): nothing of Mojang's.
            if size <= EXEMPT_BYTES:
                exempt += 1
                tables.append(rel)
            else:
                copies.append(rel)
        else:
            tables.append(rel)

    # setup.json, in the format crates/horizon-client/src/game_files.rs reads.
    declared = version_index.get("url") if pinned_index == index_sha1 else None
    setup = {
        "minecraft": version_id,
        "client": {
            "url": client_url,
            "sha1": lock["client_sha1"],
            "size": client_blob.get("size") or jar_path.stat().st_size,
        },
        "asset_index": {
            "id": index_id,
            "url": declared
            or "https://piston-data.mojang.com/v1/objects/{}/{}.json".format(
                index_sha1, index_id
            ),
            "sha1": index_sha1,
        },
        "jar_files": sorted(kinds["jar"]),
        "tables_root": TABLES_ROOT,
        # Prepared files that are Mojang's records under another path (the world generation
        # bundle's structure templates): the install copies them from the player's own jar or
        # objects, so nothing of Mojang's is published and the phone's folder still matches.
        "copies": [
            {"jar": jar_contents[sizes[rel][0]], "to": rel}
            if sizes[rel][0] in jar_contents
            else {"object": object_contents[sizes[rel][0]], "to": rel}
            for rel in copies
        ],
    }

    # The zip: fixed dates, sorted names, 0644, one compression level, the manifest first.
    out_zip.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(out_zip, "w", zipfile.ZIP_DEFLATED, compresslevel=9) as archive:

        def add(name, data):
            info = zipfile.ZipInfo(name, FIXED_DATE)
            info.compress_type = zipfile.ZIP_DEFLATED
            info.create_system = 3
            info.external_attr = 0o644 << 16
            archive.writestr(info, data)

        add(MANIFEST, json.dumps(setup, separators=(",", ":")).encode())
        for rel in tables:
            add(TABLES_ROOT + "/" + rel, (prepared / rel).read_bytes())

    # The archive's own proof, read back: every member's SHA-1 must not be Mojang's bytes (by
    # content, at whatever path), and the members must be exactly `setup.json` and the tables.
    expected = {MANIFEST} | {TABLES_ROOT + "/" + rel for rel in tables}
    leaked = []
    with zipfile.ZipFile(out_zip) as archive:
        names = set(archive.namelist())
        for name in sorted(names):
            if name == MANIFEST:
                continue
            body = archive.read(name)
            # A few bytes that match (`{}`) are nobody's data; a real file that matches is.
            if len(body) > EXEMPT_BYTES and hashlib.sha1(body).hexdigest() in mojang_bodies:
                leaked.append(name)
        extra = sorted(names - expected)
    if leaked or extra:
        out_zip.unlink(missing_ok=True)
        if leaked:
            report(
                [(name, "holds Mojang's bytes") for name in leaked],
                "the archive was removed: these members are Mojang's files:",
            )
        if extra:
            report(
                [(name, "not the manifest or a table") for name in extra],
                "the archive was removed: it holds entries this tool did not write:",
            )
        return 1

    # The way the install could write something other than this folder holds: a prepared file
    # that is one of Mojang's records under a path `jar_files` cannot name. The world generation
    # bundle's `worldgen/structure/*/**.nbt` are the jar's templates, byte for byte. They are not
    # published (Mojang's bytes), and an install can only write a member where the jar has it, so
    # the phone would lack them: refused unless the run says it accepts that gap.
    unreproduced = [
        (
            rel,
            "is "
            + (
                "client.jar's " + jar_contents[sha1]
                if sha1 in jar_contents
                else "the asset object named " + object_contents[sha1]
            )
            + " under another path",
        )
        for rel in copies
        for sha1 in [sizes[rel][0]]
    ]
    if unreproduced:
        print(
            "  copied on the phone {:>5} prepared files are Mojang's records under another path".format(
                len(unreproduced)
            )
        )
        for rel, why in unreproduced[:SHOWN]:
            print("    " + rel + " " + why)
        if len(unreproduced) > SHOWN:
            print("    ... and {} more".format(len(unreproduced) - SHOWN))

    # Two entries a folder merges (one name, two spellings of its case): the install would keep
    # the later one and lose the other, so the phone's folder could not hold both. Refused: the
    # preparation on a case-insensitive runner already lost one, and on this one it would.
    collisions = case_collisions(
        [name for name in setup["jar_files"]]
        + [object_path(key) for key in objects["by_key"]]
        + [TABLES_ROOT + "/" + rel for rel in tables]
    )
    if collisions:
        out_zip.unlink(missing_ok=True)
        report(
            [(paths[0], "shares its name with " + ", ".join(paths[1:])) for paths in collisions],
            "the archive was removed: a case-insensitive folder cannot hold both entries:",
        )
        return 1

    digest = hashlib.sha256(out_zip.read_bytes()).hexdigest()
    jar_bytes = sum(sizes[rel][1] for rel in kinds["jar"])
    object_bytes = sum(
        entry["size"] for entry in objects["by_key"].values() if entry["size"] > 0
    )
    from_mojang = (
        setup["client"]["size"] + object_bytes + prepared_index.stat().st_size
    )
    print(
        "  tables      {:>5} files, {:>8} MB, sha256 {}".format(
            len(tables), round(out_zip.stat().st_size / 1e6, 1), digest
        )
    )
    print(
        "  phone takes {:>5} jar files ({} MB), {} objects ({} MB), the index ({} MB); "
        "{} MB from Mojang in all".format(
            len(kinds["jar"]),
            round(jar_bytes / 1e6, 1),
            len(objects["by_key"]),
            round(object_bytes / 1e6, 1),
            round(prepared_index.stat().st_size / 1e6, 1),
            round(from_mojang / 1e6, 1),
        )
    )
    for reason in sorted(left_out):
        print("  left out    {:>5} {}".format(left_out[reason], reason))
    if exempt:
        print("  exempt      {:>5} files of {} bytes or less".format(exempt, EXEMPT_BYTES))
    print(
        "make_tables: {} in {} ({} files, {:.1f} MB)".format(
            out_zip, prepared, len(tables), out_zip.stat().st_size / 1e6
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
