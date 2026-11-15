import io
import json
import tarfile
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


def _zip(**files: bytes) -> bytes:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w") as z:
        for name, data in files.items():
            z.writestr(name, data)
    return buf.getvalue()


def test_a_bad_archive_member_leaves_nothing_cached(tmp_path, monkeypatch):
    monkeypatch.setattr(fetch, "_download", lambda url: _zip(**{"A.ttf": b"a", "B.ttf": b"b"}))
    entry = fetch.Entry("x", "X", "src", "OFL-1.1", ["*.ttf"], archive="https://h/x.zip")
    lock = {"x": {"B.ttf": "0" * 64}}
    with pytest.raises(ValueError, match="B.ttf"):
        fetch.fetch(entry, tmp_path, lock)
    assert not (tmp_path / "x").exists()


def test_a_cached_file_is_checked_against_the_lock(tmp_path):
    (tmp_path / "x").mkdir()
    (tmp_path / "x" / "F.ttf").write_bytes(b"changed")
    entry = fetch.Entry("x", "X", "src", "OFL-1.1", ["https://h/F.ttf"])
    with pytest.raises(ValueError, match="does not match"):
        fetch.fetch(entry, tmp_path, {"x": {"F.ttf": "0" * 64}})


def test_writing_the_lock_keeps_other_fonts(tmp_path):
    lock = tmp_path / "lock.json"
    lock.write_text(json.dumps({"other": {"O.ttf": "1" * 64}}))
    f = tmp_path / "F.ttf"
    f.write_bytes(b"font")
    fetch.write_lock(lock, {"x": [f]})
    assert set(fetch.load_lock(lock)) == {"other", "x"}


def test_deb_members_come_from_the_data_tarball():
    tar = io.BytesIO()
    with tarfile.open(fileobj=tar, mode="w:gz") as t:
        info = tarfile.TarInfo("./usr/share/fonts/F.ttf")
        info.size = 4
        t.addfile(info, io.BytesIO(b"font"))
    deb = _ar({"debian-binary": b"2.0\n", "data.tar.gz": tar.getvalue()})
    assert fetch._archive_members("https://h/f.deb", deb) == {"usr/share/fonts/F.ttf": b"font"}


def test_writing_the_lock_drops_fonts_no_longer_in_the_manifest(tmp_path):
    lock = tmp_path / "lock.json"
    lock.write_text(json.dumps({"gone": {"G.ttf": "1" * 64}}))
    f = tmp_path / "F.ttf"
    f.write_bytes(b"font")
    fetch.write_lock(lock, {"x": [f]}, known={"x"})
    assert set(fetch.load_lock(lock)) == {"x"}


@pytest.mark.parametrize("cached", [[], ["A.ttf"]], ids=["empty", "partial"])
def test_an_incomplete_archive_cache_is_unpacked_again(tmp_path, monkeypatch, cached):
    data = {"A.ttf": b"a", "B.ttf": b"b"}
    monkeypatch.setattr(fetch, "_download", lambda url: _zip(**data))
    (tmp_path / "x").mkdir()
    for name in cached:
        (tmp_path / "x" / name).write_bytes(data[name])
    entry = fetch.Entry("x", "X", "src", "OFL-1.1", ["*.ttf"], archive="https://h/x.zip")
    lock = {"x": {n: fetch._digest(b) for n, b in data.items()}}
    assert [p.name for p in fetch.fetch(entry, tmp_path, lock)] == ["A.ttf", "B.ttf"]
