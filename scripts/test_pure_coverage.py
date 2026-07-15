"""Enforce 100% production-line coverage for stateless, side-effect-free crates."""

from __future__ import annotations

import json
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parent.parent
REPORT_PATH = ROOT / "target" / "pure-coverage.json"
PURE_CRATES = (
    "crates/foundation/punctum-grid",
    "crates/foundation/punctum-input",
    "crates/foundation/punctum-terminal",
    "crates/foundation/punctum-ui",
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
            "--json",
            "--summary-only",
            "--output-path",
            str(REPORT_PATH),
        ],
        cwd=ROOT,
        check=True,
    )


def load_report() -> dict[str, Any]:
    with REPORT_PATH.open(encoding="utf-8") as report:
        return json.load(report)


def uncovered_pure_files(report: dict[str, Any]) -> list[CoverageFailure]:
    pure_roots = tuple(ROOT / crate for crate in PURE_CRATES)
    failures: list[CoverageFailure] = []
    for file in report["data"][0]["files"]:
        path = Path(file["filename"])
        if not path.suffix == ".rs" or not any(
            path.is_relative_to(root) for root in pure_roots
        ):
            continue
        lines = file["summary"]["lines"]
        if lines["covered"] != lines["count"]:
            failures.append(
                CoverageFailure(path, lines["covered"], lines["count"])
            )
    return failures


if __name__ == "__main__":
    raise SystemExit(main())
