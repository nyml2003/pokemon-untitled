from __future__ import annotations

import json
from pathlib import Path

from tools.pokemon_ops.domain.config import parse_local_config
from tools.pokemon_ops.domain.errors import ErrorCode, Result
from tools.pokemon_ops.domain.model import LocalConfig


CONFIG_FILE = "ops.local.json"


def load_local_config(source_root: Path) -> Result[LocalConfig]:
    path = source_root / CONFIG_FILE
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return Result.fail(ErrorCode.INVALID_CONFIGURATION, "ops.local.json is missing", path=str(path))
    except json.JSONDecodeError as error:
        return Result.fail(ErrorCode.INVALID_CONFIGURATION, "ops.local.json is invalid JSON", path=str(path), reason=str(error))
    except OSError as error:
        return Result.fail(ErrorCode.INVALID_CONFIGURATION, "cannot read ops.local.json", path=str(path), reason=str(error))
    return parse_local_config(raw, source_root)
