from __future__ import annotations

from typing import Callable, Protocol

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import GitMirrorStatus, GitSyncReport, LocalConfig, NativeRunRequest


ProgressReporter = Callable[[str], None]


class ProcessRunner(Protocol):
    def run(self, arguments: tuple[str, ...], cwd: Path, forward_output: bool = False) -> Result[int]: ...


class NativeRunDispatcher(Protocol):
    def dispatch(self, request: NativeRunRequest) -> Result[int]: ...


class GitMirror(Protocol):
    def initialize(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]: ...

    def inspect(self, config: LocalConfig) -> Result[GitMirrorStatus]: ...

    def sync(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]: ...
