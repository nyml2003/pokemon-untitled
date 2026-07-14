#!/usr/bin/env python3
"""Build the Hoenn Pokedex from local data and canonical front sprites.

The front sprite is the identity source.  Legacy icon files are deliberately
excluded from the generated index because some of their filenames refer to a
different creature than the corresponding front sprite.
"""

from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
import json
from pathlib import Path
import struct
from typing import Any


FIRST_DEX = 252
LAST_DEX = 385
MAGIC = b"PKDX"
VERSION = 1


@dataclass(frozen=True)
class Entry:
    dex: int
    form_id: int
    localized: str
    english: str
    types: tuple[tuple[int, str], ...]
    stats: tuple[int, int, int, int, int, int]
    front_key: str


def root() -> Path:
    return Path(__file__).resolve().parents[1]


def sized(value: str) -> bytes:
    data = value.encode("utf-8")
    if len(data) > 255:
        raise ValueError(f"text is too long: {value!r}")
    return bytes([len(data)]) + data


def write_text_atomic(path: Path, content: str) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(content, encoding="utf-8")
    temporary.replace(path)


def load_entries(repository: Path) -> list[Entry]:
    assets = repository / "assets"
    dataset = json.loads(
        (assets / "source/data/game/current-dataset/v2.json").read_text(encoding="utf-8")
    )
    type_names = {item["id"]: item["display_name"]["localized"] for item in dataset["types"]}
    records: dict[int, dict[str, Any]] = {}
    for item in dataset["pokemon"]:
        species_id = item["species_id"]
        if FIRST_DEX <= species_id <= LAST_DEX and item["is_default"]:
            records[species_id] = item
    entries: list[Entry] = []
    for dex in range(FIRST_DEX, LAST_DEX + 1):
        item = records.get(dex)
        if item is None:
            raise ValueError(f"local dataset has no default form for #{dex:03}")
        front = assets / f"source/pokemon/{dex:04}/form/00/normal/front/00.png"
        if not front.is_file():
            raise ValueError(f"missing canonical normal front: {front.relative_to(repository)}")
        type_ids = tuple(item["types"])
        entries.append(
            Entry(
                dex=dex,
                form_id=item["id"],
                localized=item["display_name"]["localized"],
                english=item["display_name"]["english"],
                types=tuple((type_id, type_names[type_id]) for type_id in type_ids),
                stats=tuple(item["base_stats"][name] for name in (
                    "hp", "attack", "defense", "special_attack", "special_defense", "speed"
                )),
                front_key=f"pokemon/{dex:04}/form/00/normal/front/00",
            )
        )
    return entries


def encode(entries: list[Entry]) -> bytes:
    output = bytearray(MAGIC + struct.pack("<HH", VERSION, len(entries)))
    for entry in entries:
        output.extend(struct.pack("<HI6H", entry.dex, entry.form_id, *entry.stats))
        output.append(len(entry.types))
        for type_id, type_name in entry.types:
            output.extend(struct.pack("<H", type_id))
            output.extend(sized(type_name))
        output.extend(sized(entry.localized))
        output.extend(sized(entry.english))
        output.extend(sized(entry.front_key))
    return bytes(output)


def catalog_entry(assets: Path, path: Path) -> dict[str, object]:
    relative = path.relative_to(assets).as_posix()
    entry: dict[str, object] = {
        "key": relative.removeprefix("source/").removesuffix(path.suffix),
        "kind": "data",
        "codec": "bin",
        "source": relative,
        "byte_length": path.stat().st_size,
        "sha256": sha256(path.read_bytes()).hexdigest(),
    }
    return entry


def update_catalog(repository: Path, binary: Path, report: Path) -> None:
    assets = repository / "assets"
    catalog_path = assets / "catalog/assets.v1.json"
    lock_path = assets / "catalog/assets.v1.lock.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    entries = [
        item for item in catalog["assets"]
        if not item["source"].startswith("source/pokemon/")
        and item["source"] not in {
            binary.relative_to(assets).as_posix(),
            report.relative_to(assets).as_posix(),
        }
    ]
    for path in sorted((assets / "source/pokemon").rglob("*.png")):
        source = path.relative_to(assets).as_posix()
        item = catalog_entry(assets, path)
        item["kind"] = "image"
        item["codec"] = "png"
        with path.open("rb") as handle:
            header = handle.read(24)
        item["dimensions"] = list(struct.unpack(">II", header[16:24]))
        entries.append(item)
    entries.append(catalog_entry(assets, binary))
    report_entry = catalog_entry(assets, report)
    report_entry["key"] = "data/game/pokedex/hoenn.v1.report"
    report_entry["codec"] = "json"
    entries.append(report_entry)
    entries.sort(key=lambda item: item["key"])
    catalog["assets"] = entries
    write_text_atomic(catalog_path, json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
    lock = {"schema_version": 1, "assets": [
        {key: item[key] for key in ("key", "source", "byte_length", "sha256", "dimensions") if key in item}
        for item in entries
    ]}
    write_text_atomic(lock_path, json.dumps(lock, ensure_ascii=False, indent=2) + "\n")


def write_report(repository: Path, entries: list[Entry]) -> Path:
    report = {
        "schema_version": 1,
        "dex_range": [FIRST_DEX, LAST_DEX],
        "count": len(entries),
        "identity_source": "normal/front/00",
        "entries": [
            {"dex": entry.dex, "name": entry.localized, "front": entry.front_key}
            for entry in entries
        ],
    }
    path = repository / "assets/source/data/game/pokedex/hoenn.v1.json"
    path.parent.mkdir(parents=True, exist_ok=True)
    write_text_atomic(path, json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return path


def main() -> None:
    repository = root()
    entries = load_entries(repository)
    binary = repository / "assets/source/data/game/pokedex/hoenn.v1.bin"
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_bytes(encode(entries))
    report = write_report(repository, entries)
    update_catalog(repository, binary, report)
    print(f"wrote {binary.relative_to(repository)} ({len(entries)} entries)")


if __name__ == "__main__":
    main()
