from __future__ import annotations

import tempfile
from dataclasses import dataclass
from pathlib import Path

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig
from tools.pokemon_ops.ports.interfaces import ProcessRunner


@dataclass(frozen=True)
class DiagramSource:
    document: Path
    index: int
    language: str
    content: str


class DocumentationService:
    def __init__(self, process_runner: ProcessRunner) -> None:
        self._process_runner = process_runner

    def check(self, config: LocalConfig, forward_output: bool = True) -> Result[int]:
        docs_root = config.source_root.path / "docs" / "v2"
        if not docs_root.is_dir():
            return Result.fail(ErrorCode.SOURCE_MISSING, "documentation root does not exist", path=str(docs_root))

        diagrams: list[DiagramSource] = []
        for document in sorted(docs_root.rglob("*.md")):
            parsed = self._extract_diagrams(document)
            if not parsed.is_ok:
                return parsed
            diagrams.extend(parsed.value or ())

        with tempfile.TemporaryDirectory(prefix="pokemon-docs-diagrams-") as directory:
            temporary_root = Path(directory)
            for diagram in diagrams:
                source = self._write_source(diagram, docs_root, temporary_root)
                rendered = self._render(diagram, source, docs_root, forward_output)
                if not rendered.is_ok:
                    return rendered

        return Result.ok(len(diagrams))

    def _extract_diagrams(self, document: Path) -> Result[tuple[DiagramSource, ...]]:
        diagrams: list[DiagramSource] = []
        active_language: str | None = None
        active_lines: list[str] = []

        for line_number, line in enumerate(document.read_text(encoding="utf-8").splitlines(), start=1):
            if line in ("```mermaid", "```plantuml"):
                if active_language is not None:
                    return Result.fail(
                        ErrorCode.DIAGRAM_VALIDATION_FAILED,
                        "nested diagram block",
                        document=str(document),
                        line=str(line_number),
                    )
                active_language = line.removeprefix("```")
                active_lines = []
                continue
            if line == "```" and active_language is not None:
                diagrams.append(DiagramSource(document, len(diagrams) + 1, active_language, "\n".join(active_lines) + "\n"))
                active_language = None
                active_lines = []
                continue
            if active_language is not None:
                active_lines.append(line)

        if active_language is not None:
            return Result.fail(
                ErrorCode.DIAGRAM_VALIDATION_FAILED,
                "unterminated diagram block",
                document=str(document),
            )
        return Result.ok(tuple(diagrams))

    def _write_source(self, diagram: DiagramSource, docs_root: Path, temporary_root: Path) -> Path:
        document_path = diagram.document.relative_to(docs_root).with_suffix("")
        output_dir = temporary_root / document_path
        output_dir.mkdir(parents=True, exist_ok=True)
        extension = "mmd" if diagram.language == "mermaid" else "puml"
        source = output_dir / f"{diagram.index:03d}.{extension}"
        source.write_text(diagram.content, encoding="utf-8")
        return source

    def _render(self, diagram: DiagramSource, source: Path, docs_root: Path, forward_output: bool) -> Result[int]:
        relative_document = str(diagram.document.relative_to(docs_root))
        if diagram.language == "mermaid":
            command = ("mmdc", "-i", str(source), "-o", str(source.with_suffix(".png")))
        else:
            command = ("plantuml", "-tpng", "-o", str(source.parent), str(source))
        result = self._process_runner.run(command, docs_root, forward_output=forward_output)
        if result.is_ok:
            return Result.ok(0)
        return Result.fail(
            ErrorCode.DIAGRAM_VALIDATION_FAILED,
            "diagram renderer failed",
            document=relative_document,
            diagram=str(diagram.index),
            renderer=command[0],
        )
