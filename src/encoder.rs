// Copyright 2018, Erlang Solutions Ltd, and S2HC Sweden AB
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::helpers::VecWriteExt;

use super::consts;
use super::errors::{CodecError, CodecResult};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString, PyTuple};
use std::borrow::Cow;
use std::io::Write;
use std::{i32, u16, u8};

pub struct Encoder<'a> {
    pub py: Python<'a>, // Python instance will live at least as long as Encoder
    pub data: Vec<u8>,
    pub encode_hook: Bound<'a, PyDict>,
    pub catch_all: Option<Bound<'a, PyAny>>,
    // A function py_codec_impl.generic_serialize_object used for unknown classes
    pub cached_generic_serialize_fn: Option<Bound<'a, PyAny>>,
}

impl<'a> Encoder<'a> {
    pub fn new(py: Python<'a>, opt: &Bound<'a, PyAny>) -> CodecResult<Self> {
        let py_opts = super::helpers::maybe_dict(opt)?;
        let encode_hook = match py_opts.get_item("encode_hook")? {
            Some(h1) => h1.extract::<Bound<'a, PyDict>>()?,
            None => PyDict::new(py),
        };
        let catch_all = encode_hook.get_item("catch_all")?;

        Ok(Encoder {
            py,
            data: Vec::with_capacity(32),
            encode_hook,
            catch_all,
            cached_generic_serialize_fn: None,
        })
    }

    pub fn encode(&mut self, py_term: &Bound<'a, PyAny>) -> CodecResult<()> {
        let type_name = py_term.get_type().name()?.to_string_lossy().into_owned();
        let type_name_ref: &str = type_name.as_ref();
        match &self.encode_hook.get_item(type_name_ref)? {
            Some(ref h1) => {
                let repr1 = h1.call1((py_term,))?;
                self.encode_default(&repr1)
            }
            None => self.encode_default(py_term),
        }
    }

    pub fn encode_default(&mut self, term: &Bound<'a, PyAny>) -> CodecResult<()> {
        let type_name = term.get_type().name()?.to_string_lossy().into_owned();
        let type_name_ref: &str = type_name.as_ref();

        match type_name_ref {
            "int" => self.write_int(term),
            "float" => {
                let val: f64 = term.extract()?;
                if !val.is_finite() {
                    return Err(CodecError::NonFiniteFloat { f: val });
                }
                self.write_float(val)
            }
            "list" => {
                let as_list = term.extract::<Bound<'a, PyList>>()?;
                self.write_list_no_tail(&as_list)?;
                self.data.push(consts::TAG_NIL_EXT);
                Ok(())
            }
            "tuple" => {
                let as_tup = term.extract::<Bound<'a, PyTuple>>()?;
                self.write_tuple(&as_tup)
            }
            "dict" => {
                let as_dict = term.extract::<Bound<'a, PyDict>>()?;
                self.write_dict(&as_dict)
            }
            "Atom" => self.write_atom(term),
            "StrictAtom" => self.write_atom(term),
            "str" => {
                let as_str = term.extract::<Bound<'a, PyString>>()?;
                self.write_str(&as_str)
            }
            "bool" => {
                let val: bool = term.extract()?;
                self.write_atom_from_cow(if val {
                    Cow::from("true")
                } else {
                    Cow::from("false")
                })
            }
            "NoneType" => self.write_atom_from_cow(Cow::from("undefined")),
            "ImproperList" => {
                let elements = term.getattr("_elements")?.extract::<Bound<'a, PyList>>()?;
                let tail = term.getattr("_tail")?;
                self.write_list_no_tail(&elements)?;
                self.encode(&tail)
            }
            "Pid" => self.write_pid(term),
            "Reference" => self.write_ref(term),
            "bytes" => {
                let py_bytes = term.extract::<Bound<'a, PyBytes>>()?;
                self.write_binary(&py_bytes)
            }
            "BitString" => self.write_bitstring(term),
            //"Fun" => return self.write_fun(&term),
            _other => self.write_unknown_object(type_name_ref, term),
        }
    }

    /// For unknown object, check whether catch_all is set, encode what it returns.
    /// If no catch_all was set, check whether object has ``__etf__(self)`` member.
    /// Else encode object as Tuple(b'ClassName', Dict(b'field', values)) trying
    ///   to avoid circular loops.
    fn write_unknown_object(&mut self, _name: &str, py_term: &Bound<'a, PyAny>) -> CodecResult<()> {
        match &self.catch_all {
            Some(ref h1) => {
                let repr1 = h1.call1((py_term,))?;
                self.encode(&repr1)
            }
            None => match py_term.getattr("__etf__") {
                Ok(h2) => {
                    let repr2 = h2.call0()?;
                    self.encode(&repr2)
                }
                Err(_) => self.write_generic_unknown_object(py_term),
            },
        }
    }

    fn write_generic_unknown_object(&mut self, py_term: &Bound<'a, PyAny>) -> CodecResult<()> {
        let py_fn = match &self.cached_generic_serialize_fn {
            Some(ref a) => a.clone(),
            None => {
                let pyimpl_m = self.py.import("term.py_codec_impl")?;
                let generic_fn = pyimpl_m.getattr("generic_serialize_object")?;
                self.cached_generic_serialize_fn = Some(generic_fn.clone());
                generic_fn
            }
        };
        let result_pair = py_fn.call1((py_term, self.py.None()))?;
        let py_pair = result_pair.extract::<Bound<'a, PyTuple>>()?;
        let result = py_pair.get_item(0)?;
        self.encode(&result)
    }

    /// Writes list tag with elements, but no tail element (NIL or other). Ensure
    /// that the calling code is writing either a NIL or a tail term.
    #[inline]
    fn write_list_no_tail(&mut self, list: &Bound<'a, PyList>) -> CodecResult<()> {
        let size = list.len();
        self.data.push(consts::TAG_LIST_EXT);
        self.data.push_u32(size as u32);
        for i in 0..size {
            let item = list.get_item(i)?;
            self.encode(&item)?;
        }
        Ok(())
    }

    #[inline]
    fn write_tuple(&mut self, tup: &Bound<'a, PyTuple>) -> CodecResult<()> {
        let size = tup.len();
        if size < u8::MAX as usize {
            self.data.push(consts::TAG_SMALL_TUPLE_EXT);
            self.data.push(size as u8);
        } else {
            self.data.push(consts::TAG_LARGE_TUPLE_EXT);
            self.data.push_u32(size as u32);
        }

        for i in 0..size {
            let item = tup.get_item(i)?;
            self.encode(&item)?;
        }
        Ok(())
    }

    /// Writes Erlang map from Python dict.
    #[inline]
    fn write_dict(&mut self, py_dict: &Bound<'a, PyDict>) -> CodecResult<()> {
        let size = py_dict.len();
        self.data.push(consts::TAG_MAP_EXT);
        self.data.push_u32(size as u32);

        for (py_key, py_value) in py_dict.iter() {
            self.encode(&py_key)?;
            self.encode(&py_value)?;
        }
        Ok(())
    }

    #[inline]
    fn write_int(&mut self, val: &Bound<'a, PyAny>) -> CodecResult<()> {
        let size: u64 = val.call_method0("bit_length")?.extract()?;
        let size: u32 = (size / 8 + 1) as u32;
        if size <= 4 {
            let v: i64 = val.extract()?;
            self.write_4byte_int(v)
        } else {
            self.write_arbitrary_int(val, size)
        }
    }

    fn write_arbitrary_int(&mut self, val: &Bound<'a, PyAny>, size: u32) -> CodecResult<()> {
        if size < 256 {
            self.data.push(consts::TAG_SMALL_BIG_EXT);
            self.data.push(size as u8);
        } else {
            self.data.push(consts::TAG_LARGE_BIG_EXT);
            self.data.push_u32(size);
        }

        let ltz: bool = val.call_method1("__lt__", (0,))?.extract()?;
        if ltz {
            self.data.push(1_u8); // we have a negative value
                                  // we make new object that we multiply with -1 to switch sign, so that we get a positive
                                  // value to pack
            let r = val.call_method1("__mul__", (-1,))?;
            let b = r
                .call_method1("to_bytes", (size, "little"))?
                .extract::<Bound<'a, PyBytes>>()?;
            let data: &[u8] = b.as_bytes();
            self.data.write_all(data)?;
        } else {
            self.data.push(0_u8);
            let b = val
                .call_method1("to_bytes", (size, "little"))?
                .extract::<Bound<'a, PyBytes>>()?;
            let data: &[u8] = b.as_bytes();
            self.data.write_all(data)?;
        }
        Ok(())
    }

    #[inline]
    fn write_4byte_int(&mut self, val: i64) -> CodecResult<()> {
        if val >= 0 && val <= u8::MAX as i64 {
            self.data.push(consts::TAG_SMALL_UINT);
            self.data.push(val as u8);
        } else if val >= i32::MIN as i64 && val <= i32::MAX as i64 {
            self.data.push(consts::TAG_INT);
            self.data.push_i32(val as i32);
        } else {
            return Err(CodecError::IntegerEncodingRange { i: val });
        }

        Ok(())
    }

    #[inline]
    fn write_float(&mut self, val: f64) -> CodecResult<()> {
        self.data.push(consts::TAG_NEW_FLOAT_EXT);
        self.data.push_f64(val);
        Ok(())
    }

    /// Encode a UTF-8 Atom
    #[inline]
    fn write_atom(&mut self, py_atom: &Bound<'a, PyAny>) -> CodecResult<()> {
        let py_text = py_atom.extract::<Bound<'a, PyString>>()?;
        let text = py_text.to_cow()?;
        self.write_atom_from_cow(text)
    }

    /// Helper which writes an atom from a PyString's Copy-on-write string
    fn write_atom_from_cow(&mut self, text: Cow<str>) -> CodecResult<()> {
        let byte_array: &[u8] = text.as_ref().as_ref();
        let str_byte_length: usize = byte_array.len();

        if str_byte_length <= u8::MAX as usize {
            self.data.push(consts::TAG_SMALL_ATOM_UTF8_EXT);
            self.data.push(str_byte_length as u8); // 8bit length
            self.data.write_all(byte_array)?; // write &[u8] string content
        } else if str_byte_length <= u16::MAX as usize {
            self.data.push(consts::TAG_ATOM_UTF8_EXT);
            self.data.push_u16(str_byte_length as u16); // 16bit length
            self.data.write_all(byte_array)?; // write &[u8] string content
        } else {
            return Err(CodecError::AtomTooLong);
        }

        Ok(())
    }

    /// Encode a UTF-8 string
    #[inline]
    fn write_str(&mut self, py_str: &Bound<'a, PyString>) -> CodecResult<()> {
        let text = py_str.to_cow()?;
        let byte_array: &[u8] = text.as_ref().as_ref();
        let str_byte_length: usize = byte_array.len();
        let can_be_encoded_as_bytes = can_be_encoded_as_byte_string(&text);

        if str_byte_length <= u8::MAX as usize && can_be_encoded_as_bytes {
            // Create an optimised byte-array structure and push bytes
            self.data.push(consts::TAG_STRING_EXT);
            self.data.push_u16(str_byte_length as u16); // 16bit length
            self.data.write_all(byte_array)?; // write &[u8] string content
        } else {
            // Create a list structure and push each codepoint as an integer
            self.data.push(consts::TAG_LIST_EXT);
            let chars_count = text.chars().count();
            self.data.push_u32(chars_count as u32); // chars, not bytes!
            for (_i, ch) in text.char_indices() {
                self.write_4byte_int(ch as i64)?
            }
            self.data.push(consts::TAG_NIL_EXT) // list terminator
        }

        Ok(())
    }

    /// Encode a Pid
    #[inline]
    fn write_pid(&mut self, py_pid: &Bound<'a, PyAny>) -> CodecResult<()> {
        let node_name = py_pid
            .getattr("node_name_")?
            .extract::<Bound<'a, PyString>>()?;

        let id: u32 = py_pid.getattr("id_")?.extract()?;

        let serial: u32 = py_pid.getattr("serial_")?.extract()?;

        let creation: u32 = py_pid.getattr("creation_")?.extract()?;

        self.data.push(consts::TAG_NEW_PID_EXT);
        self.write_atom_from_cow(node_name.to_cow()?)?;
        self.data.push_u32(id);
        self.data.push_u32(serial);
        self.data.push_u32(creation);

        Ok(())
    }

    /// Encode a Reference
    #[inline]
    fn write_ref(&mut self, py_ref: &Bound<'a, PyAny>) -> CodecResult<()> {
        let node_name = py_ref
            .getattr("node_name_")?
            .extract::<Bound<'a, PyString>>()?;

        let py_id = py_ref.getattr("id_")?.extract::<Bound<'a, PyBytes>>()?;
        let id = py_id.as_bytes();

        let creation: u32 = py_ref.getattr("creation_")?.extract()?;

        self.data.push(consts::TAG_NEWER_REF_EXT);
        self.data.push_u16((id.len() / 4) as u16);
        self.write_atom_from_cow(node_name.to_cow()?)?;
        self.data.push_u32(creation);
        self.data.write_all(id)?;

        Ok(())
    }

    /// Encode a binary (byte-string)
    #[inline]
    fn write_binary(&mut self, py_bytes: &Bound<'a, PyBytes>) -> CodecResult<()> {
        let data: &[u8] = py_bytes.as_bytes();
        self.data.push(consts::TAG_BINARY_EXT);
        self.data.push_u32(data.len() as u32);
        self.data.write_all(data)?;
        Ok(())
    }

    /// Encode a Binary bit-string (last byte has less than 8 bits)
    #[inline]
    fn write_bitstring(&mut self, py_bits: &Bound<'a, PyAny>) -> CodecResult<()> {
        let py_bytes = py_bits.getattr("value_")?.extract::<Bound<'a, PyBytes>>()?;
        let data: &[u8] = py_bytes.as_bytes();

        let last_byte_bits: u8 = py_bits.getattr("last_byte_bits_")?.extract()?;

        self.data.push(consts::TAG_BIT_BINARY_EXT);
        self.data.push_u32(data.len() as u32);
        self.data.push(last_byte_bits);
        self.data.write_all(data)?;
        Ok(())
    }
} // end impl

/// Checks first 65535 characters whether they are single-byte and are not
/// extended code points
fn can_be_encoded_as_byte_string(s: &str) -> bool {
    for (i, ch) in s.char_indices() {
        if i > u16::MAX as usize {
            return false; // too long, so result is false
        }
        if ch as u32 > u8::MAX as u32 {
            return false; // is a unicode codepoint with value larger than 255
        }
    }
    true // will fit in a 255-byte latin-1 string
}
