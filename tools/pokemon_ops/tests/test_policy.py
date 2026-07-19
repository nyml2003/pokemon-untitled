from __future__ import annotations

import unittest
from pathlib import PurePosixPath

from tools.pokemon_ops.domain.policy import make_sync_plan


class SyncPlanTests(unittest.TestCase):
    def test_excludes_local_outputs_and_deletes_only_when_requested(self) -> None:
        source = {
            PurePosixPath("src/main.rs"),
            PurePosixPath("target/debug/game"),
            PurePosixPath("tools/pokemon_ops/__pycache__/policy.pyc"),
        }
        mirror = {PurePosixPath("src/main.rs"), PurePosixPath("assets/old.png"), PurePosixPath("target/debug/game")}

        without_delete = make_sync_plan(source, mirror, delete_removed=False)
        self.assertEqual(without_delete.copies, (PurePosixPath("src/main.rs"),))
        self.assertEqual(without_delete.deletes, ())

        with_delete = make_sync_plan(source, mirror, delete_removed=True)
        self.assertEqual(with_delete.deletes, (PurePosixPath("assets/old.png"),))
