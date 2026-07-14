#!/usr/bin/env python3
"""Print the deterministic resource normalization plan without writing files."""

from __future__ import annotations

import json
from pathlib import Path

from asset_migration import asset_root, build_plan


def main() -> None:
    plan = build_plan()
    document = {
        "asset_root": str(asset_root()),
        "moves": [
            {
                "source": move.source.as_posix(),
                "target": move.target.as_posix(),
                "key": move.key,
            }
            for move in plan
        ],
    }
    print(json.dumps(document, ensure_ascii=False, indent=2) + "\n")


if __name__ == "__main__":
    main()
