#!/usr/bin/env python3
"""Move assets into the normalized source tree and write catalog files."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil

from asset_migration import asset_root, build_plan, file_hash, png_dimensions


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--apply", action="store_true", help="perform the migration")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not args.apply:
        raise SystemExit("refusing to move files without --apply; run plan_asset_migration.py first")
    root = asset_root()
    plan = build_plan()
    destinations = [root / move.target for move in plan]
    conflicts = [path for path in destinations if path.exists()]
    if conflicts:
        raise SystemExit(f"refusing to overwrite destination: {conflicts[0]}")

    for move in plan:
        source = root / move.source
        target = root / move.target
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(source), str(target))

    entries = []
    for move in plan:
        if move.key is None:
            continue
        path = root / move.target
        entry: dict[str, object] = {
            "key": move.key,
            "kind": move.kind,
            "codec": move.codec,
            "source": move.target.as_posix(),
            "byte_length": path.stat().st_size,
            "sha256": file_hash(path),
        }
        if move.codec == "png":
            entry["dimensions"] = list(png_dimensions(path))
        entries.append(entry)
    entries.sort(key=lambda entry: str(entry["key"]))
    catalog = {"schema_version": 1, "assets": entries}
    catalog_dir = root / "catalog"
    catalog_dir.mkdir(exist_ok=True)
    write_json(catalog_dir / "assets.v1.json", catalog)
    write_json(
        catalog_dir / "assets.v1.lock.json",
        {
            "schema_version": 1,
            "assets": [
                {
                    key: entry[key]
                    for key in ("key", "source", "byte_length", "sha256", "dimensions")
                    if key in entry
                }
                for entry in entries
            ],
        },
    )
    remove_empty_legacy_directories(root)


def write_json(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def remove_empty_legacy_directories(root: Path) -> None:
    for path in sorted(root.rglob("*"), key=lambda item: len(item.parts), reverse=True):
        if path.is_dir() and path.name not in {"catalog", "source", "imports"}:
            try:
                path.rmdir()
            except OSError:
                pass


if __name__ == "__main__":
    main()
