from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.pokemon_ops.adapters import streaming_process
from tools.pokemon_ops.adapters.streaming_process import run_streamed_process
from tools.pokemon_ops.domain.errors import ErrorCode
from tools.pokemon_ops.domain.model import ProgressEvent


class RecordingProgress:
    def __init__(self) -> None:
        self.events: list[ProgressEvent] = []

    def report(self, event: ProgressEvent) -> None:
        self.events.append(event)


class StreamingProcessTests(unittest.TestCase):
    def test_forwards_both_streams_and_reports_a_heartbeat(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            progress = RecordingProgress()
            result = run_streamed_process(
                (sys.executable, "-c", "import sys, time; print('out', flush=True); print('err', file=sys.stderr, flush=True); time.sleep(0.03)"),
                Path(directory),
                progress,
                "test.stream",
                heartbeat_seconds=0.01,
                unavailable_code=ErrorCode.PROCESS_FAILED,
                start_failure_code=ErrorCode.PROCESS_FAILED,
            )

        self.assertTrue(result.is_ok)
        self.assertEqual(result.value.exit_code, 0)  # type: ignore[union-attr]
        self.assertTrue(any(event.type.value == "output" and event.stream and event.stream.value == "stdout" for event in progress.events))
        self.assertTrue(any(event.type.value == "output" and event.stream and event.stream.value == "stderr" for event in progress.events))
        self.assertTrue(any(event.message.startswith("still waiting") for event in progress.events))

    def test_redacts_credentials_from_forwarded_output_and_tail(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            progress = RecordingProgress()
            result = run_streamed_process(
                (sys.executable, "-c", "print('https://user:secret@example.test/path?token=secret')"),
                Path(directory),
                progress,
                "test.redaction",
                unavailable_code=ErrorCode.PROCESS_FAILED,
                start_failure_code=ErrorCode.PROCESS_FAILED,
            )

        self.assertTrue(result.is_ok)
        self.assertEqual(progress.events[0].message, "https://***@example.test/path?token=***")
        self.assertEqual(result.value.output_tail, ("stdout: https://***@example.test/path?token=***",))  # type: ignore[union-attr]

    def test_interrupt_terminates_the_process_and_returns_cancelled(self) -> None:
        original_get = streaming_process.queue.Queue.get
        interrupted = False

        def interrupt_once(queue: object, *args: object, **kwargs: object) -> object:
            nonlocal interrupted
            if not interrupted:
                interrupted = True
                raise KeyboardInterrupt
            return original_get(queue, *args, **kwargs)

        with tempfile.TemporaryDirectory() as directory, patch.object(streaming_process.queue.Queue, "get", interrupt_once):
            progress = RecordingProgress()
            result = run_streamed_process(
                (sys.executable, "-c", "import time; time.sleep(10)"),
                Path(directory),
                progress,
                "test.cancel",
                unavailable_code=ErrorCode.PROCESS_FAILED,
                start_failure_code=ErrorCode.PROCESS_FAILED,
            )

        self.assertFalse(result.is_ok)
        self.assertEqual(result.error.code, ErrorCode.CANCELLED)  # type: ignore[union-attr]
        self.assertTrue(any(event.code == ErrorCode.CANCELLED.value for event in progress.events))
