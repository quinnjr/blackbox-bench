import pybench


def test_black_box_returns_value_unchanged():
    obj = object()
    assert pybench.black_box(obj) is obj


def test_black_box_handles_ints():
    assert pybench.black_box(42) == 42


def test_black_box_handles_lists():
    xs = [1, 2, 3]
    assert pybench.black_box(xs) is xs
