import blackbox_bench


def test_black_box_returns_value_unchanged():
    obj = object()
    assert blackbox_bench.black_box(obj) is obj


def test_black_box_handles_ints():
    assert blackbox_bench.black_box(42) == 42


def test_black_box_handles_lists():
    xs = [1, 2, 3]
    assert blackbox_bench.black_box(xs) is xs
