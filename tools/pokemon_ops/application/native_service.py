from __future__ import annotations

from tools.pokemon_ops.application.sync_service import SyncReport, SyncService
from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import BuildProfile, LocalConfig, NativeOperation, NativeRunRequest
from tools.pokemon_ops.ports.interfaces import NativeRunDispatcher


class NativeService:
    def __init__(self, sync_service: SyncService, dispatcher: NativeRunDispatcher) -> None:
        self._sync_service = sync_service
        self._dispatcher = dispatcher

    def execute(
        self,
        config: LocalConfig,
        operation: NativeOperation,
        profile: BuildProfile,
    ) -> Result[tuple[SyncReport, int]]:
        synced = self._sync_service.sync(config, delete_removed=True, dry_run=False)
        if not synced.is_ok:
            return Result(error=synced.error)
        dispatched = self._dispatcher.dispatch(
            NativeRunRequest(operation=operation, profile=profile, mirror_root=config.mirror_root)
        )
        if not dispatched.is_ok:
            return Result(error=dispatched.error)
        return Result.ok((synced.value, dispatched.value or 0))
