from __future__ import annotations

import json

from tools.pokemon_ops.adapters.streaming_process import run_streamed_process
from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig, NativeOperation, NativeRunRequest, ProgressEvent, ProgressEventType
from tools.pokemon_ops.ports.interfaces import ProgressReporter


class WindowsNativeRunDispatcher:
    def __init__(self, config: LocalConfig) -> None:
        self._config = config

    def dispatch(self, request: NativeRunRequest, progress: ProgressReporter | None = None) -> Result[int]:
        executable = self._config.windows_runner.python_executable
        if not executable.is_file():
            return Result.fail(
                ErrorCode.WINDOWS_PYTHON_UNAVAILABLE,
                "configured Windows Python executable is unavailable",
                path=str(executable),
            )
        module_path = request.mirror_root.wsl_mount_path.joinpath(*self._config.windows_runner.module.split(".")).with_suffix(".py")
        if not module_path.is_file():
            return Result.fail(
                ErrorCode.WINDOWS_RUNNER_UNAVAILABLE,
                "Windows private runner module is unavailable in the mirror",
                path=str(module_path),
            )
        payload = json.dumps(
            {
                "operation": request.operation.value,
                "profile": request.profile.value,
                "windows_root": str(request.mirror_root.windows_path),
            }
        )
        stage = "build.output" if request.operation.is_build else "run.output"
        completed = run_streamed_process(
            (str(executable), "-m", self._config.windows_runner.module),
            request.mirror_root.wsl_mount_path,
            progress,
            stage,
            input_text=payload,
            unavailable_code=ErrorCode.WINDOWS_RUNNER_UNAVAILABLE,
            start_failure_code=ErrorCode.WINDOWS_RUNNER_UNAVAILABLE,
        )
        if not completed.is_ok:
            assert completed.error is not None
            if progress is not None:
                progress.report(ProgressEvent(ProgressEventType.ERROR, stage, completed.error.message, code=completed.error.code.value))
            return Result(error=completed.error)
        assert completed.value is not None
        if completed.value.exit_code != 0:
            code = ErrorCode.BUILD_FAILED if request.operation.is_build else ErrorCode.RUN_FAILED
            if progress is not None:
                progress.report(ProgressEvent(ProgressEventType.ERROR, stage, "Windows private runner exited unsuccessfully", code=code.value))
            details = {"exit_code": str(completed.value.exit_code)}
            if completed.value.output_tail:
                details["output_tail"] = "\n".join(completed.value.output_tail)
            return Result.fail(code, "Windows private runner exited unsuccessfully", **details)
        exit_stage = "build.done" if request.operation.is_build else "run.exit"
        if progress is not None:
            progress.report(ProgressEvent(ProgressEventType.PROGRESS, exit_stage, f"Windows private runner exited with code {completed.value.exit_code}"))
        return Result.ok(completed.value.exit_code)
