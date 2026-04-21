// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::error::Error;

/// A convenience function for making a [JsValue] from a type that implements [Error]
pub(crate) fn napi_error_from_error(error: impl Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

/// A convenience function for making a [napi::Error] from `Box<dyn Error>`
pub(crate) fn napi_error_from_dyn_error(error: Box<dyn Error>) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}
