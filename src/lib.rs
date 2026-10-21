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

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

use self::decoder::Decoder;
use self::encoder::Encoder;

mod consts;
mod decoder;
mod encoder;
mod errors;
mod helpers;
mod reader;

create_exception!(native_codec_impl, PyCodecError, PyException);

/// Strips 131 byte header and unpacks if the data was compressed.
#[pyfunction]
fn binary_to_term<'py>(
    b: &Bound<'py, PyBytes>,
    opt: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut dec_state = Decoder::new(b.py(), opt)?;
    let mut reader = b.as_bytes().into();
    Ok(dec_state.decode_with_131tag(&mut reader)?)
}

#[pyfunction]
fn binary_to_term_2<'py>(
    b: &Bound<'py, PyBytes>,
    opt: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let mut dec_state = Decoder::new(b.py(), opt)?;
    let mut reader = b.as_bytes().into();
    Ok(dec_state.decode(&mut reader)?)
}

#[pyfunction]
fn term_to_binary<'py>(
    py_term: &Bound<'py, PyAny>,
    opt: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyBytes>> {
    let py = py_term.py();
    let mut enc_state = Encoder::new(py, opt)?;

    // Rest of the function is identical to ``term_to_binary_2`` except that
    // 131 byte is pushed to the output before the encoder is called
    enc_state.data.push(consts::ETF_VERSION_TAG);

    enc_state.encode(py_term)?;
    Ok(PyBytes::new(py, enc_state.data.as_ref()))
}

#[pyfunction]
fn term_to_binary_2<'py>(
    py_term: &Bound<'py, PyAny>,
    opt: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyBytes>> {
    let py = py_term.py();
    let mut enc_state = Encoder::new(py, opt)?;
    enc_state.encode(py_term)?;
    Ok(PyBytes::new(py, enc_state.data.as_ref()))
}

#[pymodule]
fn native_codec_impl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__doc__", "Erlang Term Format encoding and decoding.")?;
    m.add_function(wrap_pyfunction!(binary_to_term, m)?)?;
    m.add_function(wrap_pyfunction!(binary_to_term_2, m)?)?;
    m.add_function(wrap_pyfunction!(term_to_binary, m)?)?;
    m.add_function(wrap_pyfunction!(term_to_binary_2, m)?)?;
    m.add("PyCodecError", m.py().get_type::<PyCodecError>())?;
    Ok(())
}
