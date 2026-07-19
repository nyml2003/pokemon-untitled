from __future__ import annotations

import json
import sys
from datetime import UTC, datetime
from typing import TextIO

from tools.pokemon_ops.domain.model import ProgressEvent


class TextProgressReporter:
    def __init__(self, output: TextIO | None = None) -> None:
        self._output = output or sys.stderr

    def report(self, event: ProgressEvent) -> None:
        timestamp = datetime.now().strftime("%H:%M:%S")
        print(f"[{timestamp}] {event.stage:<20} {event.message}", file=self._output, flush=True)


class JsonLinesProgressReporter:
    def __init__(self, output: TextIO | None = None) -> None:
        self._output = output or sys.stderr

    def report(self, event: ProgressEvent) -> None:
        payload = {
            "type": event.type.value,
            "timestamp": datetime.now(UTC).isoformat().replace("+00:00", "Z"),
            "stage": event.stage,
            "message": event.message,
        }
        if event.stream is not None:
            payload["stream"] = event.stream.value
        if event.code is not None:
            payload["code"] = event.code
        if event.remediation is not None:
            payload["remediation"] = event.remediation
        print(json.dumps(payload), file=self._output, flush=True)
