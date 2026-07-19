from __future__ import annotations

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import LocalConfig, TestSuite
from tools.pokemon_ops.ports.interfaces import ProcessRunner


TEST_COMMANDS: dict[str, tuple[str, ...]] = {
    "core": ("cargo", "test", "-p", "game-session"),
    "world": ("cargo", "test", "-p", "world-application"),
    "workspace": ("cargo", "test", "--workspace"),
}


class WslTestingService:
    def __init__(self, process_runner: ProcessRunner) -> None:
        self._process_runner = process_runner

    def format(self, config: LocalConfig, check: bool) -> Result[int]:
        command = ("cargo", "fmt", "--all") + (("--", "--check") if check else ())
        return self._process_runner.run(command, config.source_root.path, forward_output=True)

    def test(self, config: LocalConfig, suite: TestSuite) -> Result[int]:
        request_ids = config.unit_suites[suite]
        for request_id in request_ids:
            command = TEST_COMMANDS.get(request_id)
            if command is None:
                from tools.pokemon_ops.domain.errors import ErrorCode

                return Result.fail(ErrorCode.INVALID_CONFIGURATION, "unknown unit test request ID", request_id=request_id)
            result = self._process_runner.run(command, config.source_root.path, forward_output=True)
            if not result.is_ok:
                return result
        return Result.ok(0)
