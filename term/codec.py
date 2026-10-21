"""Adapter module which attempts to import native (Rust) codec implementation
and then if import fails, uses Python codec implementation which is slower
but always works.
"""

from __future__ import annotations

import logging
from typing import Any, cast

from term.basetypes import DecodeOptions, EncodeOptions, Hooks, LegacyEncodeOptions

LOG = logging.getLogger("term")

try:
    import term.native_codec_impl as co_impl
except ImportError:
    LOG.warning(
        "Native term ETF codec library import failed, falling back to slower Python impl"
    )
    import term.py_codec_impl as co_impl


def binary_to_term(
    data: bytes,
    options: DecodeOptions | None = None,
    decode_hook: Hooks | None = None,
) -> tuple[Any, bytes]:
    """
    Strip 131 header and unpack if the data was compressed.

    :param data: The incoming encoded data with the 131 byte
    :param options: None or Options dict (pending design)
                    * "atom": "str" | "bytes" | "Atom" (default "Atom").
                      Returns atoms as strings, as bytes or as atom.Atom objects.
                    * "atom_call": callabe object that returns the atom representation
                    * "byte_string": "str" | "bytes" | "int_list" (default "str").
                      Returns 8-bit strings as Python str, bytes or list of integers.
    :param decode_hook:
                Key/value pairs t: str,f : callable, s.t. f(v) is run before encoding
                for values v of type t. This allows for overriding the built-in encoding.
                "catch_all": f is a callable which will return representation for unknown
                object types.
    :raises PyCodecError: when the tag is not 131, when compressed
                          data is incomplete or corrupted
    :returns: Value and Remaining unconsumed bytes
    """
    opt: DecodeOptions = options if options else {}
    if decode_hook:
        opt["decode_hook"] = decode_hook
    return co_impl.binary_to_term(data, opt)


def term_to_binary(
    term: object,
    options: EncodeOptions | LegacyEncodeOptions | None = None,
    encode_hook: Hooks | None = None,
) -> bytes:
    """
    Prepend the 131 header byte to encoded data.
    :param options: None or a dict of options with key/values "encode_hook": f where f
                is a callable which will return representation for unknown object types.
                This is kept for backward compatibility, and is equivalent to
                    encode_hook={"catch_all": f}
                None will be encoded as such and becomes Atom('undefined').
    :param encode_hook:
                Key/value pairs t: str,f : callable, s.t. f(v) is run before rust encoding
                for values of the type t. This allows for overriding the built-in encoding.
                "catch_all": f is a callable which will return representation for unknown
                object types.
    :returns: Bytes, the term object encoded with erlang binary term format
    """
    opt: EncodeOptions = cast(EncodeOptions, options) if options else {}
    legacy_hook = options.get("encode_hook") if options else None
    if callable(legacy_hook):
        # legacy encode_hook as single function transformed to 'catch_all' in new encode_hook dict
        opt["encode_hook"] = {"catch_all": legacy_hook}
    elif encode_hook:
        opt["encode_hook"] = encode_hook
    return co_impl.term_to_binary(term, opt)


PyCodecError = co_impl.PyCodecError

# aliases

encode = pack = dumps = term_to_binary
decode = unpack = loads = binary_to_term

__all__ = [
    "PyCodecError",
    "binary_to_term",
    "decode",
    "dumps",
    "encode",
    "loads",
    "pack",
    "term_to_binary",
    "unpack",
]
