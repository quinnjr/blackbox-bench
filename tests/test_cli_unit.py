"""Unit-level CLI tests that invoke main(argv=...) directly so coverage sees the code."""
import json
from pathlib import Path

import pytest

from blackbox_bench import cli

from .test_cli import _write_bench


def test_main_run_default_table_to_stdout(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "Name" in out and "f" in out


def test_main_run_json_format(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--format", "json"])
    assert rc == 0
    data = json.loads(capsys.readouterr().out)
    assert data["results"][0]["name"] == "f"


def test_main_run_html_format(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--format", "html"])
    assert rc == 0
    out = capsys.readouterr().out
    assert "<html" in out.lower()


def test_main_run_xml_format_default_junit(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--format", "xml"])
    assert rc == 0
    assert "<testsuite" in capsys.readouterr().out


def test_main_run_xml_format_raw(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main([
        "run", "bench_sample.py",
        "--warmup", "0", "--iterations", "3",
        "--format", "xml", "--xml-style", "raw",
    ])
    assert rc == 0
    assert "<blackbox-bench>" in capsys.readouterr().out


def test_main_run_output_flag_writes_file(tmp_path, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    out_file = tmp_path / "out.html"
    rc = cli.main([
        "run", "bench_sample.py",
        "--warmup", "0", "--iterations", "3",
        "--format", "html",
        "--output", str(out_file),
    ])
    assert rc == 0
    assert "<html" in out_file.read_text().lower()


def test_main_run_save_flag_writes_json_sidecar(tmp_path, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    save_file = tmp_path / "saved.json"
    rc = cli.main([
        "run", "bench_sample.py",
        "--warmup", "0", "--iterations", "3",
        "--save", str(save_file),
    ])
    assert rc == 0
    data = json.loads(save_file.read_text())
    assert data["results"][0]["name"] == "f"


def test_main_run_no_benchmarks_returns_1(tmp_path, capsys, monkeypatch):
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", str(tmp_path)])
    assert rc == 1
    assert "No benchmarks found" in capsys.readouterr().err


def test_main_run_directory_discovers_bench_files(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", str(tmp_path), "--warmup", "0", "--iterations", "3"])
    assert rc == 0
    assert "f" in capsys.readouterr().out


def test_main_run_underscore_bench_suffix_discovered(tmp_path, capsys, monkeypatch):
    (tmp_path / "thing_bench.py").write_text(
        "import blackbox_bench\n"
        "@blackbox_bench.benchmark\n"
        "def g():\n"
        "    pass\n"
    )
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", str(tmp_path), "--warmup", "0", "--iterations", "3"])
    assert rc == 0
    assert "g" in capsys.readouterr().out


def test_main_run_profile_without_pyspy_returns_2(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setenv("PATH", "")
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "2", "--profile"])
    assert rc == 2
    assert "py-spy" in capsys.readouterr().err.lower()


def test_main_run_profile_with_pyspy_invokes_subprocess(tmp_path, capsys, monkeypatch):
    """When py-spy IS available, the profile branch shells out to it with a
    record subcommand, an output flag, and the benchmark's SVG filename."""
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cli.shutil, "which", lambda name: "/usr/bin/py-spy" if name == "py-spy" else None)
    calls: list[list[str]] = []
    monkeypatch.setattr(
        cli.subprocess, "run",
        lambda cmd, check=False, timeout=None, env=None: calls.append((cmd, env)) or None,
    )
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "2", "--profile"])
    assert rc == 0
    # Exactly one py-spy invocation per registered benchmark (here: "f").
    assert len(calls) == 1
    cmd, env = calls[0]
    assert cmd[0:3] == ["py-spy", "record", "-o"]
    assert cmd[3] == "f.svg"
    assert "--" in cmd
    # The harness path is a *.py file we wrote; the bench name flows via env.
    assert cmd[-1].endswith(".py")
    assert env["PYBENCH_NAME"] == "f"


def test_main_compare_default_table_to_stdout(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--save", "baseline.json"])
    capsys.readouterr()  # discard run output
    cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--save", "current.json"])
    capsys.readouterr()
    rc = cli.main(["compare", "baseline.json", "current.json"])
    assert rc == 0
    assert "Name" in capsys.readouterr().out


def test_main_compare_json_format_to_file(tmp_path, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--save", "baseline.json"])
    cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--save", "current.json"])
    out_file = tmp_path / "diff.json"
    rc = cli.main(["compare", "baseline.json", "current.json", "--format", "json", "--output", str(out_file)])
    assert rc == 0
    data = json.loads(out_file.read_text())
    assert "rows" in data


def test_main_run_json_deprecated_alias(tmp_path, capsys, monkeypatch):
    """`--json` is kept as a deprecated alias for `--format json`."""
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "3", "--json"])
    assert rc == 0
    captured = capsys.readouterr()
    assert "deprecated" in captured.err.lower()
    data = json.loads(captured.out)
    assert data["results"][0]["name"] == "f"


def test_discover_skips_files_that_fail_to_import(tmp_path, capsys, monkeypatch):
    """A syntax error in one bench file must not abort discovery of the others."""
    _write_bench(tmp_path)
    (tmp_path / "bench_broken.py").write_text("this is not valid python =\n")
    monkeypatch.chdir(tmp_path)

    rc = cli.main(["run", str(tmp_path), "--warmup", "0", "--iterations", "3"])
    assert rc == 0
    captured = capsys.readouterr()
    assert "warning: skipping" in captured.err
    assert "f" in captured.out  # the good benchmark still ran


def test_main_run_profile_handles_pyspy_timeout(tmp_path, capsys, monkeypatch):
    """If py-spy exceeds the timeout, we print and continue rather than hang."""
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cli.shutil, "which", lambda name: "/usr/bin/py-spy")

    def _raise_timeout(*args, **kwargs):
        raise cli.subprocess.TimeoutExpired(cmd=args[0], timeout=300)

    monkeypatch.setattr(cli.subprocess, "run", _raise_timeout)
    rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "2", "--profile"])
    assert rc == 0
    assert "timed out" in capsys.readouterr().err


def test_main_run_profile_tolerates_harness_unlink_failure(tmp_path, capsys, monkeypatch):
    """If the temp harness file can't be unlinked, --profile must still succeed."""
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(cli.shutil, "which", lambda name: "/usr/bin/py-spy")
    monkeypatch.setattr(cli.subprocess, "run", lambda *a, **k: None)

    real_unlink = cli.os.unlink

    def _raise_unlink(path):
        raise OSError("simulated unlink failure")

    monkeypatch.setattr(cli.os, "unlink", _raise_unlink)
    try:
        rc = cli.main(["run", "bench_sample.py", "--warmup", "0", "--iterations", "2", "--profile"])
    finally:
        # cleanup the leaked temp harness ourselves
        monkeypatch.setattr(cli.os, "unlink", real_unlink)
    assert rc == 0


def test_main_run_target_time_ns_kwarg(tmp_path, capsys, monkeypatch):
    _write_bench(tmp_path)
    monkeypatch.chdir(tmp_path)
    rc = cli.main([
        "run", "bench_sample.py",
        "--warmup", "0", "--iterations", "2",
        "--target-time-ns", "5000000",
    ])
    assert rc == 0
