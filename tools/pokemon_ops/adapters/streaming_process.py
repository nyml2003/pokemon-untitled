from __future__ import annotations

import queue
import re
import subprocess
import threading
import time
from collections import deque
from dataclasses import dataclass
from pathlib import Path
from typing import TextIO

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import ProcessStream, ProgressEvent, ProgressEventType
from tools.pokemon_ops.ports.interfaces import ProgressReporter

_AUTHORITY_CREDENTIALS = re.compile(r"(https?://)([^\s/@]+)@")
_SECRET_QUERY = re.compile(r"([?&](?:access_token|password|token)=[^&\s]+)", re.IGNORECASE)


@dataclass(frozen=True)
class StreamedProcessResult:
    exit_code: int
    output_tail: tuple[str, ...]


def run_streamed_process(
    arguments: tuple[str, ...],
    cwd: Path,
    progress: ProgressReporter | None,
    stage: str,
    *,
    input_text: str | None = None,
    heartbeat_seconds: float = 15.0,
    unavailable_code: ErrorCode,
    start_failure_code: ErrorCode,
) -> Result[StreamedProcessResult]:
    try:
        process = subprocess.Popen(
            arguments,
            cwd=cwd,
            stdin=subprocess.PIPE if input_text is not None else None,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
    except FileNotFoundError:
        return Result.fail(unavailable_code, "required executable is unavailable", executable=arguments[0])
    except OSError as error:
        return Result.fail(start_failure_code, "cannot start process", executable=arguments[0], reason=str(error))

    if input_text is not None and process.stdin is not None:
        try:
            process.stdin.write(input_text)
            process.stdin.close()
        except BrokenPipeError:
            pass

    lines: queue.Queue[tuple[ProcessStream, str] | None] = queue.Queue()
    tail: deque[str] = deque(maxlen=40)
    readers = [
        threading.Thread(target=_read_lines, args=(process.stdout, ProcessStream.STDOUT, lines), daemon=True),
        threading.Thread(target=_read_lines, args=(process.stderr, ProcessStream.STDERR, lines), daemon=True),
    ]
    for reader in readers:
        reader.start()

    remaining = len(readers)
    started = time.monotonic()
    cancelled = False
    cancellation_deadline: float | None = None
    while remaining:
        try:
            try:
                item = lines.get(timeout=heartbeat_seconds)
            except queue.Empty:
                if cancellation_deadline is not None and time.monotonic() >= cancellation_deadline and process.poll() is None:
                    process.kill()
                    cancellation_deadline = None
                if progress is not None:
                    elapsed = int(time.monotonic() - started)
                    progress.report(ProgressEvent(ProgressEventType.PROGRESS, stage, f"still waiting ({elapsed}s)"))
                continue
            if item is None:
                remaining -= 1
                continue
            stream, line = item
            safe_line = _redact(line)
            tail.append(f"{stream.value}: {safe_line}")
            if progress is not None:
                progress.report(ProgressEvent(ProgressEventType.OUTPUT, stage, safe_line, stream=stream))
        except KeyboardInterrupt:
            if not cancelled:
                cancelled = True
                process.terminate()
                cancellation_deadline = time.monotonic() + 5

    exit_code = process.wait()
    if cancelled:
        if progress is not None:
            progress.report(ProgressEvent(ProgressEventType.ERROR, stage, "operation was cancelled", code=ErrorCode.CANCELLED.value))
        return Result.fail(ErrorCode.CANCELLED, "operation was cancelled", stage=stage)
    return Result.ok(StreamedProcessResult(exit_code=exit_code, output_tail=tuple(tail)))


def _read_lines(stream: TextIO | None, stream_name: ProcessStream, lines: queue.Queue[tuple[ProcessStream, str] | None]) -> None:
    if stream is None:
        lines.put(None)
        return
    try:
        while True:
            line = stream.readline()
            if line == "":
                return
            message = line.rstrip("\r\n")
            if message:
                lines.put((stream_name, message))
    finally:
        stream.close()
        lines.put(None)


def _redact(message: str) -> str:
    without_credentials = _AUTHORITY_CREDENTIALS.sub(r"\1***@", message)
    return _SECRET_QUERY.sub(lambda match: match.group(1).split("=", 1)[0] + "=***", without_credentials)
