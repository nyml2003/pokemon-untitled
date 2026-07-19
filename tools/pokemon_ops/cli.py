from __future__ import annotations

import argparse
import json
import sys
from dataclasses import asdict, is_dataclass
from enum import Enum
from pathlib import Path

from tools.pokemon_ops.adapters.local_config import load_local_config
from tools.pokemon_ops.adapters.local_git_mirror import LocalGitMirror
from tools.pokemon_ops.adapters.local_process_runner import LocalProcessRunner
from tools.pokemon_ops.adapters.windows_native_run_dispatcher import WindowsNativeRunDispatcher
from tools.pokemon_ops.application.native_service import NativeService
from tools.pokemon_ops.application.sync_service import SyncService
from tools.pokemon_ops.application.testing_service import WslTestingService
from tools.pokemon_ops.domain.errors import Result
from tools.pokemon_ops.domain.model import BuildProfile, NativeOperation, TestSuite


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="ops")
    parser.add_argument("--json", action="store_true", dest="json_output")
    commands = parser.add_subparsers(dest="command", required=True)
    command_parsers: list[argparse.ArgumentParser] = []
    command_parsers.append(commands.add_parser("init-mirror"))
    command_parsers.append(commands.add_parser("check"))
    command_parsers.append(commands.add_parser("doctor"))

    format_parser = commands.add_parser("format")
    command_parsers.append(format_parser)
    format_parser.add_argument("--check", action="store_true")

    test_parser = commands.add_parser("test")
    command_parsers.append(test_parser)
    test_parser.add_argument("--suite", choices=[suite.value for suite in TestSuite], default=TestSuite.ALL.value)

    sync_parser = commands.add_parser("sync")
    command_parsers.append(sync_parser)

    for name in ("build", "run"):
        native_parser = commands.add_parser(name)
        command_parsers.append(native_parser)
        native_parser.add_argument("target", choices=["game-host"])
        native_parser.add_argument("--profile", choices=[profile.value for profile in BuildProfile], default=BuildProfile.DEBUG.value)
    for command_parser in command_parsers:
        command_parser.add_argument("--json", action="store_true", dest="json_output", default=argparse.SUPPRESS)
    return parser


def _json_default(value: object) -> object:
    if is_dataclass(value):
        return asdict(value)
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, Path):
        return str(value)
    return str(value)


def _emit(result: Result[object], json_output: bool) -> int:
    if result.is_ok:
        payload = {"ok": True, "result": result.value}
        if json_output:
            print(json.dumps(payload, default=_json_default))
        else:
            print("ok")
        return 0
    assert result.error is not None
    payload = {"ok": False, "error": {"code": result.error.code.value, "message": result.error.message, "details": result.error.details}}
    if json_output:
        print(json.dumps(payload))
    else:
        print(f"{result.error.code.value}: {result.error.message}", file=sys.stderr)
        for key, value in result.error.details.items():
            print(f"  {key}: {value}", file=sys.stderr)
    exit_code = result.error.details.get("exit_code")
    return int(exit_code) if exit_code and exit_code.isdigit() else 1


def _load(source_root: Path) -> Result[object]:
    return load_local_config(source_root)


def _report_progress(message: str) -> None:
    print(f"ops: {message}", file=sys.stderr, flush=True)


def run(arguments: list[str] | None = None, source_root: Path | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(arguments)
    if not sys.platform.startswith("linux"):
        from tools.pokemon_ops.domain.errors import ErrorCode

        return _emit(Result.fail(ErrorCode.UNSUPPORTED_HOST, "ops CLI is only available from WSL"), args.json_output)

    root = (source_root or Path.cwd()).resolve()
    loaded = _load(root)
    if not loaded.is_ok:
        return _emit(loaded, args.json_output)
    config = loaded.value
    assert config is not None

    sync_service = SyncService(LocalGitMirror())
    if args.command == "init-mirror":
        return _emit(sync_service.initialize(config, _report_progress), args.json_output)
    if args.command == "check":
        return _emit(sync_service.check(config), args.json_output)
    if args.command == "doctor":
        runner_module = root.joinpath(*config.windows_runner.module.split(".")).with_suffix(".py")
        diagnostics = {
            "source_root": str(config.source_root.path),
            "mirror_root": str(config.mirror_root.wsl_mount_path),
            "windows_python": str(config.windows_runner.python_executable),
            "runner_module": str(runner_module),
        }
        if not config.source_root.path.is_dir():
            from tools.pokemon_ops.domain.errors import ErrorCode

            return _emit(Result.fail(ErrorCode.SOURCE_MISSING, "source root does not exist", path=str(config.source_root.path)), args.json_output)
        mirror_status = sync_service.check(config)
        if not mirror_status.is_ok:
            return _emit(mirror_status, args.json_output)
        if not config.windows_runner.python_executable.is_file():
            from tools.pokemon_ops.domain.errors import ErrorCode

            return _emit(
                Result.fail(ErrorCode.WINDOWS_PYTHON_UNAVAILABLE, "configured Windows Python executable is unavailable", **diagnostics),
                args.json_output,
            )
        if not runner_module.is_file():
            from tools.pokemon_ops.domain.errors import ErrorCode

            return _emit(
                Result.fail(ErrorCode.WINDOWS_RUNNER_UNAVAILABLE, "private runner module is unavailable", **diagnostics),
                args.json_output,
            )
        diagnostics.update({"source_head": mirror_status.value.source_head, "remote_head": mirror_status.value.remote_head, "mirror_head": mirror_status.value.mirror_head})
        return _emit(Result.ok(diagnostics), args.json_output)

    testing = WslTestingService(LocalProcessRunner())
    if args.command == "format":
        return _emit(testing.format(config, args.check), args.json_output)
    if args.command == "test":
        return _emit(testing.test(config, TestSuite(args.suite)), args.json_output)
    if args.command == "sync":
        return _emit(sync_service.sync(config, _report_progress), args.json_output)

    operation = NativeOperation.BUILD_GAME_HOST if args.command == "build" else NativeOperation.RUN_GAME_HOST
    native = NativeService(sync_service, WindowsNativeRunDispatcher(config))
    return _emit(native.execute(config, operation, BuildProfile(args.profile), _report_progress), args.json_output)
