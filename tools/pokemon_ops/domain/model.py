from __future__ import annotations

from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path, PureWindowsPath


class TestSuite(StrEnum):
    CORE = "core"
    WORLD = "world"
    ALL = "all"


class BuildProfile(StrEnum):
    DEBUG = "debug"
    RELEASE = "release"


class NativeOperation(StrEnum):
    BUILD_GAME_HOST = "build_game_host"
    RUN_GAME_HOST = "run_game_host"


@dataclass(frozen=True)
class SourceRoot:
    path: Path


@dataclass(frozen=True)
class MirrorRoot:
    wsl_mount_path: Path
    windows_path: PureWindowsPath


@dataclass(frozen=True)
class WindowsRunner:
    python_executable: Path
    module: str


@dataclass(frozen=True)
class LocalConfig:
    source_root: SourceRoot
    mirror_root: MirrorRoot
    windows_runner: WindowsRunner
    unit_suites: dict[TestSuite, tuple[str, ...]]


@dataclass(frozen=True)
class MirrorMarker:
    project_id: str
    source_id: str


@dataclass(frozen=True)
class NativeRunRequest:
    operation: NativeOperation
    profile: BuildProfile
    mirror_root: MirrorRoot
