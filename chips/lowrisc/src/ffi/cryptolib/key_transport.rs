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
use kernel::hil::key_transport::SecurityLevel;
use kernel::processbuffer::{ReadOnlyProcessBufferRef, ReadableProcessBuffer};
use kernel::ErrorCode;
use otbindgen::{
    keyblob_num_words, otcrypto_blinded_key_t as OtCryptoBlindedKey,
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

// Maximum key size for HMAC, in bytes.
pub const MAX_KEY_LEN: usize = 256;

/// Creates a blinded (symmetric) key from an unblinded key. An all-zero mask is
/// used.
///
/// `blinded` must be at least twice as long as `unblinded`.
pub fn blind_key<'a>(
    key_mode: OtCryptoKeyMode,
    unblinded: impl CopyIntoU8,
    blinded: &'a mut [u32],
    security_level: SecurityLevel,
) -> Result<BlindedKey<'a>, ErrorCode> {
    // Cryptolib key configuration
    let config = OtCryptoKeyConfig {
        version: CRYPTOLIB_VERSION_1,
        key_mode,
        key_length: unblinded.length(),
        hw_backed: HARDENED_BOOL_FALSE.to_native(),
        exportable: HARDENED_BOOL_FALSE.to_native(),
        security_level: match security_level {
            SecurityLevel::Low => SECURITY_LEVEL_LOW,
            SecurityLevel::Medium => SECURITY_LEVEL_MEDIUM,
            SecurityLevel::High => SECURITY_LEVEL_HIGH,
        },
    };
    // SAFETY: Cryptolib function `keyblob_num_words` has no transitive pointer
    // arguments that could cause lifetime issues.
    let blinded_keyblob_words = unsafe { keyblob_num_words(config) };
    // If we do not have enough bytes for the blinded buffer, return an error.
    if blinded.len() * size_of::<u32>() < blinded_keyblob_words {
        return Err(ErrorCode::SIZE);
    }
    // Copy of the unblinded key with 4-byte alignment.
    let mut unblinded_buf = [0u32; MAX_KEY_LEN / size_of::<u32>()];
    // SAFETY: we temporarily reinterpret `unblinded_buf` as a `u8`
    // slice to copy in the bytes, and adjust the length
    // accordingly. This cannot cause any invalid values to arise
    // because the individual bytes within a `u32` are always a valid
    // `u8`.
    //
    // PANIC: `unsized_buf` is guaranteed to be the correct length
    // because the ignored "unaligned" slices must be empty -- any
    // slice is at least 1-byte aligned.
    unsafe {
        let (_, unsized_buf, _) = unblinded_buf.align_to_mut::<u8>();
        // Don't use the padded length here, as that would overrun `unblinded`
        // if `unblinded.length() % 4 != 0`.
        unblinded.copy_into(&mut unsized_buf[..unblinded.length()])?;
    }

    // Template for blinded key
    let mut blinded_key = OtCryptoBlindedKey {
        config,
        keyblob_length: blinded_keyblob_words * size_of::<u32>(),
        keyblob: blinded.as_mut_ptr(),
        // Populated by `integrity_blinded_checksum` within
        // `otcrypto_import_blinded_key`.
        checksum: 0,
    };
    // Use all-zero mask for unblinded keys provided by userspace.
    //
    // TODO: support hardware-backed keys.
    let mask = [0u32; MAX_KEY_LEN.div_ceil(size_of::<u32>())];
    let key_share0 = OtCryptoConstWord32Buf {
        data: unblinded_buf.as_ptr(),
        len: unblinded.length().div_ceil(size_of::<u32>()),
    };
    let key_share1 = OtCryptoConstWord32Buf {
        data: mask.as_ptr(),
        // Only use as long a mask as the key we have; might be < MAX_KEY_LEN.
        len: unblinded.length().div_ceil(size_of::<u32>()),
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

pub trait CopyIntoU8 {
    fn copy_into(&self, dest: &mut [u8]) -> Result<(), ErrorCode>;
    fn length(&self) -> usize;
}

impl CopyIntoU8 for &ReadOnlyProcessBufferRef<'_> {
    fn copy_into(&self, dest: &mut [u8]) -> Result<(), ErrorCode> {
        self.enter(|src| {
            let len = src.len().min(dest.len());
            // PANIC: Slices ending at the mininum of the lengths of `src` and
            // `dest` cannot go OOB on either slice.
            src[..len].copy_to_slice(&mut dest[..len])
        })
        .map_err(|_| ErrorCode::RESERVE)
    }

    fn length(&self) -> usize {
        self.len()
    }
}

impl CopyIntoU8 for &[u8] {
    fn copy_into(&self, dest: &mut [u8]) -> Result<(), ErrorCode> {
        dest.clone_from_slice(&self);
        Ok(())
    }

    fn length(&self) -> usize {
        self.len()
    }
}
