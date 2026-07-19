from __future__ import annotations

from tools.pokemon_ops.application.sync_service import SyncService
from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import BuildProfile, GitSyncReport, LocalConfig, NativeOperation, NativeRunRequest
from tools.pokemon_ops.ports.interfaces import NativeRunDispatcher, ProgressReporter


class NativeService:
    def __init__(self, sync_service: SyncService, dispatcher: NativeRunDispatcher) -> None:
        self._sync_service = sync_service
        self._dispatcher = dispatcher

    def execute(
        self,
        config: LocalConfig,
        operation: NativeOperation,
        profile: BuildProfile,
        progress: ProgressReporter | None = None,
    ) -> Result[tuple[GitSyncReport, int]]:
        synced = self._sync_service.sync(config, progress=progress)
        if not synced.is_ok:
            return Result(error=synced.error)
        if progress is not None:
            action = "building" if operation is NativeOperation.BUILD_GAME_HOST else "running"
            progress(f"{action} game-host on Windows; native output follows")
        dispatched = self._dispatcher.dispatch(
            NativeRunRequest(operation=operation, profile=profile, mirror_root=config.mirror_root)
        )
        if not dispatched.is_ok:
            return Result(error=dispatched.error)
        return Result.ok((synced.value, dispatched.value or 0))
