"""pybench CLI."""
from __future__ import annotations

import argparse
import importlib.util
import shutil
import subprocess
import sys
from pathlib import Path

from pybench._bench import Bench, _global_registry


def _discover(path: Path) -> list[tuple[str, callable, dict]]:
    if path.is_file():
        files = [path]
    else:
        files = sorted(list(path.glob("bench_*.py")) + list(path.glob("*_bench.py")))
    _global_registry.clear()
    for f in files:
        spec = importlib.util.spec_from_file_location(f.stem, f)
        if spec and spec.loader:
            mod = importlib.util.module_from_spec(spec)
            sys.modules[f.stem] = mod
            spec.loader.exec_module(mod)
    return list(_global_registry)


def _cmd_run(args: argparse.Namespace) -> int:
    benchmarks = _discover(Path(args.path))
    if not benchmarks:
        print("No benchmarks found.", file=sys.stderr)
        return 1
    if args.profile:
        if not shutil.which("py-spy"):
            print("py-spy not found on PATH. Install with: pip install py-spy", file=sys.stderr)
            return 2
        for name, _fn, _opts in benchmarks:
            svg = Path(f"{name}.svg")
            cmd = [
                "py-spy", "record", "-o", str(svg), "--",
                sys.executable, "-c",
                f"import importlib.util,sys;"
                f"spec=importlib.util.spec_from_file_location('m','{args.path}');"
                f"m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);"
                f"m.{name}()",
            ]
            subprocess.run(cmd, check=False)
    bench = Bench(
        warmup=args.warmup,
        target_time_ns=args.target_time_ns,
        iterations=args.iterations,
    )
    for name, fn, opts in benchmarks:
        bench._registered.append((name, fn, opts))
    bench.run()

    if args.format == "table":
        text = bench.to_table()
    elif args.format == "json":
        text = bench.to_json()
    elif args.format == "html":
        text = bench.to_html()
    elif args.format == "xml":
        text = bench.to_xml(style=args.xml_style)
    else:
        raise SystemExit(f"unknown format: {args.format}")

    if args.output:
        Path(args.output).write_text(text)
    else:
        print(text)

    if args.save:
        Path(args.save).write_text(bench.to_json())
    return 0


def _cmd_compare(args: argparse.Namespace) -> int:
    from pybench._pybench import compare

    baseline = Path(args.baseline).read_text()
    current = Path(args.current).read_text()
    report = compare(baseline, current)
    text = report.format(args.format)
    if args.output:
        Path(args.output).write_text(text)
    else:
        print(text)
    return 0


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="pybench")
    sub = p.add_subparsers(dest="command", required=True)

    r = sub.add_parser("run")
    r.add_argument("path", nargs="?", default=".")
    r.add_argument("--warmup", type=int, default=5)
    r.add_argument("--iterations", type=int, default=None)
    r.add_argument("--target-time-ns", type=int, default=1_000_000_000)
    r.add_argument("--format", choices=["table", "json", "html", "xml"], default="table")
    r.add_argument("--xml-style", choices=["junit", "raw"], default="junit")
    r.add_argument("--output", default=None)
    r.add_argument("--save", default=None, help="Write JSON results to file (in addition to --output)")
    r.add_argument("--profile", action="store_true",
                   help="Wrap each benchmark in py-spy and emit SVG flamegraphs")

    c = sub.add_parser("compare")
    c.add_argument("baseline")
    c.add_argument("current")
    c.add_argument("--format", choices=["table", "json", "html", "xml"], default="table")
    c.add_argument("--output", default=None)

    args = p.parse_args(argv)
    if args.command == "run":
        return _cmd_run(args)
    if args.command == "compare":
        return _cmd_compare(args)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
