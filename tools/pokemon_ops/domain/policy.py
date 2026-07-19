from __future__ import annotations

from dataclasses import dataclass
from pathlib import PurePosixPath


EXCLUDED_TOP_LEVEL = frozenset(
    {
        ".git",
        "target",
        ".direnv",
        ".idea",
        ".vscode",
        ".pytest_cache",
        ".mypy_cache",
        "__pycache__",
        "ops.local.json",
        ".pokemon-ops-mirror.json",
    }
)


def is_syncable(path: PurePosixPath) -> bool:
    return bool(path.parts) and not any(part in EXCLUDED_TOP_LEVEL for part in path.parts)


@dataclass(frozen=True)
class SyncPlan:
    copies: tuple[PurePosixPath, ...]
    deletes: tuple[PurePosixPath, ...]


def make_sync_plan(
    source_files: set[PurePosixPath],
    mirror_files: set[PurePosixPath],
    delete_removed: bool,
) -> SyncPlan:
    source = {path for path in source_files if is_syncable(path)}
    mirror = {path for path in mirror_files if is_syncable(path)}
    deletes = mirror - source if delete_removed else set()
    return SyncPlan(copies=tuple(sorted(source)), deletes=tuple(sorted(deletes)))
