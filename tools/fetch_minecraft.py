#!/usr/bin/env python3
"""Fetches the pinned Minecraft version from Mojang's own servers into one folder.

    python3 -I fetch_minecraft.py OUT_DIR

Everything is pinned: `version.json` by research-lock.json's `manifest_sha1` and the
`accepted_manifest_sha1` list (Mojang republished the 26.3 version JSON a few times; only the
asset index differs), the two jars by their `client_sha1`/`server_sha1`, the asset index by
`accepted_asset_index_sha1` or by the SHA-1 the version JSON names, and every library and asset
object by the SHA-1 the version JSON and the index give it.

Writes OUT_DIR/version.json, client.jar, server.jar, libraries/ (Maven layout, this machine's
artifacts), assets/indexes/<id>.json and assets/objects/<2>/<hash>: the files
`horizon-prepare --prepare` imports, and the asset store `--complete` takes the sounds, languages,
panorama, fonts and built-in packs from. The library rules are read for Linux, which the table
build runs on (research-lock.json pins Java 25 for it).

Eight downloads at a time; a file already in place with the right hash is kept (an interrupted
run continues), each download is written beside its name and renamed only once its hash matches,
and a failed one is tried four times. https only, and only on Mojang's own hosts — nothing is
uploaded, and nothing is written outside OUT_DIR.
"""

import argparse
import hashlib
import json
import platform
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

MANIFEST_URL = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
HOSTS = (
    "piston-meta.mojang.com",
    "piston-data.mojang.com",
    "libraries.minecraft.net",
    "resources.download.minecraft.net",
)
OBJECTS = "https://resources.download.minecraft.net"
# tools/ sits in the repository root, beside research-lock.json.
LOCK = Path(__file__).resolve().parent.parent / "research-lock.json"
LANES = 8
TRIES = 4
CHUNK = 64 * 1024
USER_AGENT = "Magnetite tables build (Minecraft 26.3 preparation)"


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """Refuses a redirect: an address that moved is an error, never a silent second host."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise urllib.error.HTTPError(
            req.full_url, code, "redirected to " + newurl, headers, fp
        )


OPENER = urllib.request.build_opener(NoRedirect)


def allowed(url):
    """https on one of Mojang's hosts, with no credentials and on the default port."""
    parts = urllib.parse.urlsplit(url)
    return (
        parts.scheme == "https"
        and parts.hostname in HOSTS
        and not parts.username
        and not parts.password
        and parts.port in (None, 443)
    )


def get(url, limit=64 * 1024 * 1024):
    """`url`'s bytes, held in memory: JSON and other small files only."""
    if not allowed(url):
        raise SystemExit("refusing an address that is not https on Mojang's hosts: " + url)
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with OPENER.open(request, timeout=60) as response:
        body = response.read(limit + 1)
    if len(body) > limit:
        raise SystemExit(url + ": larger than the " + str(limit) + " bytes it may be")
    return body


def sha1_bytes(body):
    return hashlib.sha1(body).hexdigest()


def sha1_of(path):
    digest = hashlib.sha1()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url, destination, sha1):
    """`url` into `destination`, checked against `sha1`; returns the bytes fetched (0 if kept)."""
    if destination.is_file() and sha1_of(destination) == sha1:
        return 0
    destination.parent.mkdir(parents=True, exist_ok=True)
    part = destination.with_name(destination.name + ".part")
    last = url + ": not downloaded"
    for attempt in range(TRIES):
        if attempt:
            time.sleep(1 << attempt)
        try:
            digest = hashlib.sha1()
            got = 0
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with OPENER.open(request, timeout=60) as response:
                with part.open("wb") as out:
                    for chunk in iter(lambda: response.read(CHUNK), b""):
                        out.write(chunk)
                        digest.update(chunk)
                        got += len(chunk)
            if digest.hexdigest() != sha1:
                raise ValueError("the download does not match its hash " + sha1)
            part.replace(destination)
            return got
        except Exception as error:  # every failure is retried, with its own message
            part.unlink(missing_ok=True)
            last = url + ": " + str(error)
    raise SystemExit(last)


def download_any(url, destination, shas):
    """`url` into `destination` when its SHA-1 is one of `shas`; the SHA-1 it ended with.

    A republished asset index keeps its id while its bytes change, so the index is accepted when
    it matches the version JSON's hash or one research-lock.json lists.
    """
    if destination.is_file() and sha1_of(destination) in shas:
        return sha1_of(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    part = destination.with_name(destination.name + ".part")
    last = url + ": not downloaded"
    for attempt in range(TRIES):
        if attempt:
            time.sleep(1 << attempt)
        try:
            digest = hashlib.sha1()
            request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
            with OPENER.open(request, timeout=60) as response:
                with part.open("wb") as out:
                    for chunk in iter(lambda: response.read(CHUNK), b""):
                        out.write(chunk)
                        digest.update(chunk)
            actual = digest.hexdigest()
            if actual not in shas:
                raise ValueError(
                    "sha1 " + actual + " is neither the version JSON's nor one Horizon knows"
                )
            part.replace(destination)
            return actual
        except Exception as error:  # every failure is retried, with its own message
            part.unlink(missing_ok=True)
            last = url + ": " + str(error)
    raise SystemExit(last)


def is_hex(text, length):
    return (
        isinstance(text, str)
        and len(text) == length
        and all(c in "0123456789abcdefABCDEF" for c in text)
    )


def arch_names():
    """This machine's architecture names, as version JSONs spell them."""
    machine = platform.machine().lower()
    if machine in ("x86_64", "amd64"):
        return ("x86_64", "amd64", "x64")
    if machine in ("aarch64", "arm64"):
        return ("arm64", "aarch64")
    if machine in ("i386", "i686", "x86"):
        return ("i686",)
    return (machine,)


def rule_matches(rule):
    """One launcher rule, read as horizon-prepare reads it (artifacts.rs, `library_allowed`)."""
    system = rule.get("os") or {}
    name = system.get("name")
    if name is not None and name != "linux":
        return False
    arch = system.get("arch")
    if arch is not None:
        try:
            pattern = re.compile("^(?:" + arch + ")$")
        except re.error:
            return False
        if not any(pattern.match(n) for n in arch_names()):
            return False
    version = system.get("version")
    if version is not None:
        try:
            pattern = re.compile(version)
        except re.error:
            return False
        if not pattern.match(platform.release()):
            return False
    # A rule for a feature (a demo-only library) never applies to a game download.
    return not rule.get("features")


def allowed_library(library):
    """The last matching rule decides, as in the official launcher; no rules means allowed."""
    rules = library.get("rules") or []
    if not rules:
        return True
    result = False
    for rule in rules:
        if rule_matches(rule):
            result = rule.get("action") == "allow"
    return result


def library_jobs(version, out):
    """(url, path, sha1) for the artifacts the version JSON lists for this machine."""
    jobs = []
    for library in version.get("libraries") or []:
        artifact = (library.get("downloads") or {}).get("artifact")
        if not artifact or not allowed_library(library):
            continue
        url = artifact.get("url")
        sha1 = artifact.get("sha1")
        path = artifact.get("path")
        if not (url and path and is_hex(sha1, 40)):
            raise SystemExit("a library in the version JSON has no url, path or sha1")
        if not allowed(url):
            raise SystemExit("a library's address is not https on a Mojang host: " + str(url))
        jobs.append((url, out / "libraries" / path, sha1))
    return jobs


def object_jobs(index, out):
    """(url, path, sha1) for every object of the asset index, once per hash."""
    objects = index.get("objects")
    if not isinstance(objects, dict) or not objects:
        raise SystemExit("the asset index has no objects")
    jobs = []
    seen = set()
    for name in sorted(objects):
        entry = objects[name]
        sha1 = entry.get("hash") if isinstance(entry, dict) else None
        if not is_hex(sha1, 40):
            raise SystemExit("the asset index gives " + str(name) + " a bad hash")
        if sha1 in seen:
            continue
        seen.add(sha1)
        jobs.append((OBJECTS + "/" + sha1[:2] + "/" + sha1,
                     out / "assets" / "objects" / sha1[:2] / sha1, sha1))
    return jobs, objects


def run(jobs, label):
    """The jobs on LANES threads; returns the bytes fetched."""
    if not jobs:
        return 0
    started = time.monotonic()
    with ThreadPoolExecutor(max_workers=LANES) as pool:
        fetched = list(pool.map(lambda job: download(*job), jobs))
    kept = sum(1 for n in fetched if n == 0)
    done = sum(fetched)
    print(
        "  {:<12} {:>5} files {:>8} MB ({} fetched, {} kept) in {:.1f} s".format(
            label,
            len(jobs),
            round(done / 1e6, 1),
            len(jobs) - kept,
            kept,
            time.monotonic() - started,
        )
    )
    return done


def mb(path):
    return round(path.stat().st_size / 1e6, 1) if path.is_file() else 0.0


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("out", type=Path, help="the folder the pinned version is written to")
    out = parser.parse_args().out
    started = time.monotonic()
    if not LOCK.is_file():
        raise SystemExit("no research-lock.json beside this checkout: " + str(LOCK))
    with LOCK.open(encoding="utf-8") as stream:
        lock = json.load(stream)["minecraft"]
    version_id = lock["version"]
    accepted_json = {lock["manifest_sha1"]} | set(lock.get("accepted_manifest_sha1") or [])
    accepted_index = set(lock.get("accepted_asset_index_sha1") or [])
    out.mkdir(parents=True, exist_ok=True)
    print("fetch_minecraft: Minecraft {} into {}".format(version_id, out))

    # The version manifest names the version's own JSON; that file states every other hash.
    manifest = json.loads(get(MANIFEST_URL))
    entry = next((v for v in manifest.get("versions") or [] if v.get("id") == version_id), None)
    if entry is None or not entry.get("url"):
        raise SystemExit("the version manifest has no " + version_id)
    before = time.monotonic()
    version_path = out / "version.json"
    if not (version_path.is_file() and sha1_of(version_path) in accepted_json):
        body = get(entry["url"], limit=8 * 1024 * 1024)
        actual = sha1_bytes(body)
        if actual not in accepted_json:
            raise SystemExit(
                "{}: sha1 {} is not a {} version JSON Horizon knows (accepted: {})".format(
                    entry["url"], actual, version_id, ", ".join(sorted(accepted_json))
                )
            )
        version_path.write_bytes(body)
    print(
        "  {:<12} {:>8} MB, sha1 {} in {:.1f} s".format(
            "version.json",
            mb(version_path),
            sha1_of(version_path),
            time.monotonic() - before,
        )
    )
    with version_path.open(encoding="utf-8") as stream:
        version = json.load(stream)
    if version.get("id") != version_id:
        raise SystemExit(
            "version.json is for {}, the lock pins {}".format(version.get("id"), version_id)
        )

    # The two jars, each by the SHA-1 the lock pins and the version JSON repeats.
    total = 0
    for side, pin in (("client", lock["client_sha1"]), ("server", lock["server_sha1"])):
        blob = (version.get("downloads") or {}).get(side) or {}
        url = blob.get("url")
        if not url or blob.get("sha1") != pin:
            raise SystemExit("the version JSON's {} download is not the pinned one".format(side))
        before = time.monotonic()
        fetched = download(url, out / (side + ".jar"), pin)
        total += fetched
        print(
            "  {:<12} {:>8} MB, sha1 {}{} in {:.1f} s".format(
                side + ".jar",
                mb(out / (side + ".jar")),
                pin,
                "" if fetched else " (kept)",
                time.monotonic() - before,
            )
        )

    # The libraries the official launcher would install on this machine.
    total += run(library_jobs(version, out), "libraries")

    # The asset index, then its objects.
    index_info = version.get("assetIndex") or {}
    index_id = index_info.get("id")
    index_url = index_info.get("url")
    index_sha1 = index_info.get("sha1")
    if not (index_id and index_url and is_hex(index_sha1, 40)):
        raise SystemExit("the version JSON has no usable assetIndex")
    index_path = out / "assets" / "indexes" / (index_id + ".json")
    accepted = {index_sha1} | accepted_index
    kept = index_path.is_file() and sha1_of(index_path) in accepted
    actual_index = download_any(index_url, index_path, accepted)
    if not kept:
        total += index_path.stat().st_size
    print(
        "  {:<12} {:>8} MB, sha1 {}{}".format(
            "asset index",
            mb(index_path),
            actual_index,
            " (kept)" if kept else "",
        )
    )
    with index_path.open(encoding="utf-8") as stream:
        index = json.load(stream)
    jobs, objects = object_jobs(index, out)
    total += run(jobs, "objects")
    print(
        "  {:<12} {:>5} names, {:>8} MB of objects".format(
            "asset names",
            len(objects),
            round(sum(o.get("size", 0) for o in objects.values()) / 1e6, 1),
        )
    )
    print(
        "fetch_minecraft: {:.1f} MB fetched in {:.1f} s; {} is ready for horizon-prepare".format(
            total / 1e6, time.monotonic() - started, out
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
