from __future__ import annotations

from pathlib import Path, PurePosixPath
from typing import Protocol

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import MirrorMarker, NativeRunRequest


class FileTree(Protocol):
    def is_directory(self, path: Path) -> bool: ...

    def list_files(self, root: Path) -> Result[set[PurePosixPath]]: ...

    def is_empty(self, root: Path) -> Result[bool]: ...

    def copy_file(self, source_root: Path, destination_root: Path, path: PurePosixPath) -> Result[None]: ...

    def delete_file(self, root: Path, path: PurePosixPath) -> Result[None]: ...


class MarkerStore(Protocol):
    def read(self, mirror_root: Path) -> Result[MirrorMarker | None]: ...

    def write(self, mirror_root: Path, marker: MirrorMarker) -> Result[None]: ...


class ProcessRunner(Protocol):
    def run(self, arguments: tuple[str, ...], cwd: Path, forward_output: bool = False) -> Result[int]: ...


class NativeRunDispatcher(Protocol):
    def dispatch(self, request: NativeRunRequest) -> Result[int]: ...
