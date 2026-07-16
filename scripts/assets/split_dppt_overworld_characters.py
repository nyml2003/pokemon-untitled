#!/usr/bin/env python3
"""Split the Diamond/Pearl/Platinum overworld character sheet into PNG frames.

The source image is a 33 by 36 cell sheet. Every character occupies a 3 by 4
cell block: three animation frames for down, left, right, and up. The source
uses an opaque white background, which this script converts to transparency.

This is intentionally stdlib-only so it can be run in the same environment as
the other asset maintenance scripts:

    python scripts/assets/split_dppt_overworld_characters.py --apply
"""

from __future__ import annotations

import argparse
import binascii
import hashlib
import json
from pathlib import Path
import struct
import zlib
from collections import Counter


CELL_SIZE = 32
SOURCE = Path(r"C:\Users\nyml\code\assets\行走图\珍珠钻石白金全人物.png")
OUTPUT_PREFIX = "character/dppt/"
DIRECTIONS = ("down", "left", "right", "up")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--apply", action="store_true", help="write frames and update the catalog")
    return parser.parse_args()


def repository_root() -> Path:
    return Path(__file__).resolve().parents[2]


def read_png(path: Path) -> tuple[int, int, bytes]:
    """Read an 8-bit, non-interlaced truecolor PNG into RGB bytes."""
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError(f"not a PNG file: {path}")

    offset = 8
    chunks: dict[bytes, list[bytes]] = {}
    while offset < len(data):
        length = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        chunks.setdefault(kind, []).append(payload)
        offset += 12 + length

    width, height, bit_depth, color_type, compression, filter_method, interlace = struct.unpack(
        ">IIBBBBB", chunks[b"IHDR"][0]
    )
    if (bit_depth, color_type, compression, filter_method, interlace) != (8, 2, 0, 0, 0):
        raise ValueError("source must be a non-interlaced 8-bit RGB PNG")

    encoded = zlib.decompress(b"".join(chunks[b"IDAT"]))
    stride = width * 3
    rows = bytearray(height * stride)
    position = 0
    previous = bytearray(stride)
    for row in range(height):
        filter_type = encoded[position]
        position += 1
        current = bytearray(encoded[position : position + stride])
        position += stride
        unfilter(current, previous, filter_type, 3)
        rows[row * stride : (row + 1) * stride] = current
        previous = current
    return width, height, bytes(rows)


def unfilter(current: bytearray, previous: bytearray, filter_type: int, bpp: int) -> None:
    if filter_type == 0:
        return
    for index in range(len(current)):
        left = current[index - bpp] if index >= bpp else 0
        up = previous[index]
        up_left = previous[index - bpp] if index >= bpp else 0
        if filter_type == 1:
            current[index] = (current[index] + left) & 0xFF
        elif filter_type == 2:
            current[index] = (current[index] + up) & 0xFF
        elif filter_type == 3:
            current[index] = (current[index] + ((left + up) // 2)) & 0xFF
        elif filter_type == 4:
            current[index] = (current[index] + paeth(left, up, up_left)) & 0xFF
        else:
            raise ValueError(f"unsupported PNG filter: {filter_type}")


def paeth(left: int, up: int, up_left: int) -> int:
    estimate = left + up - up_left
    candidates = ((abs(estimate - left), left), (abs(estimate - up), up), (abs(estimate - up_left), up_left))
    return min(candidates, key=lambda candidate: candidate[0])[1]


def crop_rgba(width: int, rgb: bytes, left: int, top: int) -> tuple[bytes, int]:
    """Crop one frame, converting its dominant palette color to transparency."""
    colors = Counter(
        rgb[((top + y) * width + left + x) * 3 : ((top + y) * width + left + x) * 3 + 3]
        for y in range(CELL_SIZE)
        for x in range(min(CELL_SIZE, width - left))
    )
    background, _ = colors.most_common(1)[0]
    frame = bytearray(CELL_SIZE * CELL_SIZE * 4)
    opaque = 0
    for y in range(CELL_SIZE):
        for x in range(CELL_SIZE):
            source_x = left + x
            if source_x >= width:
                continue
            source = ((top + y) * width + source_x) * 3
            target = (y * CELL_SIZE + x) * 4
            red, green, blue = rgb[source : source + 3]
            if bytes((red, green, blue)) != background:
                frame[target : target + 4] = bytes((red, green, blue, 255))
                opaque += 1
    return bytes(frame), opaque


def write_rgba_png(path: Path, rgba: bytes) -> None:
    scanlines = b"".join(
        b"\x00" + rgba[row * CELL_SIZE * 4 : (row + 1) * CELL_SIZE * 4]
        for row in range(CELL_SIZE)
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + png_chunk(b"IHDR", struct.pack(">IIBBBBB", CELL_SIZE, CELL_SIZE, 8, 6, 0, 0, 0))
        + png_chunk(b"IDAT", zlib.compress(scanlines, level=9))
        + png_chunk(b"IEND", b"")
    )


def png_chunk(kind: bytes, payload: bytes) -> bytes:
    return struct.pack(">I", len(payload)) + kind + payload + struct.pack(">I", binascii.crc32(kind + payload) & 0xFFFFFFFF)


def generated_frames(width: int, height: int, rgb: bytes) -> list[tuple[str, bytes]]:
    if height % (CELL_SIZE * len(DIRECTIONS)) != 0:
        raise ValueError(f"unexpected source height: {height}")

    frames: list[tuple[str, bytes]] = []
    groups_per_band = (width + CELL_SIZE - 1) // CELL_SIZE // 3
    character_id = 0
    for band in range(height // (CELL_SIZE * len(DIRECTIONS))):
        for group in range(groups_per_band):
            candidate: list[tuple[str, bytes, int]] = []
            for direction_index, direction in enumerate(DIRECTIONS):
                for frame_index in range(3):
                    frame, opaque = crop_rgba(
                        width,
                        rgb,
                        (group * 3 + frame_index) * CELL_SIZE,
                        (band * len(DIRECTIONS) + direction_index) * CELL_SIZE,
                    )
                    action = "stand" if frame_index == 0 else "walk"
                    action_frame = 0 if frame_index == 0 else frame_index
                    key = f"{OUTPUT_PREFIX}{character_id:03}/{direction}/{action}/{action_frame:02}"
                    candidate.append((key, frame, opaque))

            # Credits and illustrations do not fill a complete 3-by-4 sprite block.
            if min(opaque for _, _, opaque in candidate) < 40:
                continue
            frames.extend((key, frame) for key, frame, _ in candidate)
            character_id += 1
    return frames


def asset_entry(root: Path, key: str) -> dict[str, object]:
    path = root / "source" / f"{key}.png"
    return {
        "key": key,
        "kind": "image",
        "codec": "png",
        "source": path.relative_to(root).as_posix(),
        "byte_length": path.stat().st_size,
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "dimensions": [CELL_SIZE, CELL_SIZE],
    }


def update_catalog(root: Path, keys: list[str]) -> None:
    catalog_path = root / "catalog" / "assets.v1.json"
    catalog = json.loads(catalog_path.read_text(encoding="utf-8"))
    retained = [entry for entry in catalog["assets"] if not entry["key"].startswith(OUTPUT_PREFIX)]
    entries = retained + [asset_entry(root, key) for key in keys]
    entries.sort(key=lambda entry: str(entry["key"]))
    catalog["assets"] = entries
    catalog_path.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    lock_path = root / "catalog" / "assets.v1.lock.json"
    lock = {
        "schema_version": 1,
        "assets": [
            {
                field: entry[field]
                for field in ("key", "source", "byte_length", "sha256", "dimensions")
                if field in entry
            }
            for entry in entries
        ],
    }
    lock_path.write_text(json.dumps(lock, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    args = parse_args()
    if not SOURCE.is_file():
        raise SystemExit(f"source sheet not found: {SOURCE}")
    root = repository_root() / "assets"
    width, height, rgb = read_png(SOURCE)
    frames = generated_frames(width, height, rgb)
    print(f"detected {len(frames) // 12} character sheets and {len(frames)} frames")
    if not args.apply:
        return

    destination = root / "source" / "character" / "dppt"
    if destination.exists():
        missing = [key for key, _ in frames if not (root / "source" / f"{key}.png").is_file()]
        if missing:
            raise SystemExit(f"existing output is incomplete: {root / 'source' / f'{missing[0]}.png'}")
        print("output already exists; rebuilding catalog")
    else:
        for key, frame in frames:
            write_rgba_png(root / "source" / f"{key}.png", frame)
    update_catalog(root, [key for key, _ in frames])


if __name__ == "__main__":
    main()
