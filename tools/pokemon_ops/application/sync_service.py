from __future__ import annotations

from dataclasses import dataclass

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig, MirrorMarker
from tools.pokemon_ops.domain.policy import SyncPlan, make_sync_plan
from tools.pokemon_ops.ports.interfaces import FileTree, MarkerStore


PROJECT_ID = "pokemon-untitled"


@dataclass(frozen=True)
class SyncReport:
    plan: SyncPlan
    initialized_marker: bool
    applied: bool


class SyncService:
    def __init__(self, file_tree: FileTree, marker_store: MarkerStore) -> None:
        self._file_tree = file_tree
        self._marker_store = marker_store

    def sync(self, config: LocalConfig, delete_removed: bool, dry_run: bool) -> Result[SyncReport]:
        if not self._file_tree.is_directory(config.source_root.path):
            return Result.fail(ErrorCode.SOURCE_MISSING, "source root does not exist", path=str(config.source_root.path))
        if not self._file_tree.is_directory(config.mirror_root.wsl_mount_path):
            return Result.fail(ErrorCode.MIRROR_MISSING, "mirror root does not exist", path=str(config.mirror_root.wsl_mount_path))
        source_files = self._file_tree.list_files(config.source_root.path)
        if not source_files.is_ok:
            return Result(error=source_files.error)
        mirror_files = self._file_tree.list_files(config.mirror_root.wsl_mount_path)
        if not mirror_files.is_ok:
            return Result(error=mirror_files.error)

        source_id = str(config.source_root.path)
        marker = self._marker_store.read(config.mirror_root.wsl_mount_path)
        if not marker.is_ok:
            return Result(error=marker.error)

        initialized_marker = False
        if marker.value is None:
            empty = self._file_tree.is_empty(config.mirror_root.wsl_mount_path)
            if not empty.is_ok:
                return Result(error=empty.error)
            if not empty.value:
                return Result.fail(
                    ErrorCode.UNSAFE_MIRROR,
                    "refusing to synchronize an unmarked, non-empty mirror",
                    mirror_root=str(config.mirror_root.wsl_mount_path),
                )
            initialized_marker = True
        elif marker.value.project_id != PROJECT_ID or marker.value.source_id != source_id:
            return Result.fail(
                ErrorCode.UNSAFE_MIRROR,
                "mirror marker does not belong to this source root",
                mirror_root=str(config.mirror_root.wsl_mount_path),
            )

        plan = make_sync_plan(source_files.value or set(), mirror_files.value or set(), delete_removed)
        if dry_run:
            return Result.ok(SyncReport(plan=plan, initialized_marker=initialized_marker, applied=False))

        if initialized_marker:
            written = self._marker_store.write(
                config.mirror_root.wsl_mount_path,
                MirrorMarker(project_id=PROJECT_ID, source_id=source_id),
            )
            if not written.is_ok:
                return Result(error=written.error)

        for path in plan.copies:
            copied = self._file_tree.copy_file(config.source_root.path, config.mirror_root.wsl_mount_path, path)
            if not copied.is_ok:
                return Result(error=copied.error)
        for path in plan.deletes:
            deleted = self._file_tree.delete_file(config.mirror_root.wsl_mount_path, path)
            if not deleted.is_ok:
                return Result(error=deleted.error)
        return Result.ok(SyncReport(plan=plan, initialized_marker=initialized_marker, applied=True))
