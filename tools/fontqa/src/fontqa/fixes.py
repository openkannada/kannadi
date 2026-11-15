"""Engineering fixes for shaping tables. None of them draws new outlines."""

from __future__ import annotations

from fontTools.otlLib.builder import buildLigatureSubstSubtable, buildLookup
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables import otTables as ot

from .kannada import CONSONANTS, VIRAMA

# HarfBuzz uses an AAT morx table in place of GSUB when a font has both.
AAT_TABLES = ("morx", "mort", "feat", "prop")


def drop_aat(font: TTFont) -> list[str]:
    """Remove AAT layout tables so every shaper reads the OpenType ones. Returns their tags."""
    removed = [tag for tag in AAT_TABLES if tag in font]
    for tag in removed:
        del font[tag]
    return removed


def add_new_order_blwf(font: TTFont, script: str = "knd2") -> int:
    """Give the script a blwf feature in the new Indic order, virama then consonant.

    The ligatures come from the font's old-order blwf rules, consonant then virama, including
    those reached through contextual lookups. Returns how many ligatures the new lookup has.
    """
    gsub = font["GSUB"].table
    cmap = font.getBestCmap()
    virama = cmap.get(ord(VIRAMA))
    consonants = {cmap[ord(c)] for c in CONSONANTS if ord(c) in cmap}
    if virama is None:
        return 0
    old = {}
    for index in _feature_lookups(gsub, "blwf"):
        for sub in _ligature_subtables(gsub, index):
            for first, ligs in sub.ligatures.items():
                for lig in ligs:
                    if first in consonants and lig.Component == [virama]:
                        old.setdefault(first, lig.LigGlyph)
    if not old:
        return 0
    lookup = buildLookup([buildLigatureSubstSubtable({(virama, c): g for c, g in old.items()})])
    gsub.LookupList.Lookup.append(lookup)
    gsub.LookupList.LookupCount = len(gsub.LookupList.Lookup)
    _set_feature(gsub, script, "blwf", [len(gsub.LookupList.Lookup) - 1])
    return len(old)


def _feature_lookups(gsub, tag: str) -> set[int]:
    return {i for fr in gsub.FeatureList.FeatureRecord if fr.FeatureTag == tag
            for i in fr.Feature.LookupListIndex}  # fmt: skip


def _ligature_subtables(gsub, index: int, seen: set[int] | None = None):
    """Ligature subtables of a lookup, and of the lookups its contextual rules call."""
    seen = set() if seen is None else seen
    if index in seen:
        return
    seen.add(index)
    lookup = gsub.LookupList.Lookup[index]
    for sub in lookup.SubTable:
        if lookup.LookupType == 7:
            sub = sub.ExtSubTable
        if isinstance(sub, ot.LigatureSubst):
            yield sub
        for record in _subst_records(sub):
            yield from _ligature_subtables(gsub, record.LookupListIndex, seen)


def _subst_records(sub):
    if hasattr(sub, "SubstLookupRecord"):
        return sub.SubstLookupRecord
    out = []
    for attr in ("ChainSubRuleSet", "ChainSubClassSet", "SubRuleSet", "SubClassSet"):
        for rule_set in getattr(sub, attr, None) or []:
            for rule in (
                getattr(rule_set, "ChainSubRule", None)
                or getattr(rule_set, "ChainSubClassRule", None)
                or getattr(rule_set, "SubRule", None)
                or getattr(rule_set, "SubClassRule", None)
                or []
            ):
                out += rule.SubstLookupRecord
    return out


def _set_feature(gsub, script: str, tag: str, lookups: list[int]) -> None:
    """Point every language system of the script at a new feature record, in place of tag's."""
    record = ot.FeatureRecord()
    record.FeatureTag = tag
    record.Feature = ot.Feature()
    record.Feature.FeatureParams = None
    record.Feature.LookupListIndex = lookups
    record.Feature.LookupCount = len(lookups)
    records = gsub.FeatureList.FeatureRecord
    records.append(record)
    new_index = len(records) - 1
    for sr in gsub.ScriptList.ScriptRecord:
        if sr.ScriptTag != script:
            continue
        systems = [sr.Script.DefaultLangSys] + [r.LangSys for r in sr.Script.LangSysRecord]
        for ls in filter(None, systems):
            kept = [i for i in ls.FeatureIndex if records[i].FeatureTag != tag]
            ls.FeatureIndex = kept + [new_index]
            ls.FeatureCount = len(ls.FeatureIndex)
    _sort_features(gsub)


def _sort_features(gsub) -> None:
    """OpenType wants feature records sorted by tag; renumber every reference to them."""
    if getattr(gsub, "FeatureVariations", None):
        raise ValueError("fonts with FeatureVariations are not supported")
    records = gsub.FeatureList.FeatureRecord
    order = sorted(range(len(records)), key=lambda i: records[i].FeatureTag)
    remap = {old: new for new, old in enumerate(order)}
    gsub.FeatureList.FeatureRecord = [records[i] for i in order]
    gsub.FeatureList.FeatureCount = len(records)
    for sr in gsub.ScriptList.ScriptRecord:
        systems = [sr.Script.DefaultLangSys] + [r.LangSys for r in sr.Script.LangSysRecord]
        for ls in filter(None, systems):
            ls.FeatureIndex = sorted(remap[i] for i in ls.FeatureIndex)
            if ls.ReqFeatureIndex != 0xFFFF:
                ls.ReqFeatureIndex = remap[ls.ReqFeatureIndex]
