from __future__ import annotations

import tempfile
import unittest
from pathlib import Path, PureWindowsPath

from tools.pokemon_ops.application.native_service import NativeService
from tools.pokemon_ops.application.testing_service import WslTestingService
from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import BuildProfile, GitSyncReport, LocalConfig, MirrorRoot, NativeOperation, ProgressEvent, SourceRoot, TestSuite, WindowsRunner


def config_for(root: Path) -> LocalConfig:
    source = root / "source"
    mirror = root / "mirror"
    source.mkdir()
    mirror.mkdir()
    return LocalConfig(
        source_root=SourceRoot(source),
        mirror_root=MirrorRoot(mirror, PureWindowsPath("C:/mirror")),
        windows_runner=WindowsRunner(Path("/mnt/c/Python/python.exe"), "tools.pokemon_ops.native_runner"),
        unit_suites={TestSuite.CORE: ("core",), TestSuite.WORLD: ("world",), TestSuite.ALL: ("workspace",)},
    )


class RecordingProcessRunner:
    def __init__(self) -> None:
        self.calls: list[tuple[str, ...]] = []

    def run(self, arguments: tuple[str, ...], cwd: Path, forward_output: bool = False) -> Result[int]:
        self.calls.append(arguments)
        return Result.ok(0)


class FailingSyncService:
    def sync(
        self,
        config: LocalConfig,
        progress: object = None,
    ) -> Result[object]:
        return Result.fail(ErrorCode.GIT_SYNC_FAILED, "Git sync failed")


class RecordingSyncService:
    def sync(self, config: LocalConfig, progress: object = None) -> Result[GitSyncReport]:
        return Result.ok(GitSyncReport("source", "remote", "before", "after", True))


class RecordingDispatcher:
    def __init__(self) -> None:
        self.called = False

    def dispatch(self, request: object, progress: object = None) -> Result[int]:
        self.called = True
        return Result.ok(0)


class RecordingProgress:
    def __init__(self) -> None:
        self.events: list[ProgressEvent] = []

    def report(self, event: ProgressEvent) -> None:
        self.events.append(event)


class OperationTests(unittest.TestCase):
    def test_all_unit_suite_uses_only_wsl_process_runner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            runner = RecordingProcessRunner()
            service = WslTestingService(runner)
            result = service.test(config_for(Path(directory)), TestSuite.ALL)
            self.assertTrue(result.is_ok)
            self.assertEqual(
                runner.calls,
                [("cargo", "test", "--workspace")],
            )

    def test_native_dispatch_does_not_run_after_sync_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            dispatcher = RecordingDispatcher()
            service = NativeService(FailingSyncService(), dispatcher)  # type: ignore[arg-type]
            result = service.execute(config_for(Path(directory)), NativeOperation.RUN_GAME_HOST, BuildProfile.DEBUG)
            self.assertFalse(result.is_ok)
            self.assertFalse(dispatcher.called)

    def test_native_operation_reports_windows_dispatch(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            config = config_for(root)
            dispatcher = RecordingDispatcher()
            service = NativeService(RecordingSyncService(), dispatcher)  # type: ignore[arg-type]
            progress = RecordingProgress()

            result = service.execute(
                config,
                NativeOperation.RUN_GAME_HOST,
                BuildProfile.DEBUG,
                progress=progress,
            )

            self.assertTrue(result.is_ok)
            self.assertTrue(any(event.stage == "run.start" for event in progress.events))
