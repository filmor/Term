import pytest

from term import native_codec_impl, py_codec_impl


@pytest.fixture(params=[py_codec_impl, native_codec_impl], ids=["python", "native"])
def codec(request):
    return request.param
