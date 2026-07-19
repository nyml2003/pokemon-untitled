from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from tools.pokemon_ops.domain.config import parse_local_config
from tools.pokemon_ops.domain.errors import ErrorCode
from tools.pokemon_ops.domain.model import TestSuite


def valid_data(mirror: Path) -> dict[str, object]:
    return {
        "mirror": {"wsl_mount_root": str(mirror), "windows_root": "C:\\mirror"},
        "windows_runner": {"python_executable": "/mnt/c/Python/python.exe", "module": "tools.pokemon_ops.native_runner"},
        "unit_suites": {"core": ["core"], "world": ["world"], "all": ["workspace"]},
    }


class LocalConfigTests(unittest.TestCase):
    def test_parses_distinct_source_and_mirror_roots(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            mirror = root / "mirror"
            source.mkdir()
            mirror.mkdir()
            parsed = parse_local_config(valid_data(mirror), source)
            self.assertTrue(parsed.is_ok)
            assert parsed.value is not None
            self.assertEqual(set(parsed.value.unit_suites), set(TestSuite))

    def test_rejects_overlapping_roots(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            source.mkdir()
            parsed = parse_local_config(valid_data(source / "mirror"), source)
            self.assertFalse(parsed.is_ok)
            assert parsed.error is not None
            self.assertEqual(parsed.error.code, ErrorCode.INVALID_CONFIGURATION)
