from __future__ import annotations

from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import LocalConfig, TestSuite
from tools.pokemon_ops.ports.interfaces import ProcessRunner


TEST_COMMANDS: dict[str, tuple[str, ...]] = {
    "core": ("cargo", "test", "-p", "game-session"),
    "world": ("cargo", "test", "-p", "world-application"),
    "workspace": ("cargo", "test", "--workspace"),
}

PANIC_PREVENTION_LINTS = (
    "-D",
    "clippy::unwrap_used",
    "-D",
    "clippy::expect_used",
    "-D",
    "clippy::panic",
    "-D",
    "clippy::todo",
    "-D",
    "clippy::unimplemented",
    "-D",
    "clippy::unreachable",
    "-D",
    "clippy::string_slice",
)

DEFAULT_LINT_COMMAND = ("cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings")
PRODUCTION_LINT_COMMAND = ("cargo", "clippy", "--workspace", "--lib", "--bins", "--", "-D", "warnings", *PANIC_PREVENTION_LINTS)


class WslTestingService:
    def __init__(self, process_runner: ProcessRunner) -> None:
        self._process_runner = process_runner

    def format(self, config: LocalConfig, check: bool) -> Result[int]:
        command = ("cargo", "fmt", "--all") + (("--", "--check") if check else ())
        return self._process_runner.run(command, config.source_root.path, forward_output=True)

    def lint(self, config: LocalConfig) -> Result[int]:
        default_lint = self._process_runner.run(DEFAULT_LINT_COMMAND, config.source_root.path, forward_output=True)
        if not default_lint.is_ok:
            return default_lint
        return self._process_runner.run(PRODUCTION_LINT_COMMAND, config.source_root.path, forward_output=True)

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
