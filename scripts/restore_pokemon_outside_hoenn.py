#!/usr/bin/env python3
"""Restore every tracked Pokemon source asset without touching generated data."""

from __future__ import annotations

from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile


POKEMON_ROOT = PurePosixPath("assets/source/pokemon")


def repository_root() -> Path:
    return Path(__file__).resolve().parents[1]


def archive(root: Path, output: Path) -> None:
    subprocess.run(
        ["git", "archive", "--format=tar", f"--output={output}", "HEAD", str(POKEMON_ROOT)],
        cwd=root,
        check=True,
    )


def main() -> None:
    root = repository_root()
    restored = 0
    with tempfile.TemporaryDirectory() as temporary:
        archive_path = Path(temporary) / "pokemon.tar"
        archive(root, archive_path)
        with tarfile.open(archive_path) as contents:
            for member in contents:
                if not member.isfile():
                    continue
                path = PurePosixPath(member.name)
                destination = root.joinpath(*path.parts)
                if destination.exists():
                    continue
                destination.parent.mkdir(parents=True, exist_ok=True)
                source = contents.extractfile(member)
                if source is None:
                    raise RuntimeError(f"cannot extract {member.name}")
                destination.write_bytes(source.read())
                restored += 1
    subprocess.run(["git", "lfs", "checkout", str(POKEMON_ROOT)], cwd=root, check=True)
    print(f"restored {restored} tracked Pokemon source files")


if __name__ == "__main__":
    main()
