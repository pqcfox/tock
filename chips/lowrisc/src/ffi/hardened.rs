// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use otbindgen::hardened_bool_t as NativeHardenedBool;
use otbindgen::{
    hardened_bool_kHardenedBoolFalse as NATIVE_HARDENED_BOOL_FALSE,
    hardened_bool_kHardenedBoolTrue as NATIVE_HARDENED_BOOL_TRUE,
};

/// Hardened `false` value.
pub const HARDENED_BOOL_FALSE: HardenedBool = HardenedBool(NATIVE_HARDENED_BOOL_FALSE);
/// Hardened `true` value.
pub const HARDENED_BOOL_TRUE: HardenedBool = HardenedBool(NATIVE_HARDENED_BOOL_TRUE);

/// Rust wrapper around a C `hardened_bool_t`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HardenedBool(u32);

impl HardenedBool {
    /// Convert a `HardendedBool` to its native representation for C FFI.
    pub fn to_native(self) -> NativeHardenedBool {
        // Implictly casts from `u32` to `core::ffi::c_uint`, which is valid on
        // RV32I but not portable to some esoteric architectures.
        self.0
    }
}

impl From<NativeHardenedBool> for HardenedBool {
    fn from(b: NativeHardenedBool) -> HardenedBool {
        // Implictly casts from `core::ffi::c_uint` to `u32`, which is valid on
        // RV32I but not portable to some esoteric architectures.
        HardenedBool(b)
    }
}

impl From<HardenedBool> for NativeHardenedBool {
    fn from(b: HardenedBool) -> NativeHardenedBool {
        b.to_native()
    }
}

impl From<bool> for HardenedBool {
    fn from(b: bool) -> HardenedBool {
        if b {
            HARDENED_BOOL_TRUE
        } else {
            HARDENED_BOOL_FALSE
        }
    }
}

impl TryFrom<HardenedBool> for bool {
    type Error = u32;

    fn try_from(h: HardenedBool) -> Result<bool, u32> {
        // Try to convert a hardened value to its primitive type equivalent. If
        // the conversion fails, that indicates a fault injection attack.
        if h == HARDENED_BOOL_TRUE {
            Ok(true)
        } else if h == HARDENED_BOOL_FALSE {
            Ok(false)
        } else {
            Err(h.0)
        }
    }
}
