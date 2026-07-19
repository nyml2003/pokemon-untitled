from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path, PureWindowsPath

from tools.pokemon_ops.application.metrics_service import TOKEI_COMMAND_PREFIX, WorkspaceMetricsService
from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import LocalConfig, MirrorRoot, SourceRoot, TestSuite, WindowsRunner


def config_for(root: Path) -> LocalConfig:
    return LocalConfig(
        source_root=SourceRoot(root),
        mirror_root=MirrorRoot(root / "mirror", PureWindowsPath("C:/mirror")),
        windows_runner=WindowsRunner(Path("/mnt/c/Python/python.exe"), "tools.pokemon_ops.native_runner"),
        unit_suites={TestSuite.CORE: (), TestSuite.WORLD: (), TestSuite.ALL: ()},
    )


class RecordingOutputProcessRunner:
    def __init__(self, output: str) -> None:
        self.output = output
        self.calls: list[tuple[str, ...]] = []

    def capture(self, arguments: tuple[str, ...], cwd: Path) -> Result[str]:
        self.calls.append(arguments)
        return Result.ok(self.output)


class WorkspaceMetricsServiceTests(unittest.TestCase):
    def test_lines_uses_tokei_for_production_and_tests(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "crates/demo/src").mkdir(parents=True)
            (root / "crates/demo/tests").mkdir(parents=True)
            runner = RecordingOutputProcessRunner(
                json.dumps(
                    {
                        "Rust": {
                            "blanks": 12,
                            "code": 34,
                            "comments": 5,
                            "reports": [{"name": "src/lib.rs", "stats": {}}],
                        }
                    }
                )
            )

            result = WorkspaceMetricsService(runner).lines(config_for(root))

            self.assertTrue(result.is_ok)
            report = result.value
            self.assertIsNotNone(report)
            assert report is not None
            self.assertEqual(report.production.files, 1)
            self.assertEqual(report.production.code, 34)
            self.assertEqual(report.production.comments, 5)
            self.assertEqual(report.production.blanks, 12)
            self.assertEqual(report.tests.code, 34)
            self.assertEqual(report.total.code, 68)
            self.assertEqual(runner.calls[0], (*TOKEI_COMMAND_PREFIX, str(root / "crates/demo/src")))
            self.assertEqual(runner.calls[1], (*TOKEI_COMMAND_PREFIX, str(root / "crates/demo/tests")))
