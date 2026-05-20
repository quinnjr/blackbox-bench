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


def test_cli_profile_flag_emits_svg_when_pyspy_available(tmp_path):
    import shutil
    if not shutil.which("py-spy"):
        import pytest

        pytest.skip("py-spy not installed")
    _write_bench(tmp_path)
    out = _run(
        [
            "run", "bench_sample.py",
            "--warmup", "0", "--iterations", "2",
            "--profile",
            "--output", str(tmp_path / "out.json"),
            "--format", "json",
        ],
        tmp_path,
    )
    assert out.returncode == 0, out.stderr
    svgs = list(tmp_path.glob("*.svg"))
    assert any("f" in p.name for p in svgs)


def test_cli_profile_flag_errors_if_pyspy_missing(tmp_path, monkeypatch):
    # Windows Python's _Py_HashRandomization_Init requires PATH for some
    # system DLLs; clearing it crashes the interpreter at startup before
    # any of our code runs. The in-process equivalent in
    # test_cli_unit.py::test_main_run_profile_without_pyspy_returns_2
    # exercises the same py-spy-missing branch and works everywhere.
    if sys.platform == "win32":
        import pytest

        pytest.skip("clearing PATH crashes the Windows Python interpreter")
    monkeypatch.setenv("PATH", "")
    _write_bench(tmp_path)
    env = {"PATH": ""}
    out = subprocess.run(
        [sys.executable, "-m", "pybench.cli",
         "run", "bench_sample.py",
         "--warmup", "0", "--iterations", "2",
         "--profile"],
        cwd=tmp_path,
        capture_output=True,
        text=True,
        timeout=30,
        env={**env, "PYTHONPATH": ""},
    )
    assert out.returncode != 0
    assert "py-spy" in out.stderr.lower()


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
