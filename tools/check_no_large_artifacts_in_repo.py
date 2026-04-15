#!/usr/bin/env python3
from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path


FORBIDDEN_PATH_PREFIXES = (
    "depmap_data/",
    "alignment/",
    "alignment_v2/",
    ".depmap_cache/",
)

FORBIDDEN_SUFFIXES = (
    ".duckdb",
    ".db",
    ".db-shm",
    ".db-wal",
    ".npz",
    ".xlsx",
    ".parquet",
    ".feather",
    ".pkl",
    ".joblib",
    ".rds",
    ".h5",
    ".h5ad",
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Fail if a repository tracks forbidden artifact paths or oversized files."
        )
    )
    parser.add_argument(
        "--max-bytes",
        type=int,
        default=5 * 1024 * 1024,
        help="Maximum allowed tracked file size in bytes. Default: 5242880 (5 MiB).",
    )
    return parser.parse_args()


def tracked_files() -> list[str]:
    result = subprocess.run(
        ["git", "ls-files"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    return [line.strip() for line in result.stdout.splitlines() if line.strip()]


def main() -> int:
    args = parse_args()
    repo_root = Path.cwd()
    violations: list[str] = []

    for rel_path in tracked_files():
        rel_path_posix = rel_path.replace("\\", "/")
        path = repo_root / rel_path

        if any(rel_path_posix.startswith(prefix) for prefix in FORBIDDEN_PATH_PREFIXES):
            violations.append(f"forbidden path prefix: {rel_path_posix}")
            continue

        if rel_path_posix.endswith(FORBIDDEN_SUFFIXES):
            violations.append(f"forbidden artifact suffix: {rel_path_posix}")
            continue

        if not path.is_file():
            continue

        size = path.stat().st_size
        if size > args.max_bytes:
            violations.append(
                f"tracked file too large ({size} bytes > {args.max_bytes}): {rel_path_posix}"
            )

    if violations:
        print("Large artifact policy violation(s) detected:", file=sys.stderr)
        for item in violations:
            print(f"- {item}", file=sys.stderr)
        return 1

    print(
        f"ok: no forbidden tracked artifacts and no tracked files larger than {args.max_bytes} bytes"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
