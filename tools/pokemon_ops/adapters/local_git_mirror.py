from __future__ import annotations

import subprocess
from pathlib import Path

from tools.pokemon_ops.adapters.streaming_process import run_streamed_process
from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import GitMirrorStatus, GitSyncReport, LocalConfig, ProgressEvent, ProgressEventType
from tools.pokemon_ops.ports.interfaces import ProgressReporter


class LocalGitMirror:
    def initialize(self, config: LocalConfig, progress: ProgressReporter | None = None) -> Result[GitSyncReport]:
        mirror = config.mirror_root.wsl_mount_path
        self._report_progress(progress, "mirror.validate", "validating mirror directory")
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
        self._report_progress(progress, "mirror.clone", f"cloning {config.git_mirror.remote_name}/{config.git_mirror.branch} into the Windows mirror")
        cloned = self._run_streamed(
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
            progress,
            "mirror.clone",
        )
        if not cloned.is_ok:
            return Result(error=cloned.error)
        lfs = self._pull_lfs(mirror, config, progress, "mirror.lfs")
        if not lfs.is_ok:
            return Result(error=lfs.error)
        self._report_progress(progress, "mirror.verify", "verifying initialized mirror")
        reported = self._report(config, mirror_before="", fast_forwarded=True)
        if reported.is_ok:
            self._report_progress(progress, "mirror.done", "mirror initialization completed")
        return reported

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
        self._report_progress(progress, "sync.inspect", "inspecting mirror state")
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
        self._report_progress(progress, "sync.fetch", f"fetching {config.git_mirror.remote_name}/{config.git_mirror.branch}")
        fetched = self._run_streamed(
            ("git", "fetch", "--no-tags", config.git_mirror.remote_name, config.git_mirror.branch),
            mirror,
            ErrorCode.GIT_SYNC_FAILED,
            "cannot fetch the configured mirror branch",
            progress,
            "sync.fetch",
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
            needs_lfs = self._requires_lfs_update(mirror, mirror_before)
            if not needs_lfs.is_ok:
                return Result(error=needs_lfs.error)
            self._report_progress(progress, "sync.fast_forward", f"fast-forwarding mirror to {remote_head.value}")
            merged = self._run_streamed(
                ("git", "merge", "--ff-only", "FETCH_HEAD"),
                mirror,
                ErrorCode.GIT_SYNC_FAILED,
                "cannot fast-forward the mirror",
                progress,
                "sync.fast_forward",
            )
            if not merged.is_ok:
                return Result(error=merged.error)
            if needs_lfs.value:
                lfs = self._pull_lfs(mirror, config, progress, "sync.lfs")
                if not lfs.is_ok:
                    return Result(error=lfs.error)
            else:
                self._report_progress(progress, "sync.lfs", "skipping Git LFS: fast-forward contains no changed LFS pointers")
        else:
            self._report_progress(progress, "sync.lfs", "skipping Git LFS: mirror already matches the remote commit")
        self._report_progress(progress, "sync.verify", "verifying synchronized mirror")
        reported = self._report(
            config,
            mirror_before=mirror_before,
            fast_forwarded=mirror_before != remote_head.value,
            remote_head=remote_head.value or "",
        )
        if reported.is_ok:
            message = "mirror fast-forward completed" if reported.value and reported.value.fast_forwarded else "mirror already matches the remote commit"
            self._report_progress(progress, "sync.done", message)
        return reported

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

    def _pull_lfs(self, mirror: Path, config: LocalConfig, progress: ProgressReporter | None, stage: str) -> Result[None]:
        attributes = mirror / ".gitattributes"
        if not attributes.is_file() or "filter=lfs" not in attributes.read_text(encoding="utf-8"):
            return Result.ok(None)
        self._report_progress(progress, stage, "updating Git LFS objects")
        pulled = self._run_streamed(
            ("git", "lfs", "pull", config.git_mirror.remote_name, config.git_mirror.branch),
            mirror,
            ErrorCode.GIT_LFS_UNAVAILABLE,
            "cannot update Git LFS objects",
            progress,
            stage,
        )
        if not pulled.is_ok:
            return Result(error=pulled.error)
        return Result.ok(None)

    def _requires_lfs_update(self, mirror: Path, mirror_before: str) -> Result[bool]:
        changed = self._output(
            ("git", "diff", "--name-only", "-z", "--diff-filter=ACMR", mirror_before, "FETCH_HEAD"),
            mirror,
            ErrorCode.GIT_SYNC_FAILED,
            "cannot inspect changed files for Git LFS",
        )
        if not changed.is_ok:
            return Result(error=changed.error)
        paths = [path for path in (changed.value or "").split("\0") if path]
        if ".gitattributes" in paths:
            return Result.ok(True)
        for path in paths:
            size = self._output(
                ("git", "cat-file", "-s", f"FETCH_HEAD:{path}"),
                mirror,
                ErrorCode.GIT_SYNC_FAILED,
                "cannot inspect changed file size for Git LFS",
            )
            if not size.is_ok:
                return Result(error=size.error)
            try:
                is_small_file = int(size.value or "0") <= 1024
            except ValueError:
                return Result.fail(ErrorCode.GIT_SYNC_FAILED, "cannot parse changed file size for Git LFS", path=path)
            if not is_small_file:
                continue
            content = self._output(
                ("git", "cat-file", "-p", f"FETCH_HEAD:{path}"),
                mirror,
                ErrorCode.GIT_SYNC_FAILED,
                "cannot inspect changed file content for Git LFS",
            )
            if not content.is_ok:
                return Result(error=content.error)
            if (content.value or "").startswith("version https://git-lfs.github.com/spec/v1\n"):
                return Result.ok(True)
        return Result.ok(False)

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

    def _run_streamed(
        self,
        arguments: tuple[str, ...],
        cwd: Path,
        code: ErrorCode,
        message: str,
        progress: ProgressReporter | None,
        stage: str,
    ) -> Result[None]:
        completed = run_streamed_process(
            arguments,
            cwd,
            progress,
            stage,
            unavailable_code=ErrorCode.GIT_UNAVAILABLE,
            start_failure_code=ErrorCode.GIT_SYNC_FAILED,
        )
        if not completed.is_ok:
            assert completed.error is not None
            self._report_error(progress, stage, completed.error.code, completed.error.message)
            return Result(error=completed.error)
        assert completed.value is not None
        if completed.value.exit_code != 0:
            details = {
                "executable": arguments[0],
                "exit_code": str(completed.value.exit_code),
            }
            if completed.value.output_tail:
                details["output_tail"] = "\n".join(completed.value.output_tail)
            self._report_error(progress, stage, code, message)
            return Result.fail(code, message, **details)
        return Result.ok(None)

    @staticmethod
    def _completed(arguments: tuple[str, ...], cwd: Path) -> Result[int]:
        try:
            completed = subprocess.run(arguments, cwd=cwd, check=False)
        except FileNotFoundError:
            return Result.fail(ErrorCode.GIT_UNAVAILABLE, "Git executable is unavailable", executable="git")
        except OSError as error:
            return Result.fail(ErrorCode.GIT_SYNC_FAILED, "cannot start Git", reason=str(error))
        return Result.ok(completed.returncode)

    @staticmethod
    def _report_progress(progress: ProgressReporter | None, stage: str, message: str) -> None:
        if progress is not None:
            progress.report(ProgressEvent(ProgressEventType.PROGRESS, stage, message))

    @staticmethod
    def _report_error(progress: ProgressReporter | None, stage: str, code: ErrorCode, message: str) -> None:
        if progress is not None:
            progress.report(ProgressEvent(ProgressEventType.ERROR, stage, message, code=code.value))
