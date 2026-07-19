from __future__ import annotations

import io
import json
import subprocess
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

from tools.pokemon_ops.cli import build_parser, run


def git(cwd: Path, *arguments: str) -> None:
    subprocess.run(("git", *arguments), cwd=cwd, check=True, text=True, capture_output=True)


def write_config(root: Path, mirror: Path) -> None:
    (root / "ops.local.json").write_text(
        json.dumps(
            {
                "mirror": {
                    "wsl_mount_root": str(mirror),
                    "windows_root": "C:\\mirror",
                    "remote": "origin",
                    "branch": "master",
                },
                "windows_runner": {
                    "python_executable": "/mnt/c/Python/python.exe",
                    "module": "tools.pokemon_ops.native_runner",
                },
                "unit_suites": {"core": ["core"], "world": ["world"], "all": ["workspace"]},
            }
        ),
        encoding="utf-8",
    )


def initialize_source(root: Path) -> Path:
    remote = root.parent / f"{root.name}-remote.git"
    subprocess.run(("git", "init", "--bare", str(remote)), check=True, text=True, capture_output=True)
    git(root, "init")
    git(root, "config", "user.email", "ops@example.invalid")
    git(root, "config", "user.name", "Pokemon Ops")
    (root / "tracked.txt").write_text("initial\n", encoding="utf-8")
    git(root, "add", "tracked.txt")
    git(root, "commit", "-m", "initial")
    git(root, "branch", "-M", "master")
    git(root, "remote", "add", "origin", str(remote))
    git(root, "push", "-u", "origin", "master")
    return remote


class CliTests(unittest.TestCase):
    def test_lint_command_is_available_with_json_output(self) -> None:
        arguments = build_parser().parse_args(["lint", "--json"])

        self.assertEqual(arguments.command, "lint")
        self.assertTrue(arguments.json_output)

    def test_lines_and_coverage_commands_are_available_with_json_output(self) -> None:
        lines = build_parser().parse_args(["lines", "--json"])
        coverage = build_parser().parse_args(["coverage", "--json"])

        self.assertEqual(lines.command, "lines")
        self.assertTrue(lines.json_output)
        self.assertEqual(coverage.command, "coverage")
        self.assertTrue(coverage.json_output)

    def test_docs_check_command_is_available_with_json_output(self) -> None:
        arguments = build_parser().parse_args(["docs", "check", "--json"])

        self.assertEqual(arguments.command, "docs")
        self.assertEqual(arguments.docs_command, "check")
        self.assertTrue(arguments.json_output)

    def test_init_mirror_creates_git_clone_and_preserves_json_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mirror = root.parent / f"{root.name}-mirror"
            initialize_source(root)
            write_config(root, mirror)
            output = io.StringIO()
            errors = io.StringIO()

            with redirect_stdout(output), redirect_stderr(errors):
                exit_code = run(["init-mirror", "--json"], source_root=root)

            self.assertEqual(exit_code, 0)
            self.assertTrue(json.loads(output.getvalue())["ok"])
            self.assertTrue((mirror / ".git").is_dir())
            events = [json.loads(line) for line in errors.getvalue().splitlines()]
            self.assertTrue(any(event["type"] == "progress" and event["stage"] == "mirror.clone" for event in events))

    def test_sync_reports_git_progress_to_stderr_without_breaking_json_stdout(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mirror = root.parent / f"{root.name}-mirror"
            initialize_source(root)
            write_config(root, mirror)
            run(["init-mirror"], source_root=root)
            output = io.StringIO()
            errors = io.StringIO()

            with redirect_stdout(output), redirect_stderr(errors):
                exit_code = run(["sync", "--json"], source_root=root)

            self.assertEqual(exit_code, 0)
            self.assertTrue(json.loads(output.getvalue())["ok"])
            events = [json.loads(line) for line in errors.getvalue().splitlines()]
            self.assertTrue(any(event["type"] == "progress" and event["stage"] == "sync.fetch" for event in events))
            self.assertTrue(any(event["type"] == "output" and event["stream"] == "stderr" for event in events))
