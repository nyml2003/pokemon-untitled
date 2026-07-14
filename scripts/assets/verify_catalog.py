#!/usr/bin/env python3
"""Verify the normalized asset tree and its generated catalog using only stdlib."""

from __future__ import annotations

import json
from pathlib import Path
import re

from asset_migration import asset_root, file_hash, png_dimensions


KEY = re.compile(r"^[a-z0-9][a-z0-9._-]*(?:/[a-z0-9][a-z0-9._-]*)*$")


def main() -> None:
    root = asset_root()
    catalog_path = root / "catalog" / "assets.v1.json"
    document = json.loads(catalog_path.read_text(encoding="utf-8"))
    if document.get("schema_version") != 1:
        raise SystemExit("unsupported catalog schema")
    entries = document.get("assets")
    if not isinstance(entries, list) or not entries:
        raise SystemExit("catalog has no assets")
    keys: set[str] = set()
    sources: set[str] = set()
    for entry in entries:
        verify_entry(root, entry, keys, sources)
    actual_sources = {
        path.relative_to(root).as_posix()
        for path in (root / "source").rglob("*")
        if path.is_file()
    }
    if actual_sources != sources:
        missing = sorted(actual_sources - sources)
        extra = sorted(sources - actual_sources)
        raise SystemExit(f"catalog/source mismatch: missing={missing[:1]} extra={extra[:1]}")
    for legacy in ("characters", "data", "maps", "move-category-icons", "pokemons", "testtest", "type-icons"):
        if (root / legacy).exists():
            raise SystemExit(f"legacy asset directory remains: {legacy}")


def verify_entry(root: Path, entry: object, keys: set[str], sources: set[str]) -> None:
    if not isinstance(entry, dict):
        raise SystemExit("catalog entry is not an object")
    key = entry.get("key")
    source = entry.get("source")
    if not isinstance(key, str) or KEY.fullmatch(key) is None:
        raise SystemExit(f"invalid asset key: {key!r}")
    if key in keys:
        raise SystemExit(f"duplicate asset key: {key}")
    if not isinstance(source, str) or not source.startswith("source/"):
        raise SystemExit(f"invalid asset source for {key}: {source!r}")
    if source in sources:
        raise SystemExit(f"duplicate asset source: {source}")
    path = root / source
    if not path.is_file():
        raise SystemExit(f"missing asset source: {source}")
    if entry.get("byte_length") != path.stat().st_size:
        raise SystemExit(f"length mismatch: {key}")
    if entry.get("sha256") != file_hash(path):
        raise SystemExit(f"hash mismatch: {key}")
    if entry.get("codec") == "png" and entry.get("dimensions") != list(png_dimensions(path)):
        raise SystemExit(f"PNG dimensions mismatch: {key}")
    keys.add(key)
    sources.add(source)


if __name__ == "__main__":
    main()
