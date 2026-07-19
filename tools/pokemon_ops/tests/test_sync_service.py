from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path, PureWindowsPath

from tools.pokemon_ops.adapters.local_git_mirror import LocalGitMirror
from tools.pokemon_ops.application.sync_service import SyncService
from tools.pokemon_ops.domain.errors import ErrorCode
from tools.pokemon_ops.domain.model import LocalConfig, MirrorRoot, SourceRoot, TestSuite, WindowsRunner


def git(cwd: Path, *arguments: str) -> str:
    completed = subprocess.run(("git", *arguments), cwd=cwd, check=True, text=True, capture_output=True)
    return completed.stdout.strip()


def config_for(root: Path) -> LocalConfig:
    remote = root / "remote.git"
    source = root / "source"
    mirror = root / "mirror"
    subprocess.run(("git", "init", "--bare", str(remote)), check=True, text=True, capture_output=True)
    source.mkdir()
    git(source, "init")
    git(source, "config", "user.email", "ops@example.invalid")
    git(source, "config", "user.name", "Pokemon Ops")
    (source / "tracked.txt").write_text("first\n", encoding="utf-8")
    git(source, "add", "tracked.txt")
    git(source, "commit", "-m", "initial")
    git(source, "branch", "-M", "master")
    git(source, "remote", "add", "origin", str(remote))
    git(source, "push", "-u", "origin", "master")
    return LocalConfig(
        source_root=SourceRoot(source),
        mirror_root=MirrorRoot(mirror, PureWindowsPath("C:/mirror")),
        windows_runner=WindowsRunner(Path("/mnt/c/Python/python.exe"), "tools.pokemon_ops.native_runner"),
        unit_suites={TestSuite.CORE: ("core",), TestSuite.WORLD: ("world",), TestSuite.ALL: ("workspace",)},
    )


class GitSyncServiceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.service = SyncService(LocalGitMirror())

    def test_initializes_and_fast_forwards_a_git_mirror(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            config = config_for(Path(directory))

            initialized = self.service.initialize(config)

            self.assertTrue(initialized.is_ok)
            self.assertTrue((config.mirror_root.wsl_mount_path / ".git").is_dir())
            self.assertEqual(initialized.value.mirror_after, git(config.source_root.path, "rev-parse", "HEAD"))  # type: ignore[union-attr]

            (config.source_root.path / "tracked.txt").write_text("second\n", encoding="utf-8")
            git(config.source_root.path, "commit", "-am", "update")
            git(config.source_root.path, "push")

            synchronized = self.service.sync(config)

            self.assertTrue(synchronized.is_ok)
            assert synchronized.value is not None
            self.assertTrue(synchronized.value.fast_forwarded)
            self.assertEqual(synchronized.value.mirror_after, git(config.source_root.path, "rev-parse", "HEAD"))
            self.assertEqual((config.mirror_root.wsl_mount_path / "tracked.txt").read_text(encoding="utf-8"), "second\n")

    def test_rejects_a_dirty_mirror_without_overwriting_it(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            config = config_for(Path(directory))
            self.assertTrue(self.service.initialize(config).is_ok)
            mirror_file = config.mirror_root.wsl_mount_path / "tracked.txt"
            mirror_file.write_text("local change\n", encoding="utf-8")

            result = self.service.sync(config)

            self.assertFalse(result.is_ok)
            assert result.error is not None
            self.assertEqual(result.error.code, ErrorCode.MIRROR_DIRTY)
            self.assertEqual(mirror_file.read_text(encoding="utf-8"), "local change\n")

    def test_rejects_non_empty_directory_during_initialization(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            config = config_for(Path(directory))
            config.mirror_root.wsl_mount_path.mkdir()
            (config.mirror_root.wsl_mount_path / "existing.txt").write_text("keep", encoding="utf-8")

            result = self.service.initialize(config)

            self.assertFalse(result.is_ok)
            assert result.error is not None
            self.assertEqual(result.error.code, ErrorCode.UNSAFE_MIRROR)
