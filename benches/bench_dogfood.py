"""Dogfood: pybench measuring its own harness overhead.

Run with: pybench run benches/bench_dogfood.py
A correctly-functioning Runner with overhead_subtract=True should report
the empty 'pass' benchmark at ~0ns +/- a few ns.
"""
import pybench


@pybench.benchmark
def empty_pass():
    pass


@pybench.benchmark
def trivial_arithmetic():
    1 + 1


@pybench.benchmark
def short_list_comp():
    [x * x for x in range(8)]
