"""Dogfood: blackbox_bench measuring its own harness overhead.

Run with: blackbox_bench run benches/bench_dogfood.py
A correctly-functioning Runner with overhead_subtract=True should report
the empty 'pass' benchmark at ~0ns +/- a few ns.
"""
import blackbox_bench


@blackbox_bench.benchmark
def empty_pass():
    pass


@blackbox_bench.benchmark
def trivial_arithmetic():
    1 + 1


@blackbox_bench.benchmark
def short_list_comp():
    [x * x for x in range(8)]
