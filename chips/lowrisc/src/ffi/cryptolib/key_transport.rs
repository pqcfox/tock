// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Utilities for key blinding in the OpenTitan cryptography library.

use crate::ffi::hardened::HARDENED_BOOL_FALSE;
use crate::ffi::status::Status;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr::addr_of_mut;
use kernel::ErrorCode;
use otbindgen::{
    otcrypto_blinded_key_t as OtCryptoBlindedKey,
    otcrypto_const_word32_buf_t as OtCryptoConstWord32Buf, otcrypto_import_blinded_key,
    otcrypto_key_config_t as OtCryptoKeyConfig, otcrypto_key_mode_t as OtCryptoKeyMode,
    otcrypto_key_security_level_kOtcryptoKeySecurityLevelHigh as SECURITY_LEVEL_HIGH,
    otcrypto_key_security_level_kOtcryptoKeySecurityLevelLow as SECURITY_LEVEL_LOW,
    otcrypto_key_security_level_kOtcryptoKeySecurityLevelMedium as SECURITY_LEVEL_MEDIUM,
    otcrypto_lib_version_kOtcryptoLibVersion1 as CRYPTOLIB_VERSION_1,
};

/// Wrapper around cryptolib's `otcrypto_blinded_key_t` that enforces that the
/// blinded key buffer it points to must outlive it.
pub struct BlindedKey<'a> {
    key: OtCryptoBlindedKey,
    // Marker ensures this type cannot outlive the key buffer storing its key
    // material.
    _marker: PhantomData<&'a mut [u8]>,
}

impl<'a> BlindedKey<'a> {
    /// Gets a reference to the raw key material.
    ///
    /// Unlike `into_raw`, this function ensures the lifetime of the returned
    /// reference is properly scoped to prevent use-after-free issues at
    /// compile-time.
    pub fn as_raw(&'a self) -> &'a OtCryptoBlindedKey {
        &self.key
    }
    /// Unboxes the value into the raw cryptolib blinded key value.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that the returned value does not
    /// outlive the buffer that was passed to `blind_key` when it was
    /// created.
    ///
    /// This function is not `unsafe` because it only returns a pointer, which
    /// is already unsafe to dereference. However, you must still ensure other
    /// unsafe code does not cause a use-after-free with the contained pointer.
    pub fn into_raw(self) -> OtCryptoBlindedKey {
        self.key
    }
}

/// Key security level, as defined by the OpenTitan cryptography library.
///
/// https://opentitan.org/book/doc/security/cryptolib/cryptolib_api.html#bookkeeping-data-structures
pub enum SecurityLevel {
    Low,
    Medium,
    High,
}

// Maximum key size for HMAC.
//
// TODO: this could be a generic parameter of `blind_key` if
// #![feature(generic_const_exprs)] is stabilized.
const MAX_KEY_LEN: usize = 512 / 8;

/// Creates a blinded (symmetric) key from an unblinded key. An all-zero mask is
/// used.
///
/// `blinded` must be at least twice as long as `unblinded`.
pub fn blind_key<'a>(
    key_mode: OtCryptoKeyMode,
    unblinded: &[u32],
    blinded: &'a mut [u32],
    security_level: SecurityLevel,
) -> Result<BlindedKey<'a>, ErrorCode> {
    if blinded.len() < 2 * unblinded.len() {
        return Err(ErrorCode::SIZE);
    }
    // Template for blinded key
    let mut blinded_key = OtCryptoBlindedKey {
        config: OtCryptoKeyConfig {
            version: CRYPTOLIB_VERSION_1,
            key_mode,
            key_length: unblinded.len(),
            hw_backed: HARDENED_BOOL_FALSE.to_native(),
            exportable: HARDENED_BOOL_FALSE.to_native(),
            security_level: match security_level {
                SecurityLevel::Low => SECURITY_LEVEL_LOW,
                SecurityLevel::Medium => SECURITY_LEVEL_MEDIUM,
                SecurityLevel::High => SECURITY_LEVEL_HIGH,
            },
        },
        keyblob_length: 2 * unblinded.len(),
        keyblob: blinded.as_mut_ptr(),
        // Populated by `integrity_blinded_checksum` below.
        checksum: 0,
    };
    // Use all-zero mask for unblinded keys provided by userspace.
    //
    // TODO: support hardware-backed keys.
    let mask = [0u32; MAX_KEY_LEN / size_of::<u32>()];
    let key_share0 = OtCryptoConstWord32Buf {
        data: unblinded.as_ptr(),
        len: unblinded.len(),
    };
    let key_share1 = OtCryptoConstWord32Buf {
        data: mask.as_ptr(),
        // Only use as long a mask as the key we have; might be < MAX_KEY_LEN.
        len: unblinded.len(),
    };
    // SAFETY: The `PhantomData` marker on the `BlindedKey` type ensures the key
    // structure cannot outlive `blinded`, the buffer storing the key material.
    unsafe {
        otcrypto_import_blinded_key(key_share0, key_share1, addr_of_mut!(blinded_key))
            .check()
            .map_err(|e| e.to_tock_err())?
    };
    Ok(BlindedKey {
        key: blinded_key,
        _marker: PhantomData,
    })
}
