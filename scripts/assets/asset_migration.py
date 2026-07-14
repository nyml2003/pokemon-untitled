"""Shared deterministic asset migration helpers."""

from __future__ import annotations

from dataclasses import dataclass
from hashlib import sha256
from pathlib import Path, PurePosixPath
import re
import struct
from typing import Iterable


TYPE_NAMES = {
    0: "normal",
    1: "fighting",
    2: "flying",
    3: "poison",
    4: "ground",
    5: "rock",
    6: "bug",
    7: "ghost",
    8: "steel",
    10: "fire",
    11: "water",
    12: "grass",
    13: "electric",
    14: "psychic",
    15: "ice",
    16: "dragon",
    17: "dark",
}

POKEMON_SPRITE = re.compile(
    r"^(?P<dex>\d{3})_(?P<pose>Front|Back)_(?P<form>\d+)_(?P<color>[A-Z])__frame_(?P<frame>\d+)$"
)
POKEMON_ICON = re.compile(
    r"^(?P<dex>\d{3}|egg)(?:_(?P<form>[a-z0-9-]+))?_(?P<frame>\d+)$"
)
CHARACTER_ACTION = re.compile(
    r"^(?P<direction>down|left|right|up)_(?P<action>stand|walk|run|runn)(?:_(?P<frame>\d+))?$"
)
CHARACTER_SHEET = re.compile(r"^group-(?P<group>\d+)_row-(?P<row>\d+)$")
TILE = re.compile(r"^tile-(?P<tile>\d+)$")


@dataclass(frozen=True)
class AssetMove:
    source: PurePosixPath
    target: PurePosixPath
    key: str | None
    kind: str | None
    codec: str | None


def repository_root() -> Path:
    return Path(__file__).resolve().parents[2]


def asset_root(root: Path | None = None) -> Path:
    return (root or repository_root()) / "assets"


def build_plan(root: Path | None = None) -> list[AssetMove]:
    assets = asset_root(root)
    if not assets.is_dir():
        raise ValueError(f"asset root does not exist: {assets}")
    moves = [classify(path.relative_to(assets)) for path in sorted(assets.rglob("*")) if path.is_file()]
    validate_plan(moves)
    return moves


def classify(source: Path) -> AssetMove:
    parts = source.parts
    if parts[0] == "catalog" or parts[0] in {"source", "imports"}:
        raise ValueError(f"asset tree is already partially migrated: {source.as_posix()}")
    if source == Path("data/current-data-set-v2.json"):
        return source_asset(source, "data/game/current-dataset/v2", "data", "json")
    if parts[0] == "pokeapi-current-data":
        return import_asset(source)
    if parts[0] == "testtest":
        return import_asset(source)
    if parts[:3] == ("maps", "25_47179", "tiles") and source.suffix == ".png":
        match = TILE.fullmatch(source.stem)
        if match is None:
            raise ValueError(f"unknown map tile name: {source.as_posix()}")
        tile = int(match["tile"])
        return source_asset(source, f"map/tile/{tile:04}", "image", "png")
    if source == Path("maps/25_47179/manifest.json"):
        return source_asset(source, "map/tileset/25_47179/manifest", "map-manifest", "json")
    if source == Path("characters/red/actions/manifest.json"):
        return source_asset(source, "character/red/actions/manifest", "character-manifest", "json")
    if parts[:4] == ("characters", "red", "actions", "group-00") and source.suffix == ".png":
        match = CHARACTER_ACTION.fullmatch(source.stem)
        if match is None:
            raise ValueError(f"unknown character action name: {source.as_posix()}")
        action = "run" if match["action"] == "runn" else match["action"]
        frame = int(match["frame"] or 0)
        return source_asset(
            source,
            f"character/red/{match['direction']}/{action}/{frame:02}",
            "image",
            "png",
        )
    if parts[:3] == ("characters", "red", "actions") and source.suffix == ".png":
        match = CHARACTER_SHEET.fullmatch(source.stem)
        if match is None:
            raise ValueError(f"unknown character sheet name: {source.as_posix()}")
        return source_asset(
            source,
            f"character/red/sheet/{int(match['group']):02}/{int(match['row']):02}",
            "image",
            "png",
        )
    if len(parts) == 4 and parts[0] == "pokemons" and parts[1] in {"normal", "shiny"}:
        return pokemon_sprite(source)
    if len(parts) == 3 and parts[:2] == ("pokemons", "icons"):
        if source.name == "index-gen1-5.csv":
            return import_asset(source)
        return pokemon_icon(source)
    if len(parts) == 2 and parts[0] == "type-icons" and source.suffix == ".png":
        index = int(source.stem.removeprefix("icon-"))
        name = TYPE_NAMES.get(index)
        key = f"ui/battle/type/{name}" if name is not None else f"ui/type-icon/index/{index:02}"
        return source_asset(source, key, "image", "png")
    if len(parts) == 2 and parts[0] == "move-category-icons" and source.suffix == ".png":
        return source_asset(source, f"ui/battle/move-category/{source.stem}", "image", "png")
    raise ValueError(f"unclassified asset: {source.as_posix()}")


def source_asset(source: Path, key: str, kind: str, codec: str) -> AssetMove:
    extension = f".{codec}"
    return AssetMove(
        PurePosixPath(source.as_posix()),
        PurePosixPath("source") / PurePosixPath(key + extension),
        key,
        kind,
        codec,
    )


def import_asset(source: Path) -> AssetMove:
    return AssetMove(
        PurePosixPath(source.as_posix()),
        PurePosixPath("imports") / PurePosixPath(source.as_posix()),
        None,
        None,
        None,
    )


def pokemon_sprite(source: Path) -> AssetMove:
    match = POKEMON_SPRITE.fullmatch(source.stem)
    if match is None:
        raise ValueError(f"unknown pokemon sprite name: {source.as_posix()}")
    dex = int(match["dex"])
    form = int(match["form"])
    palette = source.parts[1]
    expected_color = "C" if palette == "normal" else "S"
    if match["color"] != expected_color:
        raise ValueError(f"unexpected pokemon sprite palette: {source.as_posix()}")
    pose = match["pose"].lower()
    frame = int(match["frame"])
    return source_asset(
        source,
        f"pokemon/{dex:04}/form/{form:02}/{palette}/{pose}/{frame:02}",
        "image",
        "png",
    )


def pokemon_icon(source: Path) -> AssetMove:
    match = POKEMON_ICON.fullmatch(source.stem)
    if match is None:
        raise ValueError(f"unknown pokemon icon name: {source.as_posix()}")
    form = match["form"] or "00"
    frame = int(match["frame"])
    dex = match["dex"]
    if dex == "egg":
        key = f"pokemon/egg/form/{form}/icon/{frame:02}"
    else:
        key = f"pokemon/{int(dex):04}/form/{form}/icon/{frame:02}"
    return source_asset(source, key, "image", "png")


def validate_plan(moves: Iterable[AssetMove]) -> None:
    source_seen: set[PurePosixPath] = set()
    target_seen: set[PurePosixPath] = set()
    key_seen: set[str] = set()
    for move in moves:
        if move.source in source_seen:
            raise ValueError(f"duplicate source: {move.source}")
        if move.target in target_seen:
            raise ValueError(f"target collision: {move.target}")
        source_seen.add(move.source)
        target_seen.add(move.target)
        if move.key is not None:
            if move.key in key_seen:
                raise ValueError(f"key collision: {move.key}")
            key_seen.add(move.key)


def file_hash(path: Path) -> str:
    digest = sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_dimensions(path: Path) -> tuple[int, int]:
    with path.open("rb") as handle:
        header = handle.read(24)
    if header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
        raise ValueError(f"invalid PNG header: {path}")
    return struct.unpack(">II", header[16:24])
