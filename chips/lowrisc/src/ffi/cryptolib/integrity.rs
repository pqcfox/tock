// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use core::ptr::addr_of;
use cryptolib_integrity::integrity_unblinded_checksum;

/// Trait to avoid name collisions between checksum functions on
/// `otcrypto_unblinded_key` from multiple bindgen libraries.
pub trait IntegrityUnblindedChecksum {
    /// Convert the bindgen type into a unified type.
    fn as_unified(&self) -> cryptolib_integrity::otcrypto_unblinded_key;

    /// Set the checksum of the original type to the calculated value.
    fn set_checksum(&mut self, checksum: u32);

    /// Computes and populates the checksum on an `otcrypto_unblinded_key`.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that `self.key_storage` is a
    /// non-dangling, properly-aligned pointer that is at least
    /// `ceil_div(self.key_length / 4)` 32-bit words long.
    unsafe fn populate_checksum(&mut self) {
        let unified = self.as_unified();
        self.set_checksum(integrity_unblinded_checksum(addr_of!(unified)));
    }
}

// Macro to automatically generate decoders for all `otcrypto_unblinded_key`
// types from different bindgen libraries, which are all equivalent on the C
// side but the Rust compiler treats them as separate types.
#[macro_export]
macro_rules! integrity_unblinded_checksum {
    ($type:ty) => {
        impl $crate::ffi::cryptolib::integrity::IntegrityUnblindedChecksum for $type {
            fn as_unified(&self) -> cryptolib_integrity::otcrypto_unblinded_key {
                cryptolib_integrity::otcrypto_unblinded_key {
                    key_mode: self.key_mode,
                    key_length: self.key_length,
                    key: self.key,
                    checksum: self.checksum,
                }
            }

            fn set_checksum(&mut self, checksum: u32) {
                self.checksum = checksum;
            }
        }
    };
}

integrity_unblinded_checksum!(cryptolib_integrity::otcrypto_unblinded_key);
