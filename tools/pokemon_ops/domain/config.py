from __future__ import annotations

from pathlib import Path, PureWindowsPath
from typing import Any

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig, MirrorRoot, SourceRoot, TestSuite, WindowsRunner


def parse_local_config(data: Any, source_root: Path) -> Result[LocalConfig]:
    if not isinstance(data, dict):
        return Result.fail(ErrorCode.INVALID_CONFIGURATION, "ops.local.json must contain an object")

    mirror = data.get("mirror")
    runner = data.get("windows_runner")
    suites = data.get("unit_suites")
    if not isinstance(mirror, dict) or not isinstance(runner, dict) or not isinstance(suites, dict):
        return Result.fail(
            ErrorCode.INVALID_CONFIGURATION,
            "mirror, windows_runner, and unit_suites are required objects",
        )

    mount_root = mirror.get("wsl_mount_root")
    windows_root = mirror.get("windows_root")
    python_executable = runner.get("python_executable")
    module = runner.get("module")
    if not all(isinstance(value, str) and value for value in (mount_root, windows_root, python_executable, module)):
        return Result.fail(ErrorCode.INVALID_CONFIGURATION, "mirror and runner paths must be non-empty strings")

    parsed_suites: dict[TestSuite, tuple[str, ...]] = {}
    for suite in TestSuite:
        values = suites.get(suite.value)
        if not isinstance(values, list) or not values or not all(isinstance(value, str) and value for value in values):
            return Result.fail(
                ErrorCode.INVALID_CONFIGURATION,
                "every unit suite must contain one or more request IDs",
                suite=suite.value,
            )
        parsed_suites[suite] = tuple(values)

    source = source_root.resolve()
    mount = Path(mount_root).resolve()
    if source == mount or source in mount.parents or mount in source.parents:
        return Result.fail(
            ErrorCode.INVALID_CONFIGURATION,
            "source root and mirror root must not overlap",
            source_root=str(source),
            mirror_root=str(mount),
        )

    return Result.ok(
        LocalConfig(
            source_root=SourceRoot(source),
            mirror_root=MirrorRoot(wsl_mount_path=mount, windows_path=PureWindowsPath(windows_root)),
            windows_runner=WindowsRunner(python_executable=Path(python_executable), module=module),
            unit_suites=parsed_suites,
        )
    )
