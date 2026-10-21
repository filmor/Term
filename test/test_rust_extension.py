import pytest

import term.native_codec_impl as nativecodec
import term.py_codec_impl as pycodec
from term.atom import Atom


@pytest.mark.parametrize("data", [b"xxx", b"", b"\x83\x50\x00"])
def test_b2t_errors(data):
    with pytest.raises(nativecodec.PyCodecError):
        nativecodec.binary_to_term(data, None)


def test_b2t_library_equality_atoms():
    a = pycodec.term_to_binary(Atom("hello"))
    (b, _) = nativecodec.binary_to_term(a, None)
    assert Atom("hello") == b


def test_native_entry_points():
    value = {Atom("values"): [True, False, None, 1.5, 2**100, -(2**100), b"data"]}
    encoded = nativecodec.term_to_binary(py_term=value, opt=None)
    assert nativecodec.binary_to_term(b=encoded + b"tail", opt=None) == (value, b"tail")
    body = nativecodec.term_to_binary_2(py_term=value, opt=None)
    assert body == encoded[1:]
    assert nativecodec.binary_to_term_2(b=body, opt=None) == value


@pytest.mark.parametrize(
    ("function", "value", "hook"),
    [
        (nativecodec.binary_to_term, b"\x83a\x01", "decode_hook"),
        (nativecodec.binary_to_term_2, b"a\x01", "decode_hook"),
        (nativecodec.term_to_binary, 1, "encode_hook"),
        (nativecodec.term_to_binary_2, 1, "encode_hook"),
    ],
)
@pytest.mark.parametrize("invalid_hook", [False, True])
def test_invalid_options(function, value, hook, invalid_hook):
    options = {hook: []} if invalid_hook else []
    with pytest.raises(nativecodec.PyCodecError):
        function(value, options)
