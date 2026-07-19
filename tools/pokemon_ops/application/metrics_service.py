from __future__ import annotations

import json
from dataclasses import dataclass

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig
from tools.pokemon_ops.ports.interfaces import OutputProcessRunner


TOKEI_COMMAND_PREFIX = ("tokei", "--output", "json", "--types", "Rust")


@dataclass(frozen=True)
class RustLineCounts:
    files: int
    code: int
    comments: int
    blanks: int

    @property
    def total(self) -> int:
        return self.code + self.comments + self.blanks

    def add(self, other: "RustLineCounts") -> "RustLineCounts":
        return RustLineCounts(
            files=self.files + other.files,
            code=self.code + other.code,
            comments=self.comments + other.comments,
            blanks=self.blanks + other.blanks,
        )


@dataclass(frozen=True)
class RustLineReport:
    production: RustLineCounts
    tests: RustLineCounts

    @property
    def total(self) -> RustLineCounts:
        return self.production.add(self.tests)

    def render(self) -> str:
        rows = (
            ("production", self.production),
            ("tests", self.tests),
            ("total", self.total),
        )
        header = "scope       files    code  comments   blanks   total"
        body = "\n".join(
            f"{scope:<10}{counts.files:>5}{counts.code:>8}{counts.comments:>10}{counts.blanks:>9}{counts.total:>8}"
            for scope, counts in rows
        )
        return f"tokei Rust summary\n{header}\n{body}"


class WorkspaceMetricsService:
    def __init__(self, process_runner: OutputProcessRunner) -> None:
        self._process_runner = process_runner

    def lines(self, config: LocalConfig) -> Result[RustLineReport]:
        production = self._count(config, "src")
        if not production.is_ok:
            return Result(value=None, error=production.error)
        tests = self._count(config, "tests")
        if not tests.is_ok:
            return Result(value=None, error=tests.error)
        return Result.ok(RustLineReport(production=production.value, tests=tests.value))

    def _count(self, config: LocalConfig, directory_name: str) -> Result[RustLineCounts]:
        directories = sorted(config.source_root.path.glob(f"crates/**/{directory_name}"))
        command = (*TOKEI_COMMAND_PREFIX, *(str(directory) for directory in directories))
        output = self._process_runner.capture(command, config.source_root.path)
        if not output.is_ok:
            return Result(value=None, error=output.error)
        try:
            report = json.loads(output.value or "{}")
            rust = report["Rust"]
            reports = rust["reports"]
            return Result.ok(
                RustLineCounts(
                    files=len(reports),
                    code=int(rust["code"]),
                    comments=int(rust["comments"]),
                    blanks=int(rust["blanks"]),
                )
            )
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            return Result.fail(ErrorCode.METRICS_FAILED, "tokei returned an invalid Rust report", reason=str(error))
