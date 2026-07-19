from __future__ import annotations

import io
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

from tools.pokemon_ops.cli import run


class CliTests(unittest.TestCase):
    def test_check_returns_structured_dry_run_result(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mirror = root.parent / f"{root.name}-mirror"
            mirror.mkdir()
            (root / "ops.local.json").write_text(
                json.dumps(
                    {
                        "mirror": {"wsl_mount_root": str(mirror), "windows_root": "C:\\mirror"},
                        "windows_runner": {
                            "python_executable": "/mnt/c/Python/python.exe",
                            "module": "tools.pokemon_ops.native_runner",
                        },
                        "unit_suites": {"core": ["core"], "world": ["world"], "all": ["workspace"]},
                    }
                ),
                encoding="utf-8",
            )
            output = io.StringIO()
            with redirect_stdout(output):
                exit_code = run(["check", "--json"], source_root=root)
            self.assertEqual(exit_code, 0)
            payload = json.loads(output.getvalue())
            self.assertTrue(payload["ok"])
            self.assertFalse(payload["result"]["applied"])
