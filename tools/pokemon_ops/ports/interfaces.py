from __future__ import annotations

from pathlib import Path
from typing import Protocol

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import GitMirrorStatus, GitSyncReport, LocalConfig, NativeRunRequest, ProgressEvent


class ProgressReporter(Protocol):
    def report(self, event: ProgressEvent) -> None: ...


class ProcessRunner(Protocol):
    def run(self, arguments: tuple[str, ...], cwd: Path, forward_output: bool = False) -> Result[int]: ...


class OutputProcessRunner(Protocol):
    def capture(self, arguments: tuple[str, ...], cwd: Path) -> Result[str]: ...


class NativeRunDispatcher(Protocol):
    def dispatch(self, request: NativeRunRequest, progress: ProgressReporter | None = None) -> Result[int]: ...


class GitMirror(Protocol):
    def initialize(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]: ...

    def inspect(self, config: LocalConfig) -> Result[GitMirrorStatus]: ...

    def sync(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]: ...
