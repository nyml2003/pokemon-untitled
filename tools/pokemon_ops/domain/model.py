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
class GitMirrorConfig:
    remote_name: str
    branch: str


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
    git_mirror: GitMirrorConfig = GitMirrorConfig(remote_name="origin", branch="master")


@dataclass(frozen=True)
class NativeRunRequest:
    operation: NativeOperation
    profile: BuildProfile
    mirror_root: MirrorRoot


@dataclass(frozen=True)
class GitMirrorStatus:
    source_head: str
    remote_head: str
    mirror_head: str
    mirror_dirty: bool
    mirror_matches_remote: bool


@dataclass(frozen=True)
class GitSyncReport:
    source_head: str
    remote_head: str
    mirror_before: str
    mirror_after: str
    fast_forwarded: bool
