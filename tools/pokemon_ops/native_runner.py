from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


def main() -> int:
    try:
        request = json.loads(sys.stdin.read())
        operation = request["operation"]
        profile = request["profile"]
        windows_root = request["windows_root"]
    except (KeyError, json.JSONDecodeError, TypeError):
        return 2
    operations = {
        "build_game_host": "game-host",
        "run_game_host": "game-host",
        "build_map_editor": "map-editor",
        "run_map_editor": "map-editor",
        "build_trainer_editor": "trainer-editor",
        "run_trainer_editor": "trainer-editor",
        "build_pokemon_editor": "pokemon-editor",
        "run_pokemon_editor": "pokemon-editor",
    }
    if operation not in operations or profile not in {"debug", "release"}:
        return 2

    command = ["cargo", "build" if operation.startswith("build_") else "run", "--bin", operations[operation]]
    if profile == "release":
        command.append("--release")
    return subprocess.run(command, cwd=Path(windows_root), check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
