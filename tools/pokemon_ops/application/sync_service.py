from __future__ import annotations

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import GitMirrorStatus, GitSyncReport, LocalConfig
from tools.pokemon_ops.ports.interfaces import GitMirror, ProgressReporter


class SyncService:
    def __init__(self, mirror: GitMirror) -> None:
        self._mirror = mirror

    def initialize(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]:
        return self._mirror.initialize(config, progress)

    def check(self, config: LocalConfig) -> Result[GitMirrorStatus]:
        return self._mirror.inspect(config)

    def sync(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]:
        return self._mirror.sync(config, progress)
