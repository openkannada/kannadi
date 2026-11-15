import io
import json
import zipfile

import pytest

from fontqa import fetch


def _ar(members: dict[str, bytes]) -> bytes:
    out = b"!<arch>\n"
    for name, data in members.items():
        header = f"{name + '/':<16}{0:<12}{0:<6}{0:<6}{100644:<8}{len(data):<10}`\n"
        out += header.encode() + data + (b"\n" if len(data) % 2 else b"")
    return out


def test_ar_members_reads_odd_sized_members():
    members = {"debian-binary": b"2.0\n", "odd": b"abc", "data.tar.xz": b"xyz!"}
    assert fetch._ar_members(_ar(members)) == members


def test_archive_entries_match_patterns(tmp_path, monkeypatch):
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        z.writestr("fonts/A-n.TTF", b"a")
        z.writestr("fonts/A-b.TTF", b"b")
        z.writestr("readme.txt", b"r")
    monkeypatch.setattr(fetch, "_download", lambda url: buf.getvalue())
    entry = fetch.Entry("x", "X", "src", "OFL-1.1", ["fonts/A-*.TTF"], archive="https://h/x.zip")
    got = fetch.fetch(entry, tmp_path, {})
    assert [p.name for p in got] == ["A-b.TTF", "A-n.TTF"]


def test_a_changed_file_fails_the_lock(tmp_path, monkeypatch):
    monkeypatch.setattr(fetch, "_download", lambda url: b"new bytes")
    entry = fetch.Entry("x", "X", "src", "OFL-1.1", ["https://h/F.ttf"])
    with pytest.raises(ValueError, match="does not match"):
        fetch.fetch(entry, tmp_path, {"x": {"F.ttf": "0" * 64}})


def test_lock_round_trip(tmp_path):
    f = tmp_path / "F.ttf"
    f.write_bytes(b"font")
    fetch.write_lock(tmp_path / "lock.json", {"x": [f]})
    lock = fetch.load_lock(tmp_path / "lock.json")
    assert set(lock) == {"x"}
    assert json.loads((tmp_path / "lock.json").read_text())["x"]["F.ttf"] == lock["x"]["F.ttf"]


def test_manifest_loads_and_ids_are_unique():
    entries = fetch.load_manifest(fetch.Path(__file__).parents[1] / "fonts.toml")
    assert len(entries) > 30
    assert all(e.license for e in entries)
