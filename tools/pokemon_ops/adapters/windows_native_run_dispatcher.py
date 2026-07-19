from __future__ import annotations

import json
import subprocess

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig, NativeOperation, NativeRunRequest


class WindowsNativeRunDispatcher:
    def __init__(self, config: LocalConfig) -> None:
        self._config = config

    def dispatch(self, request: NativeRunRequest) -> Result[int]:
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
        try:
            completed = subprocess.run(
                (str(executable), "-m", self._config.windows_runner.module),
                cwd=request.mirror_root.wsl_mount_path,
                input=payload,
                text=True,
                check=False,
            )
        except OSError as error:
            return Result.fail(ErrorCode.WINDOWS_RUNNER_UNAVAILABLE, "cannot start Windows private runner", reason=str(error))
        if completed.returncode != 0:
            code = ErrorCode.BUILD_FAILED if request.operation is NativeOperation.BUILD_GAME_HOST else ErrorCode.RUN_FAILED
            return Result.fail(code, "Windows private runner exited unsuccessfully", exit_code=str(completed.returncode))
        return Result.ok(completed.returncode)
