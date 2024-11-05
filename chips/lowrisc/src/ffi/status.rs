// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use crate::ffi::hardened::HardenedBool;
use kernel::ErrorCode;

/// Trait to avoid name collisions between decoder functions on status types
/// from different bindgen libraries.
pub trait Status {
    /// Convert the status to a unified bindgen type.
    fn value(&self) -> u32;

    /// Decode a `Status` value to an integer, returning an error if the status
    /// indicates an error.
    fn decode_to_u32(&self) -> Result<u32, OpenTitanError> {
        let status = self.value();
        // Highest-order bit is 0 on ok, 1 on error.
        let ok = (status >> 31) & 0x1 == 0;
        if ok {
            Ok(status)
        } else {
            // Bits 16-30: module identifier
            let mod_ident = (status >> 16) & 0x7FFF;
            // Bits 5-15: line number
            let line_num = (status >> 5) & 0x7FF;
            // Bits 0-4: error code
            let error_code = status & 0x1F;

            Err(OpenTitanError {
                mod_identifier: [
                    parse_ascii_5bit(mod_ident & 0x1F),
                    parse_ascii_5bit((mod_ident >> 5) & 0x1F),
                    parse_ascii_5bit((mod_ident >> 10) & 0x1F),
                ],
                line_number: line_num,
                error_code: StatusCode::from(error_code),
            })
        }
    }

    fn check(&self) -> Result<(), OpenTitanError> {
        self.decode_to_u32().map(|_| ())
    }

    /// Decode a `Status` value to an boolean, returning an error if the
    /// status indicates an error or the hardened boolean was an invalid
    /// value.
    fn decode_to_bool(&self) -> Result<bool, StatusAsBoolError> {
        let hardened_bool_raw = self
            .decode_to_u32()
            .map_err(StatusAsBoolError::OpenTitanError)?;
        let hardened_bool: HardenedBool = hardened_bool_raw.into();
        hardened_bool
            .try_into()
            .map_err(StatusAsBoolError::InvalidBool)
    }
}

// Macro to automatically generate decoders for all `..Status` types from
// different bindgen libraries, which are all equivalent on the C side but the
// Rust compiler treats them as separate types.
#[macro_export]
macro_rules! status_type {
    ($status_type:ty) => {
        impl $crate::ffi::status::Status for $status_type {
            fn value(&self) -> u32 {
                // CAST: casting `i32` as `u32` is a no-op here because no
                // integer comparison is done when parsing error codes.
                self.value as u32
            }
        }
    };
}

status_type!(base_status::status_t);

pub enum StatusAsBoolError {
    // FFI code reported an error.
    OpenTitanError(OpenTitanError),
    // FFI code reported no error, but the `hardened_bool_t` encoded in the
    // status value was invalid. This indicates a fault-injection attack.
    InvalidBool(u32),
}

impl StatusAsBoolError {
    /// Convert a `StatusAsBoolError` to a Tock result
    pub fn to_tock(&self) -> Result<(), ErrorCode> {
        match self {
            StatusAsBoolError::OpenTitanError(e) => e.to_tock(),
            StatusAsBoolError::InvalidBool(_) => Err(ErrorCode::FAIL),
        }
    }

    /// Same as `to_tock`, except `Ok` is mapped to ErrorCode::FAIL. Useful for
    /// parsing status codes where `Ok` in the `StatusCode` location indicates a
    /// self-inconsistent `status_t`.
    pub fn to_tock_err(&self) -> ErrorCode {
        match self.to_tock() {
            Ok(()) => ErrorCode::FAIL,
            Err(e) => e,
        }
    }
}

/// A decoded OpenTitan error status.
#[derive(Debug)]
pub struct OpenTitanError {
    /// Module identifier
    pub mod_identifier: [char; 3],
    /// Line number (in C code) the error was thrown from
    pub line_number: u32,
    /// Error code
    pub error_code: StatusCode,
}

impl OpenTitanError {
    /// Convert an `OpenTitanError` to a Tock result
    pub fn to_tock(&self) -> Result<(), ErrorCode> {
        match self.error_code {
            StatusCode::Ok => Ok(()),
            StatusCode::Cancelled => Err(ErrorCode::CANCEL),
            StatusCode::InvalidArgument => Err(ErrorCode::INVAL),
            StatusCode::NotFound => Err(ErrorCode::NODEVICE),
            StatusCode::AlreadyExists => Err(ErrorCode::ALREADY),
            StatusCode::PermissionDenied | StatusCode::Unimplemented => Err(ErrorCode::NOSUPPORT),
            StatusCode::FailedPrecondition | StatusCode::Unauthenticated => Err(ErrorCode::RESERVE),
            StatusCode::OutOfRange => Err(ErrorCode::SIZE),
            StatusCode::Unavailable => Err(ErrorCode::BUSY),
            // All other status codes produce `FAIL`, including the following:
            // - Unknown
            // - DeadlineExceeded,
            // - ResourceExhausted
            // - Aborted
            // - Internal
            // - DataLoss
            _ => Err(ErrorCode::FAIL),
        }
    }

    /// Same as `to_tock`, except `Ok` is mapped to ErrorCode::FAIL. Useful for
    /// parsing status codes where `Ok` in the `StatusCode` location indicates a
    /// self-inconsistent `status_t`.
    pub fn to_tock_err(&self) -> ErrorCode {
        match self.to_tock() {
            Ok(()) => ErrorCode::FAIL,
            Err(e) => e,
        }
    }
}

impl From<OpenTitanError> for Result<(), ErrorCode> {
    fn from(err: OpenTitanError) -> Result<(), ErrorCode> {
        err.to_tock()
    }
}

/// OpenTitan status codes. See
/// opentitan:sw/device/lib/base/internal/absl_status.h for a description of
/// each status code.
#[derive(Debug)]
pub enum StatusCode {
    Ok,
    Cancelled,
    Unknown,
    InvalidArgument,
    DeadlineExceeded,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    ResourceExhausted,
    FailedPrecondition,
    Aborted,
    OutOfRange,
    Unimplemented,
    Internal,
    Unavailable,
    DataLoss,
    Unauthenticated,
    /// An error code that does not match one of the defined codes.
    InvalidErrorCode(u32),
}

impl From<u32> for StatusCode {
    fn from(val: u32) -> StatusCode {
        match val {
            base_status::absl_status_code_kOk => StatusCode::Ok,
            base_status::absl_status_code_kCancelled => StatusCode::Cancelled,
            base_status::absl_status_code_kUnknown => StatusCode::Unknown,
            base_status::absl_status_code_kInvalidArgument => StatusCode::InvalidArgument,
            base_status::absl_status_code_kDeadlineExceeded => StatusCode::DeadlineExceeded,
            base_status::absl_status_code_kNotFound => StatusCode::NotFound,
            base_status::absl_status_code_kAlreadyExists => StatusCode::AlreadyExists,
            base_status::absl_status_code_kPermissionDenied => StatusCode::PermissionDenied,
            base_status::absl_status_code_kResourceExhausted => StatusCode::ResourceExhausted,
            base_status::absl_status_code_kFailedPrecondition => StatusCode::FailedPrecondition,
            base_status::absl_status_code_kAborted => StatusCode::Aborted,
            base_status::absl_status_code_kOutOfRange => StatusCode::OutOfRange,
            base_status::absl_status_code_kUnimplemented => StatusCode::Unimplemented,
            base_status::absl_status_code_kInternal => StatusCode::Internal,
            base_status::absl_status_code_kUnavailable => StatusCode::Unavailable,
            base_status::absl_status_code_kDataLoss => StatusCode::DataLoss,
            val => StatusCode::InvalidErrorCode(val),
        }
    }
}

/// Decode a 5-bit ASCII character from an OpenTitan module ID that stores a
/// valid between 0x40 ('@') and 0x5F ('_'), inclusive
fn parse_ascii_5bit(ascii: u32) -> char {
    // CAST: Downcasting to a `u8` cannot perform unintentional truncation
    // because we only care about the least significant 5 bits.
    ((ascii & 0x1F) as u8 + (b'@')) as char
}
