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
    BUILD_MAP_EDITOR = "build_map_editor"
    RUN_MAP_EDITOR = "run_map_editor"
    BUILD_TRAINER_EDITOR = "build_trainer_editor"
    RUN_TRAINER_EDITOR = "run_trainer_editor"
    BUILD_POKEMON_EDITOR = "build_pokemon_editor"
    RUN_POKEMON_EDITOR = "run_pokemon_editor"

    @property
    def is_build(self) -> bool:
        return self.value.startswith("build_")

    @property
    def target(self) -> str:
        prefix = "build_" if self.is_build else "run_"
        return self.value.removeprefix(prefix).replace("_", "-")


class ProgressEventType(StrEnum):
    PROGRESS = "progress"
    OUTPUT = "output"
    WARNING = "warning"
    ERROR = "error"


class ProcessStream(StrEnum):
    STDOUT = "stdout"
    STDERR = "stderr"


@dataclass(frozen=True)
class ProgressEvent:
    type: ProgressEventType
    stage: str
    message: str
    stream: ProcessStream | None = None
    code: str | None = None
    remediation: str | None = None


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
