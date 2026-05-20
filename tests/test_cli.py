import json
import subprocess
import sys
from pathlib import Path


def _write_bench(tmp_path: Path) -> Path:
    p = tmp_path / "bench_sample.py"
    p.write_text(
        "import pybench\n"
        "@pybench.benchmark\n"
        "def f():\n"
        "    sum(range(10))\n"
    )
    return p


def _run(args: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "-m", "pybench.cli", *args],
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=30,
    )


def test_cli_run_default_table(tmp_path):
    _write_bench(tmp_path)
    out = _run(["run", "bench_sample.py", "--warmup", "0", "--iterations", "5"], tmp_path)
    assert out.returncode == 0, out.stderr
    assert "Name" in out.stdout
    assert "f" in out.stdout


def test_cli_run_json_output(tmp_path):
    _write_bench(tmp_path)
    out = _run(
        ["run", "bench_sample.py", "--warmup", "0", "--iterations", "5", "--format", "json"],
        tmp_path,
    )
    assert out.returncode == 0, out.stderr
    data = json.loads(out.stdout)
    assert data["results"][0]["name"] == "f"


def test_cli_run_html_to_file(tmp_path):
    _write_bench(tmp_path)
    out_file = tmp_path / "out.html"
    out = _run(
        [
            "run",
            "bench_sample.py",
            "--warmup", "0",
            "--iterations", "5",
            "--format", "html",
            "--output", str(out_file),
        ],
        tmp_path,
    )
    assert out.returncode == 0, out.stderr
    html = out_file.read_text()
    assert "<html" in html.lower() and "f" in html


def test_cli_run_xml_junit_default(tmp_path):
    _write_bench(tmp_path)
    out = _run(
        ["run", "bench_sample.py", "--warmup", "0", "--iterations", "5", "--format", "xml"],
        tmp_path,
    )
    assert out.returncode == 0
    assert "<testsuite" in out.stdout


def test_cli_run_xml_raw_style(tmp_path):
    _write_bench(tmp_path)
    out = _run(
        [
            "run", "bench_sample.py",
            "--warmup", "0", "--iterations", "5",
            "--format", "xml", "--xml-style", "raw",
        ],
        tmp_path,
    )
    assert out.returncode == 0
    assert "<pybench>" in out.stdout
