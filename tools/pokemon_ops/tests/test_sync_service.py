from __future__ import annotations

import tempfile
import unittest
from pathlib import Path, PureWindowsPath

from tools.pokemon_ops.adapters.local_file_tree import JsonMarkerStore, LocalFileTree
from tools.pokemon_ops.application.sync_service import SyncService
from tools.pokemon_ops.domain.errors import ErrorCode
from tools.pokemon_ops.domain.model import LocalConfig, MirrorRoot, SourceRoot, TestSuite, WindowsRunner


def make_config(source: Path, mirror: Path) -> LocalConfig:
    return LocalConfig(
        source_root=SourceRoot(source),
        mirror_root=MirrorRoot(mirror, PureWindowsPath("C:/mirror")),
        windows_runner=WindowsRunner(Path("/mnt/c/Python/python.exe"), "tools.pokemon_ops.native_runner"),
        unit_suites={TestSuite.CORE: ("core",), TestSuite.WORLD: ("world",), TestSuite.ALL: ("workspace",)},
    )


class SyncServiceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.files = LocalFileTree()
        self.service = SyncService(self.files, JsonMarkerStore())

    def test_initializes_empty_mirror_and_deletes_removed_files(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            mirror = root / "mirror"
            source.mkdir()
            mirror.mkdir()
            (source / "assets").mkdir()
            (source / "assets" / "new.png").write_text("new", encoding="utf-8")
            config = make_config(source, mirror)

            first = self.service.sync(config, delete_removed=True, dry_run=False)
            self.assertTrue(first.is_ok)
            self.assertTrue((mirror / ".pokemon-ops-mirror.json").is_file())
            self.assertEqual((mirror / "assets" / "new.png").read_text(encoding="utf-8"), "new")

            (source / "assets" / "new.png").unlink()
            second = self.service.sync(config, delete_removed=True, dry_run=False)
            self.assertTrue(second.is_ok)
            self.assertFalse((mirror / "assets" / "new.png").exists())

    def test_rejects_unmarked_non_empty_mirror(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            mirror = root / "mirror"
            source.mkdir()
            mirror.mkdir()
            (source / "new.txt").write_text("new", encoding="utf-8")
            (mirror / "unowned.txt").write_text("old", encoding="utf-8")

            result = self.service.sync(make_config(source, mirror), delete_removed=True, dry_run=False)
            self.assertFalse(result.is_ok)
            assert result.error is not None
            self.assertEqual(result.error.code, ErrorCode.UNSAFE_MIRROR)
