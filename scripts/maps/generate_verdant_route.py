#!/usr/bin/env python3
"""Generate the Verdant Route's editable connected map projects.

The maps are deliberately built from the checked-in map materials rather than
from a hidden raster export. Re-running this script is deterministic.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path


WIDTH = 72
HEIGHT = 56
ROOT = Path(__file__).resolve().parents[2]
SOURCE_MAP = ROOT / "maps" / "demo-map.json"
OUTPUT = ROOT / "maps" / "verdant-route"
WORLD_MANIFEST = OUTPUT / "world.json"

GRASS = "forest-meadow"
PATH = "clearing-sand-path"
PLANT = "clearing-plant"
FLOWER = "clearing-flower"
SIGN = "clearing-sign"
LOG_LEFT = "clearing-log-left"
LOG_RIGHT = "clearing-log-right"
UNDERBRUSH_LEFT = "forest-underbrush-left-0102-0262"
UNDERBRUSH_RIGHT = "forest-underbrush-right-0102-0263"
UNDERSTORY = "verdant-understory"
WATER = "verdant-water"
WATER_LILY = "verdant-water-lily"

EXTRA_MATERIALS = [
    {"id": UNDERBRUSH_LEFT, "layers": ["tile-0102", "tile-0262"]},
    {"id": UNDERBRUSH_RIGHT, "layers": ["tile-0102", "tile-0263"]},
    {"id": UNDERSTORY, "layers": ["tile-0251"]},
    {"id": WATER, "layers": ["tile-0267"]},
    {"id": WATER_LILY, "layers": ["tile-0267", "tile-0268"]},
]

CENTRAL_ACTORS = [
    {
        "id": "forest-guide",
        "position": [38, 27],
        "facing": "left",
        "appearance": "dppt/000",
    },
    {
        "id": "forest-scout",
        "position": [42, 27],
        "facing": "down",
        "appearance": "dppt/001",
    },
    {
        "id": "forest-ranger",
        "position": [35, 32],
        "facing": "left",
        "appearance": "dppt/002",
    },
    {
        "id": "forest-collector",
        "position": [29, 27],
        "facing": "right",
        "appearance": "dppt/003",
    },
]

TREE_3X3 = [
    ["forest-tree-003-r0c0", "forest-tree-003-r0c1", "forest-tree-003-r0c2"],
    ["forest-tree-003-r1c0", "forest-tree-003-r1c1", "forest-tree-003-r1c2"],
    ["forest-tree-003-r2c0", "forest-tree-003-r2c1", "forest-tree-003-r2c2"],
]
TREE_2X3 = [
    ["forest-tree-001-r0c0", "forest-tree-001-r0c1"],
    ["forest-tree-001-r1c0", "forest-tree-001-r1c1"],
    ["forest-tree-001-r2c0", "forest-tree-001-r2c1"],
]
TALL_GRASS = [
    [
        "forest-tall-grass-015-r0c0",
        "forest-tall-grass-015-r0c1",
        "forest-tall-grass-015-r0c2",
        "forest-tall-grass-015-r0c3",
    ],
    [
        "forest-tall-grass-015-r1c0",
        "forest-tall-grass-015-r1c1",
        "forest-tall-grass-015-r1c2",
        "forest-tall-grass-015-r1c3",
    ],
    [
        "forest-tall-grass-015-r2c0",
        "forest-tall-grass-015-r2c1",
        "forest-tall-grass-015-r2c2",
        "forest-tall-grass-015-r2c3",
    ],
    [
        "forest-tall-grass-015-r3c0",
        "forest-tall-grass-015-r3c1",
        "forest-tall-grass-015-r3c2",
        "forest-tall-grass-015-r3c3",
    ],
    [
        "forest-tall-grass-015-r4c0",
        "forest-tall-grass-015-r4c1",
        "forest-tall-grass-015-r4c2",
        "forest-tall-grass-015-r4c3",
    ],
]
UNDERBRUSH = [["tile-0262", "tile-0263"]]


@dataclass(frozen=True)
class MapSpec:
    slug: str
    coord: tuple[int, int]
    anchor: tuple[int, int]
    ports: tuple[tuple[str, int], ...]
    tree_centers: tuple[tuple[int, int], ...]
    tall_grass: tuple[tuple[int, int], ...]
    decorations: tuple[tuple[str, int, int], ...]


SPECS = (
    MapSpec(
        "northern-thicket",
        (0, 0),
        (38, 29),
        (("east", 19), ("south", 24)),
        ((10, 11), (23, 17), (54, 10), (61, 20), (15, 41), (55, 43)),
        ((25, 35), (47, 31)),
        ((FLOWER, 18, 30), (PLANT, 50, 38), (LOG_LEFT, 59, 34)),
    ),
    MapSpec(
        "moss-pass",
        (1, 0),
        (31, 31),
        (("west", 19), ("east", 32), ("south", 44)),
        ((10, 12), (25, 9), (57, 11), (64, 38), (12, 43), (51, 42)),
        ((18, 32), (44, 35)),
        ((PLANT, 30, 19), (FLOWER, 54, 31), (LOG_RIGHT, 17, 25)),
    ),
    MapSpec(
        "stoneleaf-rise",
        (2, 0),
        (34, 26),
        (("west", 32), ("south", 27)),
        ((12, 9), (28, 16), (52, 13), (62, 31), (15, 41), (49, 43)),
        ((22, 31), (42, 37)),
        ((FLOWER, 33, 22), (PLANT, 57, 39), (SIGN, 48, 26)),
    ),
    MapSpec(
        "western-meadow",
        (0, 1),
        (37, 27),
        (("north", 24), ("east", 26), ("south", 42)),
        ((11, 10), (24, 16), (57, 13), (63, 34), (13, 43), (53, 44)),
        ((24, 30), (43, 38)),
        ((PLANT, 18, 24), (FLOWER, 48, 17), (LOG_LEFT, 31, 43)),
    ),
    MapSpec(
        "wayfarer-crossroads",
        (1, 1),
        (35, 27),
        (("north", 44), ("east", 22), ("south", 32), ("west", 26)),
        ((10, 11), (25, 10), (57, 11), (63, 39), (12, 43), (52, 43)),
        ((19, 32), (47, 33)),
        ((SIGN, 42, 24), (FLOWER, 30, 37), (LOG_RIGHT, 56, 29)),
    ),
    MapSpec(
        "sunlit-clearing",
        (2, 1),
        (39, 30),
        (("north", 27), ("east", 29), ("south", 50), ("west", 22)),
        ((10, 12), (24, 12), (59, 9), (64, 35), (13, 43), (51, 43)),
        ((18, 35), (46, 36)),
        ((FLOWER, 35, 19), (PLANT, 51, 25), (LOG_LEFT, 25, 42)),
    ),
    MapSpec(
        "old-east-road",
        (3, 1),
        (27, 31),
        (("west", 29),),
        ((11, 9), (28, 12), (54, 11), (63, 25), (56, 42), (17, 43)),
        ((22, 35), (42, 31)),
        ((SIGN, 58, 29), (PLANT, 37, 20), (LOG_RIGHT, 47, 40)),
    ),
    MapSpec(
        "southern-field",
        (0, 2),
        (40, 26),
        (("north", 42), ("east", 31)),
        ((11, 11), (26, 10), (59, 14), (64, 37), (15, 43), (49, 43)),
        ((22, 29), (43, 35)),
        ((FLOWER, 30, 18), (PLANT, 57, 32), (LOG_LEFT, 33, 45)),
    ),
    MapSpec(
        "fern-hollow",
        (1, 2),
        (32, 29),
        (("north", 32), ("east", 19), ("west", 31)),
        ((11, 10), (25, 14), (57, 12), (63, 37), (13, 43), (52, 43)),
        ((19, 33), (47, 30)),
        ((PLANT, 27, 21), (FLOWER, 55, 36), (LOG_RIGHT, 17, 40)),
    ),
    MapSpec(
        "quiet-grove",
        (2, 2),
        (36, 27),
        (("north", 50), ("west", 19)),
        ((12, 10), (26, 11), (56, 13), (64, 32), (14, 42), (52, 44)),
        ((23, 35), (43, 29)),
        ((FLOWER, 35, 18), (PLANT, 49, 38), (LOG_LEFT, 27, 43)),
    ),
)

PONDS = {
    "western-meadow": ((48, 33, 11, 7),),
    "sunlit-clearing": ((15, 32, 10, 6),),
    "fern-hollow": ((42, 35, 12, 8), (25, 18, 7, 5)),
    "quiet-grove": ((15, 30, 14, 10),),
}

UNDERSTORY_PATCHES = {
    "northern-thicket": ((29, 10, 10, 6), (30, 39, 12, 7)),
    "moss-pass": ((23, 10, 13, 9), (26, 40, 14, 7)),
    "stoneleaf-rise": ((30, 8, 12, 7), (30, 39, 11, 8)),
    "western-meadow": ((29, 10, 13, 6),),
    "wayfarer-crossroads": ((27, 9, 13, 6), (25, 38, 16, 7)),
    "sunlit-clearing": ((27, 10, 15, 7),),
    "old-east-road": ((29, 9, 13, 10), (29, 39, 13, 8)),
    "southern-field": ((28, 9, 14, 8), (28, 39, 13, 7)),
    "fern-hollow": ((27, 8, 13, 8),),
    "quiet-grove": ((29, 9, 13, 9),),
}

TALL_GRASS_FIELDS = {
    "northern-thicket": ((25, 35), (47, 31)),
    "moss-pass": ((18, 32), (44, 35), (33, 41)),
    "stoneleaf-rise": ((22, 31),),
    "western-meadow": ((24, 30),),
    "wayfarer-crossroads": ((19, 32),),
    "sunlit-clearing": ((18, 35), (46, 36), (28, 30), (38, 38)),
    "old-east-road": ((22, 35),),
    "southern-field": ((22, 29), (43, 35), (30, 40), (52, 24)),
    "fern-hollow": ((19, 33), (47, 30)),
    "quiet-grove": ((43, 29),),
}

# Large encounter meadows give the individual maps a clear gameplay identity.
# They remain intentionally absent from the crossroads and old road, where
# navigation and sight lines matter more than wild encounters.
TALL_GRASS_MEADOWS = {
    "northern-thicket": ((9, 31, 13, 12), (48, 29, 14, 13)),
    "moss-pass": ((13, 25, 12, 13), (43, 35, 14, 11)),
    "stoneleaf-rise": ((17, 30, 12, 12),),
    "western-meadow": ((29, 22, 15, 12),),
    "sunlit-clearing": ((9, 23, 14, 12), (42, 32, 13, 11)),
    "southern-field": ((13, 22, 15, 13), (45, 27, 14, 12)),
    "fern-hollow": ((12, 27, 12, 13), (47, 24, 12, 12)),
    "quiet-grove": ((39, 24, 13, 12),),
}

# These are additional clustered-canopy origins, not loose decorative trees.
# Together with `tree_centers`, they make a route segment feel wooded without
# blocking the paths that connect the world grid.
FOREST_GROVES = {
    "northern-thicket": ((6, 7), (14, 8), (6, 17), (48, 6), (55, 14)),
    "moss-pass": ((7, 8), (47, 7), (53, 15)),
    "stoneleaf-rise": ((8, 7), (48, 7), (54, 15)),
    "western-meadow": ((7, 8), (54, 8)),
    "wayfarer-crossroads": ((8, 8), (56, 8)),
    "sunlit-clearing": ((53, 7),),
    "old-east-road": ((7, 7), (15, 9), (47, 7), (53, 15), (8, 36)),
    "southern-field": ((7, 8), (54, 10)),
    "fern-hollow": ((7, 7), (53, 8), (7, 39)),
    "quiet-grove": ((8, 7), (16, 9), (47, 8), (54, 17), (9, 37)),
}

# A map can introduce one locally distinctive tree silhouette, but its primary
# vegetation is always the shared overlapping forest canopy below. That keeps
# the route visually coherent without making every map look like a field of
# isolated props.
FEATURE_TREES = {
    "moss-pass": ((55, 37),),
    "stoneleaf-rise": ((58, 38),),
    "western-meadow": ((17, 20),),
    "wayfarer-crossroads": ((57, 39),),
    "sunlit-clearing": ((58, 27),),
    "southern-field": ((57, 32),),
    "fern-hollow": ((56, 18),),
}


def index(x: int, y: int) -> int:
    return y * WIDTH + x


def path_cells(
    anchor: tuple[int, int], ports: tuple[tuple[str, int], ...]
) -> set[tuple[int, int]]:
    """Join each shared-border entrance to a local, deliberately offset trail hub."""
    cells: set[tuple[int, int]] = set()

    def rect(left: int, top: int, right: int, bottom: int) -> None:
        for x in range(max(0, left), min(WIDTH, right)):
            for y in range(max(0, top), min(HEIGHT, bottom)):
                cells.add((x, y))

    def trail(start: tuple[int, int], end: tuple[int, int]) -> None:
        x, y = start
        end_x, end_y = end
        while (x, y) != (end_x, end_y):
            cells.add((x, y))
            cells.add((x + 1, y))
            if x != end_x and (abs(end_x - x) >= abs(end_y - y) or y == end_y):
                x += 1 if end_x > x else -1
            elif y != end_y:
                y += 1 if end_y > y else -1
        cells.add((end_x, end_y))
        cells.add((end_x + 1, end_y))

    anchor_x, anchor_y = anchor
    rect(anchor_x - 1, anchor_y - 1, anchor_x + 3, anchor_y + 2)
    for direction, offset in ports:
        if direction == "north":
            rect(offset, 0, offset + 4, 4)
            trail((offset + 1, 3), (anchor_x, anchor_y))
        elif direction == "south":
            rect(offset, HEIGHT - 4, offset + 4, HEIGHT)
            trail((offset + 1, HEIGHT - 4), (anchor_x, anchor_y))
        elif direction == "west":
            rect(0, offset, 4, offset + 4)
            trail((3, offset + 1), (anchor_x, anchor_y))
        elif direction == "east":
            rect(WIDTH - 4, offset, WIDTH, offset + 4)
            trail((WIDTH - 4, offset + 1), (anchor_x, anchor_y))
        else:
            raise ValueError(f"unknown direction {direction}")
    return cells


def place_material(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    material: str,
    x: int,
    y: int,
    blocked: bool = False,
    encounter: bool = False,
) -> None:
    if 0 <= x < WIDTH and 0 <= y < HEIGHT:
        position = index(x, y)
        cells[position] = {"material": material}
        collision[position] = "blocked" if blocked else "walkable"
        events[position] = "encounter" if encounter else None


def place_underbrush(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    protected: set[tuple[int, int]] | None = None,
) -> bool:
    footprint = ((x, y), (x + 1, y))
    if not all(
        0 <= column < WIDTH
        and 0 <= row < HEIGHT
        and cells[index(column, row)]["material"] == GRASS
        and (protected is None or (column, row) not in protected)
        for column, row in footprint
    ):
        return False
    place_material(cells, collision, events, UNDERBRUSH_LEFT, x, y, blocked=True)
    place_material(cells, collision, events, UNDERBRUSH_RIGHT, x + 1, y, blocked=True)
    return True


def can_place(
    x: int,
    y: int,
    footprint: tuple[int, int],
    protected: set[tuple[int, int]],
) -> bool:
    width, height = footprint
    return all(
        3 <= column < WIDTH - 3
        and 3 <= row < HEIGHT - 3
        and (column, row) not in protected
        for row in range(y, y + height)
        for column in range(x, x + width)
    )


def place_tree(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    protected: set[tuple[int, int]],
    template: list[list[str]] = TREE_3X3,
) -> bool:
    height = len(template)
    width = len(template[0])
    if not can_place(x, y, (width, height), protected) or any(
        cells[index(column, row)]["material"] != GRASS
        for row in range(y, y + height)
        for column in range(x, x + width)
    ):
        return False
    for row, materials in enumerate(template):
        for column, material in enumerate(materials):
            place_material(cells, collision, events, material, x + column, y + row, blocked=True)
    return True


def place_tall_grass(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    protected: set[tuple[int, int]],
) -> bool:
    # Authoring coordinates describe a desired meadow. When a local trail has
    # moved into it, retain the encounter patch by shifting it inside the same
    # nearby field instead of silently dropping gameplay content.
    candidates = sorted(
        (
            (candidate_x, candidate_y)
            for candidate_y in range(2, HEIGHT - 6)
            for candidate_x in range(2, WIDTH - 5)
        ),
        key=lambda candidate: abs(candidate[0] - x) + abs(candidate[1] - y),
    )
    for origin_x, origin_y in candidates:
        footprint = [
            (column, row)
            for row in range(origin_y, origin_y + 5)
            for column in range(origin_x, origin_x + 4)
        ]
        if not can_place(origin_x, origin_y, (4, 5), protected) or any(
            cells[index(column, row)]["material"] != GRASS
            for column, row in footprint
        ):
            continue
        for row, materials in enumerate(TALL_GRASS):
            for column, material in enumerate(materials):
                place_material(
                    cells,
                    collision,
                    events,
                    material,
                    origin_x + column,
                    origin_y + row,
                    encounter=True,
                )
        return True
    return False


def place_tall_grass_meadow(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    width: int,
    height: int,
    protected: set[tuple[int, int]],
) -> None:
    """Fill an encounter meadow with whole 4x5 grass objects only."""
    for origin_y in range(y, y + height - len(TALL_GRASS) + 1, len(TALL_GRASS)):
        for origin_x in range(x, x + width - len(TALL_GRASS[0]) + 1, len(TALL_GRASS[0])):
            footprint = [
                (column, row)
                for row in range(origin_y, origin_y + len(TALL_GRASS))
                for column in range(origin_x, origin_x + len(TALL_GRASS[0]))
            ]
            if not all(
                ((column - x + 0.5) / width - 0.5) ** 2 * 4
                + ((row - y + 0.5) / height - 0.5) ** 2 * 4
                <= 1
                and (column, row) not in protected
                and cells[index(column, row)]["material"] == GRASS
                for column, row in footprint
            ):
                continue
            for row, materials in enumerate(TALL_GRASS):
                for column, material in enumerate(materials):
                    place_material(
                        cells,
                        collision,
                        events,
                        material,
                        origin_x + column,
                        origin_y + row,
                        encounter=True,
                    )


def place_ground_patch(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    width: int,
    height: int,
    protected: set[tuple[int, int]],
) -> None:
    for row in range(y, y + height):
        for column in range(x, x + width):
            normalized_x = (column - x + 0.5) / width - 0.5
            normalized_y = (row - y + 0.5) / height - 0.5
            if (
                normalized_x * normalized_x * 4 + normalized_y * normalized_y * 4 <= 1
                and (column + row * 3) % 7 != 0
                and (column, row) not in protected
                and cells[index(column, row)]["material"] == GRASS
            ):
                place_material(cells, collision, events, UNDERSTORY, column, row)


def place_pond(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    width: int,
    height: int,
    protected: set[tuple[int, int]],
) -> None:
    for row in range(y, y + height):
        for column in range(x, x + width):
            normalized_x = (column - x + 0.5) / width - 0.5
            normalized_y = (row - y + 0.5) / height - 0.5
            if (
                normalized_x * normalized_x * 4 + normalized_y * normalized_y * 4 > 1
                or (column, row) in protected
                or cells[index(column, row)]["material"] != GRASS
            ):
                continue
            material = WATER_LILY if (column * 3 + row * 5) % 11 == 0 else WATER
            place_material(cells, collision, events, material, column, row, blocked=True)


def place_tree_grove(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    x: int,
    y: int,
    protected: set[tuple[int, int]],
) -> None:
    """Place touching whole trees, never partial or overlapping crowns."""
    for offset_x, offset_y in ((0, 0), (3, 0), (6, 0), (0, 3), (3, 3), (6, 3), (0, 6), (3, 6)):
        place_tree(cells, collision, events, x + offset_x, y + offset_y, protected)


def place_outer_tree_wall(
    cells: list[dict[str, str]],
    collision: list[str],
    events: list[str | None],
    direction: str,
) -> None:
    """Close a missing world neighbour without cutting a multi-cell tree."""
    if direction in {"north", "south"}:
        y = 0 if direction == "north" else HEIGHT - 3
        for row in range(y, y + 3):
            for x in range(0, WIDTH, 2):
                place_underbrush(cells, collision, events, x, row)
    elif direction in {"west", "east"}:
        pair_x = 0 if direction == "west" else WIDTH - 2
        for y in range(HEIGHT):
            place_underbrush(cells, collision, events, pair_x, y)
    else:
        raise ValueError(f"unknown direction {direction}")


def build_map(
    source: dict[str, object],
    spec: MapSpec,
    occupied: set[tuple[int, int]],
) -> dict[str, object]:
    # The route starts from one semantic base material. It must not inherit the
    # demo map's exploratory overlap composites, because a generated region
    # needs every foreground object to have a complete, checkable footprint.
    cells = [{"material": GRASS} for _ in range(WIDTH * HEIGHT)]
    collision = ["walkable"] * (WIDTH * HEIGHT)
    events: list[str | None] = [None] * (WIDTH * HEIGHT)
    road = path_cells(spec.anchor, spec.ports)
    protected = {
        (x, y)
        for path_x, path_y in road
        for x in range(path_x - 2, path_x + 3)
        for y in range(path_y - 2, path_y + 3)
        if 0 <= x < WIDTH and 0 <= y < HEIGHT
    }

    neighbours = {
        "north": (spec.coord[0], spec.coord[1] - 1),
        "south": (spec.coord[0], spec.coord[1] + 1),
        "west": (spec.coord[0] - 1, spec.coord[1]),
        "east": (spec.coord[0] + 1, spec.coord[1]),
    }
    for direction, coordinate in neighbours.items():
        if coordinate not in occupied:
            place_outer_tree_wall(cells, collision, events, direction)

    for x, y in road:
        place_material(cells, collision, events, PATH, x, y)

    for x, y, width, height in PONDS.get(spec.slug, ()):
        place_pond(cells, collision, events, x, y, width, height, protected)
    for material, x, y in spec.decorations:
        if (x, y) not in protected:
            place_material(
                cells,
                collision,
                events,
                material,
                x,
                y,
                blocked=material in {LOG_LEFT, LOG_RIGHT},
            )

    # Add a little uneven understory without forming a border wall.
    for x, y in ((8, 30), (18, 46), (29, 8), (45, 47), (61, 31)):
        if (x, y) not in protected:
            place_underbrush(cells, collision, events, x, y, protected)

    for x, y, width, height in TALL_GRASS_MEADOWS.get(spec.slug, ()):
        place_tall_grass_meadow(cells, collision, events, x, y, width, height, protected)
    for x, y in TALL_GRASS_FIELDS.get(spec.slug, spec.tall_grass):
        place_tall_grass(cells, collision, events, x, y, protected)

    for x, y in spec.tree_centers:
        place_tree(cells, collision, events, x, y, protected)
    for x, y in FOREST_GROVES.get(spec.slug, ()):
        place_tree_grove(cells, collision, events, x, y, protected)
    for x, y in FEATURE_TREES.get(spec.slug, ()):
        place_tree(cells, collision, events, x, y, protected, TREE_2X3)

    used_materials = {cell["material"] for cell in cells}
    available_materials = [*source["materials"], *EXTRA_MATERIALS]
    project = {
        "format_version": source["format_version"],
        "id": f"verdant-route-{spec.slug}",
        "tile_size": source["tile_size"],
        "width": WIDTH,
        "height": HEIGHT,
        "materials": [
            material
            for material in available_materials
            if material["id"] in used_materials
        ],
        "visual_cells": cells,
        "collision_cells": collision,
        "event_cells": events,
        "player_spawn": [36, 28],
        "actors": CENTRAL_ACTORS if spec.slug == "wayfarer-crossroads" else [],
    }
    return project


def border(project: dict[str, object], direction: str) -> list[tuple[str, str]]:
    cells = project["visual_cells"]
    collision = project["collision_cells"]
    if direction == "east":
        coordinates = [(WIDTH - 1, y) for y in range(HEIGHT)]
    elif direction == "west":
        coordinates = [(0, y) for y in range(HEIGHT)]
    elif direction == "north":
        coordinates = [(x, 0) for x in range(WIDTH)]
    elif direction == "south":
        coordinates = [(x, HEIGHT - 1) for x in range(WIDTH)]
    else:
        raise ValueError(f"unknown direction {direction}")
    return [(cells[index(x, y)]["material"], collision[index(x, y)]) for x, y in coordinates]


def set_boundary_material(project: dict[str, object], x: int, y: int, material: str) -> None:
    project["visual_cells"][index(x, y)] = {"material": material}
    project["collision_cells"][index(x, y)] = "blocked"
    project["event_cells"][index(x, y)] = None


def validate_connections(projects: dict[tuple[int, int], dict[str, object]]) -> None:
    """Shared borders preserve collision and all traversable terrain exactly."""
    def matches(left: tuple[str, str], right: tuple[str, str]) -> bool:
        left_material, left_collision = left
        right_material, right_collision = right
        if left_collision != right_collision:
            return False
        if left_material == right_material:
            return True
        # An outer tree wall can turn across an otherwise shared border. Its
        # complementary edge slices differ, but it remains the same blocked
        # world boundary rather than a separator between the two maps.
        return left_collision == "blocked" and left_material.startswith("forest-") and right_material.startswith("forest-")

    for (x, y), project in projects.items():
        east = projects.get((x + 1, y))
        south = projects.get((x, y + 1))
        if east is not None:
            assert all(
                matches(left, right)
                for left, right in zip(border(project, "east"), border(east, "west"))
            ), (x, y, "east")
        if south is not None:
            assert all(
                matches(top, bottom)
                for top, bottom in zip(border(project, "south"), border(south, "north"))
            ), (x, y, "south")


def align_outer_corners(projects: dict[tuple[int, int], dict[str, object]]) -> None:
    """Continue an exterior underbrush wall through a concave world corner."""
    directions = {
        "north": ((0, -1), lambda offset: (offset, 0), lambda offset: (offset, HEIGHT - 1)),
        "south": ((0, 1), lambda offset: (offset, HEIGHT - 1), lambda offset: (offset, 0)),
        "west": ((-1, 0), lambda offset: (0, offset), lambda offset: (WIDTH - 1, offset)),
        "east": ((1, 0), lambda offset: (WIDTH - 1, offset), lambda offset: (0, offset)),
    }
    for (x, y), project in projects.items():
        for delta, own_position, neighbour_position in directions.values():
            neighbour = projects.get((x + delta[0], y + delta[1]))
            if neighbour is None:
                continue
            for offset in range(WIDTH if delta[1] else HEIGHT):
                own_x, own_y = own_position(offset)
                own_material = project["visual_cells"][index(own_x, own_y)]["material"]
                if own_material == UNDERBRUSH_LEFT:
                    neighbour_x, neighbour_y = neighbour_position(offset)
                    if delta == (-1, 0):
                        neighbour_x = WIDTH - 2
                    if neighbour_x + 1 < WIDTH:
                        place_underbrush(
                            neighbour["visual_cells"],
                            neighbour["collision_cells"],
                            neighbour["event_cells"],
                            neighbour_x,
                            neighbour_y,
                        )


def validate_outer_walls(projects: dict[tuple[int, int], dict[str, object]]) -> None:
    for (x, y), project in projects.items():
        neighbours = {
            "north": (x, y - 1),
            "south": (x, y + 1),
            "west": (x - 1, y),
            "east": (x + 1, y),
        }
        for direction, coordinate in neighbours.items():
            if coordinate in projects:
                continue
            for material, collision in border(project, direction):
                assert material.startswith("forest-") and collision == "blocked", (
                    x,
                    y,
                    direction,
                )


def validate_complete_footprints(project: dict[str, object]) -> None:
    """Reject a route map containing an incomplete multi-cell object."""
    materials = {
        material["id"]: material["layers"]
        for material in project["materials"]
    }

    def validate_template(name: str, template: list[list[str]]) -> None:
        parts = {
            tile: (column, row)
            for row, tile_row in enumerate(template)
            for column, tile in enumerate(tile_row)
        }
        for row in range(HEIGHT):
            for column in range(WIDTH):
                layers = materials[project["visual_cells"][index(column, row)]["material"]]
                for tile, (part_x, part_y) in parts.items():
                    if tile not in layers:
                        continue
                    assert layers[0] == "tile-0102", (name, column, row, layers)
                    origin_x = column - part_x
                    origin_y = row - part_y
                    assert 0 <= origin_x and 0 <= origin_y, (name, column, row, tile)
                    assert origin_x + len(template[0]) <= WIDTH, (name, column, row, tile)
                    assert origin_y + len(template) <= HEIGHT, (name, column, row, tile)
                    for expected_y, expected_row in enumerate(template):
                        for expected_x, expected_tile in enumerate(expected_row):
                            expected_layers = materials[
                                project["visual_cells"][
                                    index(origin_x + expected_x, origin_y + expected_y)
                                ]["material"]
                            ]
                            assert expected_tile in expected_layers, (
                                name,
                                origin_x,
                                origin_y,
                                expected_tile,
                            )

    validate_template("tree-3x3", TREE_3X3)
    validate_template("tree-2x3", TREE_2X3)
    validate_template("tall-grass-4x5", TALL_GRASS)
    validate_template("underbrush-1x2", UNDERBRUSH)


def main() -> None:
    source = json.loads(SOURCE_MAP.read_text(encoding="utf-8"))
    if (source["width"], source["height"]) != (WIDTH, HEIGHT):
        raise ValueError("demo map no longer matches the standard outdoor map size")

    OUTPUT.mkdir(parents=True, exist_ok=True)
    occupied = {spec.coord for spec in SPECS}
    projects = {
        spec.coord: build_map(source, spec, occupied)
        for spec in SPECS
    }
    align_outer_corners(projects)
    validate_connections(projects)
    validate_outer_walls(projects)
    for project in projects.values():
        validate_complete_footprints(project)
    for spec in SPECS:
        path = OUTPUT / f"{spec.slug}.json"
        path.write_text(
            json.dumps(projects[spec.coord], ensure_ascii=True, indent=2) + "\n",
            encoding="utf-8",
            newline="\r\n",
        )
    WORLD_MANIFEST.write_text(
        json.dumps(
            {
                "format_version": "world-map-layout-v1",
                "initial": [1, 1],
                "maps": [
                    {"coordinate": list(spec.coord), "file": f"{spec.slug}.json"}
                    for spec in SPECS
                ],
            },
            ensure_ascii=True,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
        newline="\r\n",
    )
    print(f"generated {len(SPECS)} maps in {OUTPUT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
