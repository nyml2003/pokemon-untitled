from __future__ import annotations

from pathlib import Path

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import LocalConfig
from tools.pokemon_ops.ports.interfaces import ProcessRunner


DATA_IMPORT_COMMAND = (
    "cargo",
    "run",
    "-p",
    "game-data-import",
    "--",
    "--source",
    "assets/imports/pokeapi-current-data",
    "--output",
    "assets/source/data/game/current-dataset/v2.json",
    "--version-group",
    "emerald",
)


class DataService:
    def __init__(self, process_runner: ProcessRunner) -> None:
        self._process_runner = process_runner

    def generate(self, config: LocalConfig) -> Result[int]:
        return self._process_runner.run(DATA_IMPORT_COMMAND, Path(config.source_root.path), forward_output=True)
