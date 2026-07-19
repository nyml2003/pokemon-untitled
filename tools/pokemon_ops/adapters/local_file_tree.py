from __future__ import annotations

import json
import os
import shutil
from pathlib import Path, PurePosixPath

from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import MirrorMarker
from tools.pokemon_ops.domain.policy import EXCLUDED_TOP_LEVEL


MARKER_FILE = ".pokemon-ops-mirror.json"


class LocalFileTree:
    def is_directory(self, path: Path) -> bool:
        return path.is_dir()

    def list_files(self, root: Path) -> Result[set[PurePosixPath]]:
        if not root.is_dir():
            return Result.fail(ErrorCode.SOURCE_MISSING, "directory does not exist", path=str(root))
        files: set[PurePosixPath] = set()
        try:
            for current, directories, names in os.walk(root):
                current_path = Path(current)
                relative_directory = current_path.relative_to(root)
                directories[:] = [name for name in directories if name not in EXCLUDED_TOP_LEVEL]
                for name in names:
                    relative = PurePosixPath((relative_directory / name).as_posix())
                    if relative.parts and not any(part in EXCLUDED_TOP_LEVEL for part in relative.parts):
                        files.add(relative)
        except OSError as error:
            return Result.fail(ErrorCode.COPY_FAILED, "cannot enumerate directory", path=str(root), reason=str(error))
        return Result.ok(files)

    def is_empty(self, root: Path) -> Result[bool]:
        if not root.is_dir():
            return Result.fail(ErrorCode.MIRROR_MISSING, "mirror directory does not exist", path=str(root))
        try:
            return Result.ok(not any(root.iterdir()))
        except OSError as error:
            return Result.fail(ErrorCode.COPY_FAILED, "cannot inspect mirror directory", path=str(root), reason=str(error))

    def copy_file(self, source_root: Path, destination_root: Path, path: PurePosixPath) -> Result[None]:
        source = source_root.joinpath(*path.parts)
        destination = destination_root.joinpath(*path.parts)
        try:
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        except OSError as error:
            return Result.fail(ErrorCode.COPY_FAILED, "cannot copy file", source=str(source), destination=str(destination), reason=str(error))
        return Result.ok(None)

    def delete_file(self, root: Path, path: PurePosixPath) -> Result[None]:
        target = root.joinpath(*path.parts)
        try:
            target.unlink(missing_ok=True)
            parent = target.parent
            while parent != root and parent.exists() and not any(parent.iterdir()):
                parent.rmdir()
                parent = parent.parent
        except OSError as error:
            return Result.fail(ErrorCode.COPY_FAILED, "cannot delete mirror file", path=str(target), reason=str(error))
        return Result.ok(None)


class JsonMarkerStore:
    def read(self, mirror_root: Path) -> Result[MirrorMarker | None]:
        path = mirror_root / MARKER_FILE
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            return Result.ok(None)
        except (OSError, json.JSONDecodeError) as error:
            return Result.fail(ErrorCode.UNSAFE_MIRROR, "cannot read mirror marker", path=str(path), reason=str(error))
        project_id = data.get("project_id") if isinstance(data, dict) else None
        source_id = data.get("source_id") if isinstance(data, dict) else None
        if not isinstance(project_id, str) or not isinstance(source_id, str):
            return Result.fail(ErrorCode.UNSAFE_MIRROR, "mirror marker is invalid", path=str(path))
        return Result.ok(MirrorMarker(project_id=project_id, source_id=source_id))

    def write(self, mirror_root: Path, marker: MirrorMarker) -> Result[None]:
        path = mirror_root / MARKER_FILE
        try:
            path.write_text(
                json.dumps({"project_id": marker.project_id, "source_id": marker.source_id}, indent=2) + "\n",
                encoding="utf-8",
            )
        except OSError as error:
            return Result.fail(ErrorCode.COPY_FAILED, "cannot write mirror marker", path=str(path), reason=str(error))
        return Result.ok(None)
