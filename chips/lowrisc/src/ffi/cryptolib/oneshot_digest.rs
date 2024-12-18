// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.  Confidential information of zeroRISC Inc. All rights
// reserved.

// Driver that implements oneshot digest operations using the OpenTitan
// cryptography library.
//
// See
// https://opentitan.org/book/doc/security/cryptolib/cryptolib_api.html#hash-functions
// for more details.

use crate::ffi::cryptolib::key_transport::{blind_key, MAX_KEY_LEN};
use crate::ffi::status::Status;
use core::mem::size_of;
use core::ptr::addr_of;
use kernel::hil::key_transport::SecurityLevel;
use kernel::processbuffer::{
    ReadOnlyProcessBufferRef, ReadWriteProcessBufferRef, ReadableProcessBuffer,
};
use kernel::ErrorCode;
use otbindgen::{
    otcrypto_const_byte_buf_t as OtCryptoConstByteBuf, otcrypto_hash,
    otcrypto_hash_digest_t as OtCryptoHashDigest, otcrypto_hmac, otcrypto_kmac,
    otcrypto_word32_buf_t as OtCryptoWord32Buf, otcrypto_xof_cshake, otcrypto_xof_shake,
};

// Implementation of oneshot hash functions (SHA-2 and SHA-3).

pub struct OtCryptoOneshotDigest;

macro_rules! oneshot_hash_impl {
    {$trait:ty, $width:expr, $mode:expr} => {
        impl $trait for OtCryptoOneshotDigest {
            fn digest(input: &ReadOnlyProcessBufferRef<'_>, digest: &mut [u32; $width / size_of::<u32>()]) -> Result<(), ErrorCode> {
                // SAFETY: `input` and `digest` live long enough for the
                // cryptolib APIs used.
                unsafe {
                    let input_message = OtCryptoConstByteBuf {
                        data: input.ptr(),
                        len: input.len(),
                    };
                    // Digest structure for hash digest operations
                    let otcrypto_digest = OtCryptoHashDigest {
                        mode: $mode,
                        data: digest.as_mut_ptr(),
                        len: digest.len(),
                    };
                    otcrypto_hash(input_message, otcrypto_digest)
                        .check()
                        .map_err(|e| e.to_tock_err())
                }
            }
        }
    }
}

oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha256, 32, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha256}
oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha384, 48, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha384}
oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha512, 64, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha512}
oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha3_224, 28, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha3_224}
oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha3_256, 32, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha3_256}
oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha3_384, 48, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha3_384}
oneshot_hash_impl! {kernel::hil::oneshot_digest::Sha3_512, 64, otbindgen::otcrypto_hash_mode_kOtcryptoHashModeSha3_512}

// Implementation of Keccak-based XOFs (SHAKE and cSHAKE).

macro_rules! oneshot_shake_impl {
    {$trait:ty, $mode:expr} => {
        impl $trait for OtCryptoOneshotDigest {
            fn digest(input: &ReadOnlyProcessBufferRef<'_>, digest: &mut ReadWriteProcessBufferRef<'_>) -> Result<(), ErrorCode> {
                // Ensure `input` and `digest` do not overlap, as this would
                // violate memory safety.
                check_nonoverlapping(input, digest)?;
                // SAFETY: `input` and `digest` live long enough for the
                // cryptolib APIs used.
                //
                // `input` and `digest` were manually checked to not overlap, as
                // to not violate the `const` property on `input_message`.
                unsafe {
                    let input_message = OtCryptoConstByteBuf {
                        data: input.ptr(),
                        len: input.len(),
                    };
                    // Digest structure for hash digest operations
                    //
                    // SAFETY: We uphold the invariant of `digest_aligned_ptr`
                    // by not using `digest` in an immutable context while
                    // `aligned_ptr` is in-scope.
                    let digest_len = digest.len() / size_of::<u32>();
                    let aligned_ptr = digest_aligned_ptr(digest)?;
                    // Digest structure for hash digest operations
                    let digest = OtCryptoHashDigest {
                        mode: $mode,
                        data: aligned_ptr,
                        len: digest_len,
                    };
                    otcrypto_xof_shake(input_message, digest)
                        .check()
                        .map_err(|e| e.to_tock_err())
                }
            }
        }
    }
}

oneshot_shake_impl! {kernel::hil::oneshot_digest::Shake128, otbindgen::otcrypto_hash_mode_kOtcryptoHashXofModeShake128}
oneshot_shake_impl! {kernel::hil::oneshot_digest::Shake256, otbindgen::otcrypto_hash_mode_kOtcryptoHashXofModeShake256}

macro_rules! oneshot_cshake_impl {
    {$trait:ty, $mode:expr} => {
        impl $trait for OtCryptoOneshotDigest {
            fn digest(
                input: &ReadOnlyProcessBufferRef<'_>,
                function_name: &ReadOnlyProcessBufferRef<'_>,
                customization_string: &ReadOnlyProcessBufferRef<'_>,
                digest: &mut ReadWriteProcessBufferRef<'_>,
            ) -> Result<(), ErrorCode> {
                // Ensure `digest` does not overlap with any of the inputs, as
                // this would violate memory safety.
                check_nonoverlapping(input, digest)?;
                check_nonoverlapping(function_name, digest)?;
                check_nonoverlapping(customization_string, digest)?;
                // SAFETY: `input_message` and `digest` live long enough for the
                // cryptolib APIs used.
                //
                // `digest` was manually checked to not overlap with `input`,
                // `function_name` or `customization_string`, as to not violate
                // the `const` property on the latter three.
                unsafe {
                    let input_message = OtCryptoConstByteBuf {
                        data: input.ptr(),
                        len: input.len(),
                    };
                    // CSHAKE function name
                    let cshake_function_name = OtCryptoConstByteBuf {
                        data: function_name.ptr(),
                        len: function_name.len(),
                    };
                    // CSHAKE customization string
                    let cshake_customization_string = OtCryptoConstByteBuf {
                        data: customization_string.ptr(),
                        len: customization_string.len(),
                    };
                    // Digest structure for hash digest operations
                    //
                    // SAFETY: We uphold the invariant of `digest_aligned_ptr`
                    // by not using `digest` in an immutable context while
                    // `aligned_ptr` is in-scope.
                    let digest_len = digest.len() / size_of::<u32>();
                    let aligned_ptr = digest_aligned_ptr(digest)?;
                    // Digest structure for hash digest operations
                    let digest = OtCryptoHashDigest {
                        mode: $mode,
                        data: aligned_ptr,
                        len: digest_len,
                    };
                    otcrypto_xof_cshake(input_message, cshake_function_name, cshake_customization_string, digest)
                        .check()
                        .map_err(|e| e.to_tock_err())
                }
            }
        }
    }
}

oneshot_cshake_impl! {kernel::hil::oneshot_digest::Cshake128, otbindgen::otcrypto_hash_mode_kOtcryptoHashXofModeCshake128}
oneshot_cshake_impl! {kernel::hil::oneshot_digest::Cshake256, otbindgen::otcrypto_hash_mode_kOtcryptoHashXofModeCshake256}

// Implementation of oneshot HMACs.

macro_rules! oneshot_hmac_impl {
    {$trait:ty, $width:expr, $key_mode:expr} => {
        impl $trait for OtCryptoOneshotDigest {
            fn digest(key: &ReadOnlyProcessBufferRef<'_>, input: &ReadOnlyProcessBufferRef<'_>, security_level: SecurityLevel, digest: &mut [u32; $width / size_of::<u32>()]) -> Result<(), ErrorCode> {
                // SAFETY: `key`, `input`, and `digest` live long enough for the
                // cryptolib APIs used.
                unsafe {
                    // Allocate twice the max key length for the blinded key.
                    let mut blinded_buf = [0u32; 2 * MAX_KEY_LEN / size_of::<u32>()];
                    let blinded_key = blind_key($key_mode, key, &mut blinded_buf, security_level)?;
                    let input_message = OtCryptoConstByteBuf {
                        data: input.ptr(),
                        len: input.len(),
                    };
                    // Digest structure for hash digest operations
                    let tag = OtCryptoWord32Buf {
                        data: digest.as_mut_ptr(),
                        len: digest.len(),
                    };
                    // SAFETY: `blinded_key` outlives the call to
                    // `otcrypto_hmac`, satisfying the constraints of
                    // `BlindedKey::into_raw`.
                    let raw = blinded_key.into_raw();
                    otcrypto_hmac(addr_of!(raw), input_message, tag)
                        .check()
                        .map_err(|e| e.to_tock_err())
                }
            }
        }
    }
}

oneshot_hmac_impl! {kernel::hil::oneshot_digest::HmacSha256, 32, otbindgen::otcrypto_key_mode_kOtcryptoKeyModeHmacSha256}
oneshot_hmac_impl! {kernel::hil::oneshot_digest::HmacSha384, 48, otbindgen::otcrypto_key_mode_kOtcryptoKeyModeHmacSha384}
oneshot_hmac_impl! {kernel::hil::oneshot_digest::HmacSha512, 64, otbindgen::otcrypto_key_mode_kOtcryptoKeyModeHmacSha512}

// Implementation of oneshot KMACs.

macro_rules! oneshot_kmac_impl {
    {$trait:ty, $key_mode:expr, $kmac_mode:expr} => {
        impl $trait for OtCryptoOneshotDigest {
            fn digest(key: &ReadOnlyProcessBufferRef<'_>, input: &ReadOnlyProcessBufferRef<'_>, customization_string: &ReadOnlyProcessBufferRef<'_>, security_level: SecurityLevel, digest: &mut ReadWriteProcessBufferRef<'_>) -> Result<(), ErrorCode> {
                check_nonoverlapping(key, digest)?;
                check_nonoverlapping(input, digest)?;
                check_nonoverlapping(customization_string, digest)?;
                // SAFETY: `key`, `input`, and `digest` live long enough for the
                // cryptolib APIs used.
                //
                // `digest` was manually checked to not overlap with `key`,
                // `input` or `customization_string`, as to not violate the
                // `const` property on the latter three.
                unsafe {
                    // Allocate twice the max key length for the blinded key.
                    let mut blinded_buf = [0u32; 2 * MAX_KEY_LEN / size_of::<u32>()];
                    let blinded_key = blind_key($key_mode, key, &mut blinded_buf, security_level)?;
                    let input_message = OtCryptoConstByteBuf {
                        data: input.ptr(),
                        len: input.len(),
                    };
                    // KMAC customization string
                    let kmac_customization_string = OtCryptoConstByteBuf {
                        data: customization_string.ptr(),
                        len: customization_string.len(),
                    };
                    let output_len_bytes = digest.len();
                    let output_len_words = output_len_bytes / size_of::<u32>();
                    // Digest structure for hash digest operations
                    //
                    // SAFETY: We uphold the invariant of `digest_aligned_ptr`
                    // by not using `digest` in an immutable context while
                    // `aligned_ptr` is in-scope.
                    let aligned_ptr = digest_aligned_ptr(digest)?;
                    let tag = OtCryptoWord32Buf {
                        data: aligned_ptr,
                        len: output_len_words,
                    };
                    // SAFETY: `blinded_key` outlives the call to
                    // `otcrypto_hmac`, satisfying the constraints of
                    // `BlindedKey::into_raw`.
                    let raw = blinded_key.into_raw();
                    otcrypto_kmac(addr_of!(raw), input_message, $kmac_mode, kmac_customization_string, output_len_bytes, tag)
                        .check()
                        .map_err(|e| e.to_tock_err())
                }
            }
        }
    }
}

oneshot_kmac_impl! {
    kernel::hil::oneshot_digest::Kmac128,
    otbindgen::otcrypto_key_mode_kOtcryptoKeyModeKmac128,
    otbindgen::otcrypto_kmac_mode_kOtcryptoKmacModeKmac128
}
oneshot_kmac_impl! {
    kernel::hil::oneshot_digest::Kmac256,
    otbindgen::otcrypto_key_mode_kOtcryptoKeyModeKmac256,
    otbindgen::otcrypto_kmac_mode_kOtcryptoKmacModeKmac256
}

/// Verifies the allocations `immutable` and `mutable` do not overlap in
/// memory.
///
/// # Returns
///
/// + Ok(()): If the allocations do not overlap
/// + Err(ErrorCode::NOMEM): If the allocations overlap
///
/// A `ReadableProcessBuffer` is in fact weaker than a `&[u8]`, because the
/// former may overlap with a mutable slice if the application `allow`ed two
/// overlapping memory chunks in different syscalls. In the cryptolib API calls
/// in this file, we need to ensure the `const uintX_t *` parameters do not
/// overlap with any (mutable) `uintY_t *` parameters, as this would violate the
/// `const` qualifier on the former, causing undefined behavior.
pub fn check_nonoverlapping(
    immutable: &ReadOnlyProcessBufferRef<'_>,
    mutable: &mut ReadWriteProcessBufferRef<'_>,
) -> Result<(), ErrorCode> {
    // OVERFLOW: Neither of these `wrapping_add` calls can overflow, because
    // otherwise Tock could not allocate `*.len()` addressable bytes in the
    // buffer.
    if immutable.ptr().wrapping_add(immutable.len()) <= mutable.ptr()
        || mutable.ptr().wrapping_add(mutable.len()) <= immutable.ptr()
    {
        Ok(())
    } else {
        Err(ErrorCode::NOMEM)
    }
}

/// Converts a reference to a writable process-allocated buffer to a `*mut u32`,
/// if the buffer is properly aligned.
///
/// # Returns
///
/// + Ok(..), if the buffer is 4-byte aligned.
/// + Err(ErrorCode::NOMEM), if the buffer is improperly aligned.
///
/// # Safety
///
/// The caller must ensure the returned pointer and the original buffer are not
/// accessed simultaneously (if at least one of the accesses is a write).
pub unsafe fn digest_aligned_ptr(
    buf: &mut ReadWriteProcessBufferRef<'_>,
) -> Result<*mut u32, ErrorCode> {
    let maybe_aligned = buf.ptr() as *const u32;
    if buf.len() % size_of::<u32>() == 0 && maybe_aligned.align_offset(size_of::<u32>()) == 0 {
        Ok(maybe_aligned as *mut u32)
    } else {
        Err(ErrorCode::NOMEM)
    }
}
