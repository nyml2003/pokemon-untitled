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
    if operation not in {"build_game_host", "run_game_host"} or profile not in {"debug", "release"}:
        return 2

    command = ["cargo", "build" if operation == "build_game_host" else "run", "--bin", "game-host"]
    if profile == "release":
        command.append("--release")
    return subprocess.run(command, cwd=Path(windows_root), check=False).returncode


if __name__ == "__main__":
    raise SystemExit(main())
