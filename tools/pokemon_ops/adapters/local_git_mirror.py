from __future__ import annotations

import subprocess
from pathlib import Path
from typing import Iterable

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import GitMirrorStatus, GitSyncReport, LocalConfig
from tools.pokemon_ops.ports.interfaces import ProgressReporter


class LocalGitMirror:
    def initialize(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]:
        mirror = config.mirror_root.wsl_mount_path
        if mirror.exists() and any(mirror.iterdir()):
            return Result.fail(
                ErrorCode.UNSAFE_MIRROR,
                "refusing to initialize a non-empty mirror directory",
                mirror_root=str(mirror),
                remediation="configure an empty mirror directory",
            )

        source_remote = self._source_remote(config)
        if not source_remote.is_ok:
            return Result(error=source_remote.error)
        if progress is not None:
            progress(f"cloning {config.git_mirror.remote_name}/{config.git_mirror.branch} into the Windows mirror")
        cloned = self._run_forwarded(
            (
                "git",
                "clone",
                "--branch",
                config.git_mirror.branch,
                "--single-branch",
                source_remote.value or "",
                str(mirror),
            ),
            config.source_root.path,
            ErrorCode.GIT_SYNC_FAILED,
            "cannot clone the configured mirror branch",
        )
        if not cloned.is_ok:
            return Result(error=cloned.error)
        lfs = self._pull_lfs(mirror, config, progress)
        if not lfs.is_ok:
            return Result(error=lfs.error)
        return self._report(config, mirror_before="", fast_forwarded=True)

    def inspect(self, config: LocalConfig) -> Result[GitMirrorStatus]:
        mirror = config.mirror_root.wsl_mount_path
        if not mirror.is_dir():
            return Result.fail(
                ErrorCode.MIRROR_MISSING,
                "mirror root does not exist",
                path=str(mirror),
                remediation="run ops init-mirror",
            )
        source_head = self._output(("git", "rev-parse", "HEAD"), config.source_root.path, ErrorCode.GIT_REMOTE_UNAVAILABLE, "cannot read source Git revision")
        if not source_head.is_ok:
            return Result(error=source_head.error)
        remote_head = self._output(
            ("git", "ls-remote", "--heads", config.git_mirror.remote_name, f"refs/heads/{config.git_mirror.branch}"),
            config.source_root.path,
            ErrorCode.GIT_REMOTE_UNAVAILABLE,
            "cannot read the configured remote branch",
        )
        if not remote_head.is_ok:
            return Result(error=remote_head.error)
        remote_parts = (remote_head.value or "").split()
        if not remote_parts:
            return Result.fail(
                ErrorCode.GIT_REMOTE_UNAVAILABLE,
                "configured remote branch does not exist",
                remote=config.git_mirror.remote_name,
                branch=config.git_mirror.branch,
            )
        valid = self._validate_mirror(config)
        if not valid.is_ok:
            return Result(error=valid.error)
        mirror_head = valid.value or ""
        dirty = self._output(("git", "status", "--porcelain", "--untracked-files=no"), mirror, ErrorCode.GIT_SYNC_FAILED, "cannot read mirror status")
        if not dirty.is_ok:
            return Result(error=dirty.error)
        return Result.ok(
            GitMirrorStatus(
                source_head=source_head.value or "",
                remote_head=remote_parts[0],
                mirror_head=mirror_head,
                mirror_dirty=bool(dirty.value),
                mirror_matches_remote=mirror_head == remote_parts[0],
            )
        )

    def sync(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]:
        mirror = config.mirror_root.wsl_mount_path
        valid = self._validate_mirror(config)
        if not valid.is_ok:
            return Result(error=valid.error)
        mirror_before = valid.value or ""
        dirty = self._output(("git", "status", "--porcelain", "--untracked-files=no"), mirror, ErrorCode.GIT_SYNC_FAILED, "cannot read mirror status")
        if not dirty.is_ok:
            return Result(error=dirty.error)
        if dirty.value:
            return Result.fail(
                ErrorCode.MIRROR_DIRTY,
                "mirror has tracked local modifications",
                mirror_root=str(mirror),
                remediation="commit or discard the mirror changes before running ops",
            )
        if progress is not None:
            progress(f"fetching {config.git_mirror.remote_name}/{config.git_mirror.branch}")
        fetched = self._run_forwarded(
            ("git", "fetch", "--no-tags", config.git_mirror.remote_name, config.git_mirror.branch),
            mirror,
            ErrorCode.GIT_SYNC_FAILED,
            "cannot fetch the configured mirror branch",
        )
        if not fetched.is_ok:
            return Result(error=fetched.error)
        remote_head = self._output(("git", "rev-parse", "FETCH_HEAD"), mirror, ErrorCode.GIT_SYNC_FAILED, "cannot read fetched revision")
        if not remote_head.is_ok:
            return Result(error=remote_head.error)
        ancestor = self._completed(("git", "merge-base", "--is-ancestor", "HEAD", "FETCH_HEAD"), mirror)
        if not ancestor.is_ok:
            return Result(error=ancestor.error)
        if ancestor.value != 0:
            return Result.fail(
                ErrorCode.MIRROR_DIVERGED,
                "mirror cannot fast-forward to the configured remote branch",
                mirror_head=mirror_before,
                remote_head=remote_head.value or "",
            )
        if mirror_before != remote_head.value:
            if progress is not None:
                progress(f"fast-forwarding mirror to {remote_head.value}")
            merged = self._run_forwarded(
                ("git", "merge", "--ff-only", "FETCH_HEAD"),
                mirror,
                ErrorCode.GIT_SYNC_FAILED,
                "cannot fast-forward the mirror",
            )
            if not merged.is_ok:
                return Result(error=merged.error)
        lfs = self._pull_lfs(mirror, config, progress)
        if not lfs.is_ok:
            return Result(error=lfs.error)
        return self._report(
            config,
            mirror_before=mirror_before,
            fast_forwarded=mirror_before != remote_head.value,
            remote_head=remote_head.value or "",
        )

    def _source_remote(self, config: LocalConfig) -> Result[str]:
        return self._output(
            ("git", "remote", "get-url", config.git_mirror.remote_name),
            config.source_root.path,
            ErrorCode.GIT_REMOTE_UNAVAILABLE,
            "configured source remote is unavailable",
        )

    def _validate_mirror(self, config: LocalConfig) -> Result[str]:
        mirror = config.mirror_root.wsl_mount_path
        if not mirror.is_dir():
            return Result.fail(
                ErrorCode.MIRROR_MISSING,
                "mirror root does not exist",
                path=str(mirror),
                remediation="run ops init-mirror",
            )
        repository = self._output(("git", "rev-parse", "--is-inside-work-tree"), mirror, ErrorCode.UNSAFE_MIRROR, "mirror is not a Git worktree")
        if not repository.is_ok or repository.value != "true":
            return Result.fail(
                ErrorCode.UNSAFE_MIRROR,
                "mirror is not a Git worktree",
                mirror_root=str(mirror),
                remediation="configure an empty mirror directory and run ops init-mirror",
            )
        source_remote = self._source_remote(config)
        if not source_remote.is_ok:
            return Result(error=source_remote.error)
        mirror_remote = self._output(
            ("git", "remote", "get-url", config.git_mirror.remote_name),
            mirror,
            ErrorCode.UNSAFE_MIRROR,
            "mirror does not have the configured remote",
        )
        if not mirror_remote.is_ok:
            return Result(error=mirror_remote.error)
        if mirror_remote.value != source_remote.value:
            return Result.fail(
                ErrorCode.UNSAFE_MIRROR,
                "mirror remote does not match the source remote",
                expected_remote=source_remote.value or "",
                actual_remote=mirror_remote.value or "",
            )
        branch = self._output(("git", "branch", "--show-current"), mirror, ErrorCode.UNSAFE_MIRROR, "cannot read mirror branch")
        if not branch.is_ok:
            return Result(error=branch.error)
        if branch.value != config.git_mirror.branch:
            return Result.fail(
                ErrorCode.UNSAFE_MIRROR,
                "mirror is checked out on the wrong branch",
                expected_branch=config.git_mirror.branch,
                actual_branch=branch.value or "",
            )
        return self._output(("git", "rev-parse", "HEAD"), mirror, ErrorCode.UNSAFE_MIRROR, "cannot read mirror revision")

    def _pull_lfs(self, mirror: Path, config: LocalConfig, progress: ProgressReporter | None) -> Result[None]:
        attributes = mirror / ".gitattributes"
        if not attributes.is_file() or "filter=lfs" not in attributes.read_text(encoding="utf-8"):
            return Result.ok(None)
        if progress is not None:
            progress("updating Git LFS objects")
        pulled = self._run_forwarded(
            ("git", "lfs", "pull", config.git_mirror.remote_name, config.git_mirror.branch),
            mirror,
            ErrorCode.GIT_LFS_UNAVAILABLE,
            "cannot update Git LFS objects",
        )
        if not pulled.is_ok:
            return Result(error=pulled.error)
        return Result.ok(None)

    def _report(
        self,
        config: LocalConfig,
        mirror_before: str,
        fast_forwarded: bool,
        remote_head: str | None = None,
    ) -> Result[GitSyncReport]:
        source_head = self._output(("git", "rev-parse", "HEAD"), config.source_root.path, ErrorCode.GIT_REMOTE_UNAVAILABLE, "cannot read source Git revision")
        if not source_head.is_ok:
            return Result(error=source_head.error)
        mirror_after = self._output(("git", "rev-parse", "HEAD"), config.mirror_root.wsl_mount_path, ErrorCode.GIT_SYNC_FAILED, "cannot read mirror revision")
        if not mirror_after.is_ok:
            return Result(error=mirror_after.error)
        return Result.ok(
            GitSyncReport(
                source_head=source_head.value or "",
                remote_head=remote_head or mirror_after.value or "",
                mirror_before=mirror_before,
                mirror_after=mirror_after.value or "",
                fast_forwarded=fast_forwarded,
            )
        )

    def _output(self, arguments: tuple[str, ...], cwd: Path, code: ErrorCode, message: str) -> Result[str]:
        try:
            completed = subprocess.run(arguments, cwd=cwd, check=False, text=True, capture_output=True)
        except FileNotFoundError:
            return Result.fail(ErrorCode.GIT_UNAVAILABLE, "Git executable is unavailable", executable="git")
        except OSError as error:
            return Result.fail(ErrorCode.GIT_SYNC_FAILED, "cannot start Git", reason=str(error))
        if completed.returncode != 0:
            return Result.fail(
                code,
                message,
                executable=arguments[0],
                exit_code=str(completed.returncode),
                reason=completed.stderr.strip(),
            )
        return Result.ok(completed.stdout.strip())

    def _run_forwarded(self, arguments: tuple[str, ...], cwd: Path, code: ErrorCode, message: str) -> Result[None]:
        completed = self._completed(arguments, cwd)
        if not completed.is_ok:
            return Result(error=completed.error)
        if completed.value != 0:
            return Result.fail(code, message, executable=arguments[0], exit_code=str(completed.value))
        return Result.ok(None)

    def _completed(self, arguments: Iterable[str], cwd: Path) -> Result[int]:
        try:
            completed = subprocess.run(tuple(arguments), cwd=cwd, check=False)
        except FileNotFoundError:
            return Result.fail(ErrorCode.GIT_UNAVAILABLE, "Git executable is unavailable", executable="git")
        except OSError as error:
            return Result.fail(ErrorCode.GIT_SYNC_FAILED, "cannot start Git", reason=str(error))
        return Result.ok(completed.returncode)
