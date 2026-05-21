#!/usr/bin/env python3

import argparse
import subprocess
import sys
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Run a Bevy example for a bounded duration and print the last log lines."
    )
    parser.add_argument("example", help="Example binary name, e.g. point_spot_scene")
    parser.add_argument(
        "--seconds",
        type=float,
        default=5.0,
        help="How long to let the example run before terminating it",
    )
    parser.add_argument(
        "--lines",
        type=int,
        default=120,
        help="How many trailing lines to print",
    )
    parser.add_argument(
        "--cargo",
        action="store_true",
        help="Run via `cargo run --example ...` instead of the built binary",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parent.parent
    if args.cargo:
        cmd = ["cargo", "run", "-p", "bevy_luna", "--example", args.example]
    else:
        cmd = [str(repo_root / "target" / "debug" / "examples" / args.example)]

    try:
        completed = subprocess.run(
            cmd,
            cwd=repo_root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=args.seconds,
            check=False,
        )
        output = completed.stdout
    except subprocess.TimeoutExpired as timeout:
        output = timeout.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")

    lines = output.splitlines(keepends=True)
    sys.stdout.write("".join(lines[-args.lines :]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
