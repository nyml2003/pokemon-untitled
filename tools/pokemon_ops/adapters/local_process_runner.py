from __future__ import annotations

import subprocess
from pathlib import Path

from tools.pokemon_ops.domain.errors import ErrorCode, Result


class LocalProcessRunner:
    def run(self, arguments: tuple[str, ...], cwd: Path, forward_output: bool = False) -> Result[int]:
        try:
            completed = subprocess.run(
                arguments,
                cwd=cwd,
                check=False,
                stdout=None if forward_output else subprocess.PIPE,
                stderr=None if forward_output else subprocess.PIPE,
            )
        except FileNotFoundError:
            return Result.fail(ErrorCode.PROCESS_FAILED, "required executable is unavailable", executable=arguments[0])
        except OSError as error:
            return Result.fail(ErrorCode.PROCESS_FAILED, "cannot start process", reason=str(error))
        if completed.returncode != 0:
            return Result.fail(
                ErrorCode.PROCESS_FAILED,
                "process exited unsuccessfully",
                executable=arguments[0],
                exit_code=str(completed.returncode),
            )
        return Result.ok(completed.returncode)
