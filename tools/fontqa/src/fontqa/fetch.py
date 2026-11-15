"""Download the fonts named in the manifest into a cache directory."""

from __future__ import annotations

import fnmatch
import hashlib
import io
import json
import shutil
import tarfile
import tempfile
import tomllib
import urllib.parse
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path

USER_AGENT = "kannadi-fontqa/0.1"


@dataclass
class Entry:
    id: str
    name: str
    source: str
    license: str
    files: list[str]
    archive: str | None = None
    local: bool = False


def load_manifest(path: Path) -> list[Entry]:
    with open(path, "rb") as f:
        data = tomllib.load(f)
    entries = [Entry(**e) for e in data["font"]]
    ids = [e.id for e in entries]
    if len(ids) != len(set(ids)):
        raise ValueError(f"duplicate font ids in {path}")
    return entries


def load_lock(path: Path) -> dict[str, dict[str, str]]:
    """sha256 of every fetched file, by font id and file name."""
    return json.loads(path.read_text()) if path.exists() else {}


def write_lock(path: Path, files: dict[str, list[Path]], known: set[str] | None = None) -> None:
    """Record the files' hashes, keeping the entries of known fonts not fetched this time."""
    lock = {k: v for k, v in load_lock(path).items() if known is None or k in known}
    for fid, paths in files.items():
        lock[fid] = {p.name: _digest(p.read_bytes()) for p in sorted(paths)}
    path.write_text(json.dumps(lock, indent=1, sort_keys=True) + "\n")


def _digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _download(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as resp:
        return resp.read()


def _ar_members(data: bytes) -> dict[str, bytes]:
    """The members of a Unix ar archive, the container of a .deb package."""
    if not data.startswith(b"!<arch>\n"):
        raise ValueError("not an ar archive")
    out, pos = {}, 8
    while pos + 60 <= len(data):
        header = data[pos : pos + 60]
        name = header[:16].decode().strip().rstrip("/")
        size = int(header[48:58].decode().strip())
        out[name] = data[pos + 60 : pos + 60 + size]
        pos += 60 + size + (size % 2)
    return out


def _archive_members(url: str, data: bytes) -> dict[str, bytes]:
    path = urllib.parse.urlparse(url).path.lower()
    if path.endswith(".zip"):
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            return {n: z.read(n) for n in z.namelist() if not n.endswith("/")}
    if path.endswith(".deb"):
        inner = next(v for k, v in _ar_members(data).items() if k.startswith("data.tar"))
        with tarfile.open(fileobj=io.BytesIO(inner)) as t:
            return {
                m.name.removeprefix("./"): t.extractfile(m).read()
                for m in t.getmembers()
                if m.isfile()
            }
    raise ValueError(f"unsupported archive type: {url}")


def _safe_name(name: str) -> str:
    return Path(urllib.parse.unquote(name)).name


def fetch(entry: Entry, cache: Path, lock: dict[str, dict[str, str]]) -> list[Path]:
    """Return the entry's font files, downloading any that are not cached yet.

    Every file, cached or new, must match its hash in the lock when the lock has one.
    """
    if entry.local:
        return [Path(f) for f in entry.files if Path(f).exists()]
    expected = lock.get(entry.id, {})
    target = cache / entry.id
    if entry.archive:
        if not target.exists():
            _extract(entry, target, expected)
        paths = sorted(p for p in target.iterdir() if p.is_file())
    else:
        paths = []
        for url in entry.files:
            path = target / _safe_name(urllib.parse.urlparse(url).path)
            if not path.exists():
                data = _download(url)
                _check(path.name, data, expected)
                target.mkdir(parents=True, exist_ok=True)
                _write(path, data)
            paths.append(path)
    for path in paths:
        _check(path.name, path.read_bytes(), expected)
    return paths


def _extract(entry: Entry, target: Path, expected: dict[str, str]) -> None:
    """Unpack the chosen archive members into a temporary folder, then move it into place."""
    members = _archive_members(entry.archive, _download(entry.archive))
    chosen = {n: b for n, b in members.items() if any(fnmatch.fnmatch(n, p) for p in entry.files)}
    if not chosen:
        raise ValueError(f"{entry.id}: no archive member matches {entry.files}")
    for name, data in chosen.items():
        _check(_safe_name(name), data, expected)
    target.parent.mkdir(parents=True, exist_ok=True)
    tmp = Path(tempfile.mkdtemp(dir=target.parent, prefix=f".{target.name}-"))
    try:
        for name, data in chosen.items():
            (tmp / _safe_name(name)).write_bytes(data)
        tmp.rename(target)
    except BaseException:
        shutil.rmtree(tmp, ignore_errors=True)
        raise


def _check(name: str, data: bytes, expected: dict[str, str]) -> None:
    digest = _digest(data)
    if name in expected and expected[name] != digest:
        raise ValueError(f"{name}: sha256 {digest} does not match the lock file")


def _write(path: Path, data: bytes) -> None:
    """Write through a temporary file so an interrupted run leaves no partial font."""
    tmp = path.with_name(path.name + ".part")
    tmp.write_bytes(data)
    tmp.replace(path)
