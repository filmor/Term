Erlang Term and Codec for Python
================================

This project is a part of http://github.com/Pyrlang/Pyrlang
but does not depend on it and can be used separately.

The Term library adds support classes to represent Erlang terms in Python 
and implements a codec for encoding and decoding data in Erlang
External Term Format (abbreviated ETF) written in Python and a native 
and (most likely) safe Python extension written in Rust. 

The extension or Python implementation is selected automatically when you import 
`term.codec` and all API is available via `term.codec` module. If native 
extension was not found, a warning will be logged and the Python implementation
will be used.

## Installing

### From PyPI

If you just run 
```
pip install pyrlang-term
```
an `abi3` wheel will be installed if one is available for your platform.
Otherwise, installation builds the native extension from source and requires
Rust.

If you want to build the native one, you'll need rust and a few more packages.

To install rust (from https://www.rust-lang.org/tools/install):

```
curl https://sh.rustup.rs -sSf | sh
```

Build requirements, including maturin, are installed automatically from
`pyproject.toml`. The Python codec remains available as `term.py_codec_impl`.

The native extension uses PyO3 with Python's stable ABI (`abi3`), targeting
Python 3.10 and later. A single native wheel per platform supports these Python
versions without rebuilding for each interpreter.

### From Source

1. Clone [Term](https://github.com/Pyrlang/Term) repository
2. Install [uv](https://docs.astral.sh/uv/getting-started/installation/).
3. Go to the Term directory and run `uv sync --extra dev`.

Package metadata, optional dependencies, and the maturin build configuration
are defined in `pyproject.toml`. To build a source distribution and wheel, run
`uv build`. Maturin includes the Python package and Rust sources in source
distributions and tags native wheels as `cp310-abi3`.

## Testing

To run the tests:

```
uv run --extra dev pytest
```

The tests exercise both codecs and require a working Rust extension.
Documentation dependencies can be installed with `uv sync --extra docs`.

The package includes type annotations and stubs for the native codec. Check
the types with:

```
uv run --extra dev ty check term
```

Codec options are described by `DecodeOptions` and `EncodeOptions` in
`term.basetypes`. Decoded values are typed as `Any`, since custom atom factories
and decode hooks can return arbitrary Python objects.


## Atoms

The native representation of atoms are found in `term.atom`. There are Two
classes, `Atom` and `StrictAtom`. `Atom` is the default, it will become an
atom when converting back to `etf`, however it evaluates as string so it's
possible to use a map with atom keys as keyword argument.

The drawback of this is if you may have a map with both atoms and string
/binaries with the same content

```erlang
#{foo => <<"atom">>, "foo" => <<"list">>}
```
Then you'll get
```python
In [1]: from term import codec

In [2]: data = bytes([131,116,0, ...])

In [3]: codec.binary_to_term(data)
Out[3]: ({Atom('foo'): b'list'}, b'')
```

To allow for this we've added another atom type `StrictAtom` that will give you:
```python
In[4]: codec.binary_to_term(data, {"atom": "StrictAtom"})
Out[4]: ({StrictAtom("foo"): b"atom", "foo": b"list"}, b"")
```
Still `StrictAtom('foo') == 'foo'` so it you need something different still, you
can put in your custom atom class

```python
In [5]: class A:
   ...:     def __init__(self, s):
   ...:         self._s = s
   ...:     def __repr__(self):
   ...:         return 'A({})'.format(self._s)
   ...:

In [6]: codec.binary_to_term(data, {'atom_call': A})
Out[6]: ({A(foo): b'atom', 'foo': b'list'}, b'')

```
The `'atom_call'` option takes any callable that takes a string as input, and
the return value will be used for the atom representation. Only `Atom` and
`StrictAtom` can be natively parsed back to atom when decoded. If you roll your
own, make sure to use `encode_hook` when encoding.

More Documentation
-------------

Here: https://pyrlang.github.io/Term/
