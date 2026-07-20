from __future__ import annotations

from tools.pokemon_ops.application.sync_service import SyncService
from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import BuildProfile, GitSyncReport, LocalConfig, NativeOperation, NativeRunRequest, ProgressEvent, ProgressEventType
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
        action = "building" if operation.is_build else "running"
        stage = "build.start" if operation.is_build else "run.start"
        if progress is not None:
            progress.report(ProgressEvent(ProgressEventType.PROGRESS, stage, f"{action} {operation.target} on Windows; native output follows"))
        dispatched = self._dispatcher.dispatch(
            NativeRunRequest(operation=operation, profile=profile, mirror_root=config.mirror_root),
            progress=progress,
        )
        if not dispatched.is_ok:
            return Result(error=dispatched.error)
        return Result.ok((synced.value, dispatched.value or 0))
