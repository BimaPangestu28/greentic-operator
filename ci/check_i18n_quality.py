#!/usr/bin/env python3
"""Validate translation catalogs for key parity and English-clone quality."""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path
from typing import Dict


def parse_ratio(env_name: str, default: float) -> float:
    raw = os.getenv(env_name)
    if raw is None or raw.strip() == "":
        return default
    try:
        value = float(raw)
    except ValueError as exc:
        raise SystemExit(f"{env_name} must be a float between 0.0 and 1.0, got: {raw!r}") from exc
    if value < 0.0 or value > 1.0:
        raise SystemExit(f"{env_name} must be between 0.0 and 1.0, got: {value}")
    return value


def load_json(path: Path) -> Dict[str, str]:
    try:
        with path.open("r", encoding="utf-8") as handle:
            raw = json.load(handle)
    except Exception as exc:  # noqa: BLE001
        raise SystemExit(f"failed to read/parse JSON file {path}: {exc}") from exc
    if not isinstance(raw, dict):
        raise SystemExit(f"catalog file is not a JSON object: {path}")
    non_string = [k for k, v in raw.items() if not isinstance(v, str)]
    if non_string:
        preview = ", ".join(non_string[:5])
        raise SystemExit(f"catalog {path} has non-string values for keys: {preview}")
    return raw


def validate_catalog_dir(path: Path, max_same_ratio: float) -> list[str]:
    errors: list[str] = []
    base_path = path / "en.json"
    if not base_path.exists():
        return [f"missing base catalog: {base_path}"]

    base = load_json(base_path)
    base_keys = set(base.keys())
    key_total = len(base_keys)
    if key_total == 0:
        return [f"base catalog has no keys: {base_path}"]

    for locale_file in sorted(path.glob("*.json")):
        locale = locale_file.stem
        if locale.startswith("en"):
            continue

        data = load_json(locale_file)
        keys = set(data.keys())
        missing = sorted(base_keys - keys)
        extra = sorted(keys - base_keys)
        if missing or extra:
            details = []
            if missing:
                details.append(f"missing={len(missing)}")
            if extra:
                details.append(f"extra={len(extra)}")
            errors.append(f"{locale_file}: key mismatch ({', '.join(details)})")
            continue

        same = sum(1 for key in base_keys if data[key] == base[key])
        if same == key_total:
            errors.append(f"{locale_file}: appears to be a full English clone ({same}/{key_total})")
            continue

        ratio = same / key_total
        if ratio > max_same_ratio:
            errors.append(
                f"{locale_file}: too many English-identical values ({same}/{key_total} = {ratio:.1%}, max {max_same_ratio:.1%})"
            )

    return errors


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    cli_max_same_ratio = parse_ratio("I18N_MAX_SAME_RATIO_OPERATOR_CLI", 0.60)
    wizard_max_same_ratio = parse_ratio("I18N_MAX_SAME_RATIO_OPERATOR_WIZARD", 0.40)

    checks = [
        (root / "i18n" / "operator_cli", cli_max_same_ratio),
        (root / "i18n" / "operator_wizard", wizard_max_same_ratio),
    ]

    all_errors: list[str] = []
    for catalog_dir, max_same_ratio in checks:
        all_errors.extend(validate_catalog_dir(catalog_dir, max_same_ratio))

    if all_errors:
        print("[i18n-check] FAILED:")
        for err in all_errors:
            print(f"  - {err}")
        return 1

    print("[i18n-check] ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
