"""Enforce 100% production-line coverage for stateless, side-effect-free crates."""

from __future__ import annotations

import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent
REPORT_PATH = ROOT / "target" / "pure-coverage.lcov"
PURE_CRATES = (
    "crates/foundation/punctum-grid",
    "crates/foundation/punctum-input",
    "crates/foundation/punctum-terminal",
    "crates/foundation/punctum-ui",
    "crates/domain/narrative-token",
    "crates/domain/narrative-cps",
    "crates/domain/narrative-compiler",
    "crates/presentation/game-asset-plan",
)


@dataclass(frozen=True)
class CoverageFailure:
    path: Path
    covered: int
    total: int


def main() -> int:
    run_coverage()
    failures = uncovered_pure_files(load_report())
    if not failures:
        print("pure crate production-line coverage: 100%")
        return 0
    for failure in failures:
        print(
            f"{failure.path.relative_to(ROOT)}: "
            f"{failure.covered}/{failure.total} lines covered",
            file=sys.stderr,
        )
    return 1


def run_coverage() -> None:
    subprocess.run(
        [
            "cargo",
            "llvm-cov",
            "--workspace",
            "--lcov",
            "--output-path",
            str(REPORT_PATH),
        ],
        cwd=ROOT,
        check=True,
    )


def load_report() -> dict[Path, list[tuple[int, int]]]:
    coverage: dict[Path, list[tuple[int, int]]] = {}
    current_path: Path | None = None
    with REPORT_PATH.open(encoding="utf-8") as report:
        for line in report:
            line = line.rstrip("\n")
            if line.startswith("SF:"):
                current_path = Path(line.removeprefix("SF:"))
                coverage[current_path] = []
            elif line.startswith("DA:") and current_path is not None:
                line_number, count = line.removeprefix("DA:").split(",", maxsplit=1)
                coverage[current_path].append((int(line_number), int(count)))
            elif line == "end_of_record":
                current_path = None
    return coverage


def uncovered_pure_files(
    report: dict[Path, list[tuple[int, int]]],
) -> list[CoverageFailure]:
    pure_roots = tuple(ROOT / crate for crate in PURE_CRATES)
    failures: list[CoverageFailure] = []
    for path, line_counts in report.items():
        if not path.suffix == ".rs" or not any(
            path.is_relative_to(root) for root in pure_roots
        ):
            continue
        production_counts = [
            count for line_number, count in line_counts if line_number < first_test_line(path)
        ]
        covered = sum(count > 0 for count in production_counts)
        total = len(production_counts)
        if covered != total:
            failures.append(
                CoverageFailure(path, covered, total)
            )
    return failures


def first_test_line(path: Path) -> int:
    with path.open(encoding="utf-8") as source:
        for line_number, line in enumerate(source, start=1):
            if line == "#[cfg(test)]\n":
                return line_number - 1
    return sys.maxsize


if __name__ == "__main__":
    raise SystemExit(main())
