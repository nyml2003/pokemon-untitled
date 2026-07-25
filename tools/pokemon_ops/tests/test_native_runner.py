from __future__ import annotations

import io
import json
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools.pokemon_ops.native_runner import main


class NativeRunnerTests(unittest.TestCase):
    def test_page_demo_run_forwards_only_the_registered_argument_shape(self) -> None:
        request = {
            "operation": "run_game_page_demo",
            "profile": "debug",
            "windows_root": "C:\\mirror",
            "demo": "shop-potion-preview",
        }
        with (
            patch("sys.stdin", io.StringIO(json.dumps(request))),
            patch(
                "tools.pokemon_ops.native_runner.subprocess.run",
                return_value=SimpleNamespace(returncode=0),
            ) as run,
        ):
            result = main()

        self.assertEqual(result, 0)
        run.assert_called_once_with(
            [
                "cargo",
                "run",
                "--bin",
                "game-page-demo",
                "--",
                "--page-demo",
                "shop-potion-preview",
            ],
            cwd=Path("C:\\mirror"),
            check=False,
        )

    def test_build_rejects_a_demo_argument(self) -> None:
        request = {
            "operation": "build_game_page_demo",
            "profile": "debug",
            "windows_root": "C:\\mirror",
            "demo": "shop-potion-preview",
        }
        with (
            patch("sys.stdin", io.StringIO(json.dumps(request))),
            patch("tools.pokemon_ops.native_runner.subprocess.run") as run,
        ):
            result = main()

        self.assertEqual(result, 2)
        run.assert_not_called()
