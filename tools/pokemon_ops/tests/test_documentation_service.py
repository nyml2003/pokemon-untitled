from __future__ import annotations

import tempfile
import unittest
from pathlib import Path, PureWindowsPath

from tools.pokemon_ops.application.documentation_service import DocumentationService
from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import LocalConfig, MirrorRoot, SourceRoot, TestSuite, WindowsRunner


def config_for(root: Path) -> LocalConfig:
    return LocalConfig(
        source_root=SourceRoot(root),
        mirror_root=MirrorRoot(root / "mirror", PureWindowsPath("C:/mirror")),
        windows_runner=WindowsRunner(Path("/mnt/c/Python/python.exe"), "tools.pokemon_ops.native_runner"),
        unit_suites={TestSuite.CORE: ("core",), TestSuite.WORLD: ("world",), TestSuite.ALL: ("workspace",)},
    )


class RecordingProcessRunner:
    def __init__(self) -> None:
        self.calls: list[tuple[tuple[str, ...], bool]] = []

    def run(self, arguments: tuple[str, ...], cwd: Path, forward_output: bool = False) -> Result[int]:
        self.calls.append((arguments, forward_output))
        return Result.ok(0)


class DocumentationServiceTests(unittest.TestCase):
    def test_check_extracts_mermaid_and_plantuml_to_temporary_sources(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            document = root / "docs" / "v2" / "current" / "001-主题" / "001-图示.md"
            document.parent.mkdir(parents=True)
            document.write_text(
                "# 图示\n\n```mermaid\nflowchart LR\n    A --> B\n```\n\n```plantuml\n@startuml\nA -> B\n@enduml\n```\n",
                encoding="utf-8",
            )
            runner = RecordingProcessRunner()

            result = DocumentationService(runner).check(config_for(root))

            self.assertTrue(result.is_ok)
            self.assertEqual(result.value, 2)
            self.assertEqual([call[0][0] for call in runner.calls], ["mmdc", "plantuml"])
            self.assertTrue(runner.calls[0][0][2].endswith(".mmd"))
            self.assertTrue(runner.calls[1][0][-1].endswith(".puml"))
            self.assertTrue(all(call[1] for call in runner.calls))

    def test_check_can_suppress_renderer_output(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            document = root / "docs" / "v2" / "current" / "001-主题" / "001-图示.md"
            document.parent.mkdir(parents=True)
            document.write_text("```mermaid\nflowchart LR\n    A --> B\n```\n", encoding="utf-8")
            runner = RecordingProcessRunner()

            result = DocumentationService(runner).check(config_for(root), forward_output=False)

            self.assertTrue(result.is_ok)
            self.assertEqual([call[1] for call in runner.calls], [False])

    def test_check_rejects_unterminated_diagram_block(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            document = root / "docs" / "v2" / "current" / "001-主题" / "001-图示.md"
            document.parent.mkdir(parents=True)
            document.write_text("```mermaid\nflowchart LR\n", encoding="utf-8")

            result = DocumentationService(RecordingProcessRunner()).check(config_for(root))

            self.assertFalse(result.is_ok)
            self.assertEqual(result.error.code.value, "DiagramValidationFailed")
