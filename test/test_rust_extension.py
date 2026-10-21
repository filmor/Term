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
