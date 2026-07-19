from __future__ import annotations

from dataclasses import dataclass, field
from enum import StrEnum
from typing import Generic, TypeVar


class ErrorCode(StrEnum):
    INVALID_CONFIGURATION = "InvalidConfiguration"
    SOURCE_MISSING = "SourceMissing"
    MIRROR_MISSING = "MirrorMissing"
    UNSAFE_MIRROR = "UnsafeMirror"
    COPY_FAILED = "CopyFailed"
    GIT_UNAVAILABLE = "GitUnavailable"
    GIT_REMOTE_UNAVAILABLE = "GitRemoteUnavailable"
    GIT_LFS_UNAVAILABLE = "GitLfsUnavailable"
    GIT_SYNC_FAILED = "GitSyncFailed"
    MIRROR_DIRTY = "MirrorDirty"
    MIRROR_DIVERGED = "MirrorDiverged"
    WINDOWS_PYTHON_UNAVAILABLE = "WindowsPythonUnavailable"
    WINDOWS_RUNNER_UNAVAILABLE = "WindowsRunnerUnavailable"
    PROCESS_FAILED = "ProcessFailed"
    BUILD_FAILED = "BuildFailed"
    RUN_FAILED = "RunFailed"
    UNSUPPORTED_HOST = "UnsupportedHost"


@dataclass(frozen=True)
class Diagnostic:
    code: ErrorCode
    message: str
    details: dict[str, str] = field(default_factory=dict)


T = TypeVar("T")


@dataclass(frozen=True)
class Result(Generic[T]):
    value: T | None = None
    error: Diagnostic | None = None

    @property
    def is_ok(self) -> bool:
        return self.error is None

    @classmethod
    def ok(cls, value: T) -> "Result[T]":
        return cls(value=value)

    @classmethod
    def fail(
        cls,
        code: ErrorCode,
        message: str,
        **details: str,
    ) -> "Result[T]":
        return cls(error=Diagnostic(code=code, message=message, details=details))
