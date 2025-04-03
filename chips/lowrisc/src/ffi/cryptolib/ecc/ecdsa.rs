// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use crate::ffi::cryptolib::mux::{CryptolibMux, OtbnJob, OtbnOperation};
use crate::ffi::hardened::HardenedBool;
use crate::ffi::status::Status;
use core::mem::size_of;
use core::ptr::{addr_of, addr_of_mut};
use kernel::hil::public_key_crypto::ecc::EllipticCurve;
use kernel::hil::public_key_crypto::ecc::{EcdsaP256, EcdsaP384};
use kernel::hil::public_key_crypto::ecc::{HashMode, SetHashMode};
use kernel::hil::public_key_crypto::ecc::{P256, P384};
use kernel::hil::public_key_crypto::keys::PubKeyMut;
use kernel::hil::public_key_crypto::signature::ClientVerify;
use kernel::hil::time::Alarm;
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::ErrorCode;
use otbindgen::{
    integrity_unblinded_checksum, otcrypto_const_word32_buf_t as OtCryptoConstWord32Buf,
    otcrypto_ecc_curve_t as OtCryptoEccCurve,
    otcrypto_ecc_curve_type_kOtcryptoEccCurveTypeNistP256 as CURVE_TYPE_P256,
    otcrypto_ecc_curve_type_kOtcryptoEccCurveTypeNistP384 as CURVE_TYPE_P384,
    otcrypto_ecdsa_verify_async_finalize, otcrypto_ecdsa_verify_async_start,
    otcrypto_hash_digest_t as OtCryptoHashDigest,
    otcrypto_hash_mode_kOtcryptoHashModeSha256 as HASH_MODE_SHA256,
    otcrypto_hash_mode_kOtcryptoHashModeSha384 as HASH_MODE_SHA384,
    otcrypto_hash_mode_kOtcryptoHashModeSha3_256 as HASH_MODE_SHA3_256,
    otcrypto_hash_mode_kOtcryptoHashModeSha3_384 as HASH_MODE_SHA3_384,
    otcrypto_hash_mode_kOtcryptoHashModeSha3_512 as HASH_MODE_SHA3_512,
    otcrypto_hash_mode_kOtcryptoHashModeSha512 as HASH_MODE_SHA512,
    otcrypto_key_mode_kOtcryptoKeyModeEcdsaP256 as KEY_MODE_ECDSA_P256,
    otcrypto_key_mode_kOtcryptoKeyModeEcdsaP384 as KEY_MODE_ECDSA_P384,
    otcrypto_unblinded_key_t as OtCryptoUnblindedKey,
};

/// Size in bytes of an OTBN wide data register
pub const WDR_SIZE: usize = 32;

macro_rules! ecdsa_driver {
    {
        driver = $driver:ident,
        curve = $curve:ident,
        hil = $hil:ident,
        curve_type = $curve_type:ident,
        key_mode = $key_mode:ident,
        verify_job = $verify_job:ident,
        otbn_operation = $otbn_operation:ident,
    } => {
        /// OTBN utility that verifies an ECDSA signature based on the public key
        /// derived from a private key sideloaded by the KeyManager driver.

        // TODO: when #![feature(generic_const_exprs)] is stabilized, we can
        // make the curve a generic parameter to this type.
        pub struct $driver<'a, A: Alarm<'a>> {
            cryptolib_mux: &'a CryptolibMux<'a, A>,
            verify_client: OptionalCell<&'a dyn ClientVerify<{ $curve::HASH_LEN }, { $curve::SIG_LEN }>>,
            /// Public key [x | y]
            public_key_buf: TakeCell<'static, [u8]>,
            /// The hash mode used to compute the digest
            hash_mode: OptionalCell<HashMode>,
            /// A self-reference, used for populating the `parent` field of the job
            /// struct, since the HIL traits give a reference with too weak a lifetime.
            self_reference: OptionalCell<&'a $driver<'a, A>>,
            verify_timeout: A::Ticks,
        }

        impl<'a, A: Alarm<'a>> $driver<'a, A> {
            pub fn new(
                cryptolib_mux: &'a CryptolibMux<'a, A>,
                verify_timeout: A::Ticks,
            ) -> $driver<'a, A> {
                $driver {
                    cryptolib_mux,
                    verify_client: OptionalCell::empty(),
                    public_key_buf: TakeCell::empty(),
                    hash_mode: OptionalCell::empty(),
                    self_reference: OptionalCell::empty(),
                    verify_timeout,
                }
            }

            /// Sets the public key for verification. If this is not called,
            /// the public key left over in the OTBN dmem is used instead.
            pub fn set_public_key_buf(&self, buf: &'static mut [u8]) {
                self.public_key_buf.put(Some(buf));
            }

            /// Invoked when a `verify` operation completes
            fn verify_done(
                &self,
                result: Result<bool, ErrorCode>,
                hash: &'static mut [u8; $curve::HASH_LEN],
                signature: &'static mut [u8; $curve::SIG_LEN],
            ) {
                self.verify_client
                    .map(|client| client.verification_done(result, hash, signature));
            }

            /// Call this before submitting jobs.
            ///
            /// This is a workaround so that `submit_otbn_job` understands the correct
            /// "outlives" relationship between `self` and the `CryptolibMux`, which the
            /// lifetimes on `self` in the HIL traits are too weak to communicate.
            pub fn set_self_ref(&'a self) {
                self.self_reference.set(self);
            }
        }

        /// ECDSA Job to submit to a `TimeoutMux`.
        pub struct $verify_job<'a, A: Alarm<'a>> {
            /// Driver responsible for the job
            parent: &'a $driver<'a, A>,
            /// The hash mode used to compute `unaligned_hash`.
            hash_mode: HashMode,
            /// Unaligned hash buffer; the client expects this back.
            unaligned_hash: TakeCell<'static, [u8; $curve::HASH_LEN]>,
            /// Unaligned signature buffer; the client expects this back.
            unaligned_signature: TakeCell<'static, [u8; $curve::SIG_LEN]>,
            /// 4-byte aligned public key buffer
            public_key: [u32; 2 * $curve::COORD_LEN / size_of::<u32>()],
            /// 4-byte aligned hash buffer
            hash: [u32; $curve::HASH_LEN / size_of::<u32>()],
            /// 4-byte aligned signature buffer
            signature: [u32; $curve::SIG_LEN / size_of::<u32>()],
        }

        impl<'a, A: Alarm<'a>> OtbnJob<'a, A> for $verify_job<'a, A> {
            /// Initialize an ECDSA verify cryptolib operation.
            fn setup(&mut self) -> Result<(), ErrorCode> {
                let mode = match self.hash_mode {
                    HashMode::Sha256 => HASH_MODE_SHA256,
                    HashMode::Sha384 => HASH_MODE_SHA384,
                    HashMode::Sha512 => HASH_MODE_SHA512,
                    HashMode::Sha3_256 => HASH_MODE_SHA3_256,
                    HashMode::Sha3_384 => HASH_MODE_SHA3_384,
                    HashMode::Sha3_512 => HASH_MODE_SHA3_512,
                };
                // SAFETY: The pointers to the public key, digest, and signature are
                // only valid within the context of this function. However, this is
                // sufficient because `integrity_unblinded_checksum` retains no state
                // and `otcrypto_ecdsa_verify_async_start` copies the data directly to
                // OTBN memory and discards the pointers to them afterwards.
                unsafe {
                    let mut public_key = OtCryptoUnblindedKey {
                        key_mode: $key_mode,
                        key_length: self.public_key.len() * size_of::<u32>(),
                        key: self.public_key.as_mut_ptr(),
                        checksum: 0xFFFF, // placeholder value
                    };
                    let message_digest = OtCryptoHashDigest {
                        mode,
                        data: self.hash.as_mut_ptr(),
                        // Hash length in 32-bit words
                        len: self.hash.len(),
                    };
                    let signature = OtCryptoConstWord32Buf {
                        data: self.signature.as_ptr(),
                        len: self.signature.len(),
                    };
                    // Populate the checksum
                    public_key.checksum = integrity_unblinded_checksum(addr_of!(public_key));
                    let elliptic_curve = OtCryptoEccCurve {
                        curve_type: $curve_type,
                        // NULL, because we use a named curve.
                        domain_parameter: core::ptr::null(),
                    };
                    let status = otcrypto_ecdsa_verify_async_start(
                        addr_of!(public_key),
                        message_digest,
                        signature,
                        addr_of!(elliptic_curve),
                    );
                    status.check().map_err(|e| (e.to_tock_err()))
                }
            }

            fn parent(&mut self) -> &'a CryptolibMux<'a, A> {
                self.parent.cryptolib_mux
            }

            /// Handler for when an ECDSA verify operation is complete
            fn on_complete(&mut self, status: Result<(), ErrorCode>) {
                // Grab the unaligned buffers we promised to return to the client. This
                // should never fail; if it does, we have no hope of making an upcall,
                // so just return.
                let hash = match self.unaligned_hash.take() {
                    None => return,
                    Some(h) => h,
                };
                let signature = match self.unaligned_signature.take() {
                    None => return,
                    Some(s) => s,
                };
                match status {
                    Err(e) => self.parent.verify_done(Err(e), hash, signature),
                    Ok(()) => {
                        let mut verification_result = HardenedBool::from(false).to_native();
                        // SAFETY: the signature in the internal state of the job never
                        // changes and we pass the same elliptic curve argument, so we
                        // uphold the cryptolib requirement that these parameters
                        // `_async_start` and `_async_finalize` call are
                        // consistent. `self.signature` and `verification_result` are
                        // guaranteed by the type system to be properly-aligned pointers
                        // to valid memory.
                        self.parent.verify_done(
                            unsafe {
                                let signature = OtCryptoConstWord32Buf {
                                    data: self.signature.as_ptr(),
                                    len: self.signature.len(),
                                };
                                let elliptic_curve = OtCryptoEccCurve {
                                    curve_type: $curve_type,
                                    // NULL, because we use a named curve.
                                    domain_parameter: core::ptr::null(),
                                };
                                otcrypto_ecdsa_verify_async_finalize(
                                    addr_of!(elliptic_curve),
                                    signature,
                                    addr_of_mut!(verification_result),
                                )
                                    .check()
                            }
                            .map_err(|e| e.to_tock_err())
                                .and_then(|()| {
                                    // No error; check the verification result.
                                    HardenedBool::from(verification_result)
                                        .try_into()
                                    // This branch occurs if the `HardendedBool` was an
                                    // invalid value.
                                        .map_err(|_| ErrorCode::FAIL)
                                }),
                            hash,
                            signature,
                        );
                    }
                }
            }

            fn on_timeout(&self) {
                // Grab the unaligned buffers we promised to return to the client. This
                // should never fail; if it does, we have no hope of making an upcall,
                // so just return.
                let hash = match self.unaligned_hash.take() {
                    None => return,
                    Some(h) => h,
                };
                let signature = match self.unaligned_signature.take() {
                    None => return,
                    Some(s) => s,
                };
                self.parent
                    .verify_done(Err(ErrorCode::FAIL), hash, signature);
            }
        }

        impl<'a, A: Alarm<'a>> $hil<'a> for $driver<'a, A> {
            /// Set the client instance which will receive the `verification_done()`
            /// callback.
            fn set_verify_client(
                &self,
                client: &'a dyn ClientVerify<{ $curve::HASH_LEN }, { $curve::SIG_LEN }>,
            ) {
                self.verify_client.set(client);
            }

            /// Verify the signature matches the given hash.
            ///
            /// If this returns `Ok(())`, then the `verification_done()` callback will
            /// be called. If this returns `Err()`, no callback will be called.
            ///
            /// The valid `ErrorCode`s that can occur are:
            ///
            /// - `OFF`: the underlying digest engine is powered down and cannot be
            ///   used.
            /// - `BUSY`: there is an outstanding operation already in process, and the
            ///   verification engine cannot accept another request.
            fn verify(
                &self,
                hash: &'static mut [u8; $curve::HASH_LEN],
                signature: &'static mut [u8; $curve::SIG_LEN],
            ) -> Result<
                (),
            (
                ErrorCode,
                &'static mut [u8; $curve::HASH_LEN],
                &'static mut [u8; $curve::SIG_LEN],
            ),
            > {
                if self.public_key_buf.is_none() {
                    return Err((ErrorCode::INVAL, hash, signature));
                }
                // Get the self-reference, which has a stronger lifetime bound than the
                // current function's context.
                //
                // If this fails, the board definition failed to call `setup()`.
                let parent: &'a Self = match self.self_reference.get() {
                    Some(p) => p,
                    None => return Err((ErrorCode::INVAL, hash, signature)),
                };
                // Take out the hash mode so the caller cannot accidentally use
                // a stale value for another verification.
                let hash_mode = match self.hash_mode.take() {
                    Some(h) => h,
                    // Return an error if the caller did not set the hash mode.
                    None => return Err((ErrorCode::RESERVE, hash, signature)),
                };
                // PANIC: We explicitly checked the `None` case separately to avoid a
                // lifetime conflict.
                self.public_key_buf
                    .map(|public_key| {
                        let mut state = $verify_job {
                            parent,
                            hash_mode,
                            unaligned_hash: TakeCell::empty(),
                            unaligned_signature: TakeCell::empty(),
                            hash: [0u32; $curve::HASH_LEN / size_of::<u32>()],
                            signature: [0u32; $curve::SIG_LEN / size_of::<u32>()],
                            public_key: [0u32; 2 * $curve::COORD_LEN / size_of::<u32>()],
                        };
                        // Copy verification parameters to 4-byte aligned slices.
                        memcpy_u8_u32(hash, &mut state.hash);
                        memcpy_u8_u32(signature, &mut state.signature);
                        memcpy_u8_u32(public_key, &mut state.public_key);
                        // Populate unaligned buffers so we can return them to the
                        // client when the operation finishes.
                        state.unaligned_hash.put(Some(hash));
                        state.unaligned_signature.put(Some(signature));
                        self.cryptolib_mux
                            .submit_otbn_job(
                                OtbnOperation::$otbn_operation(state),
                                self.verify_timeout,
                            )
                            .map_err(|(e, op)| {
                                match op {
                                    OtbnOperation::$otbn_operation(state) => (
                                        e,
                                        // PANIC: These `unwrap`s cannot panic because we just
                                        // populated those `TakeCell`s above. Unfortunately the
                                        // `unwrap` is unavoidable here because the API requires
                                        // returning the original buffers on error.
                                        state.unaligned_hash.take().unwrap(),
                                        state.unaligned_signature.take().unwrap(),
                                    ),
                                    // PANIC: We explicitly set this variant above, so
                                    // `op` cannot be anything else.
                                    //
                                    _ => unreachable!(),
                                }
                            })
                    })
                    .unwrap()
            }
        }

        impl<'a, A: Alarm<'a>> PubKeyMut for $driver<'a, A> {
            /// Import an existing public key.
            ///
            /// The reference to the `public_key` is stored internally and can be
            /// retrieved with the `pub_key()` function.
            /// The `public_key` can be either a mutable static or an immutable static,
            /// depending on where the key is stored (flash or memory).
            ///
            /// The possible ErrorCodes are:
            ///     - `BUSY`: A key is already imported or in the process of being
            ///               generated.
            ///     - `INVAL`: An invalid key was supplied.
            ///     - `SIZE`: An invalid key size was supplied.
            fn import_public_key(
                &self,
                public_key: &'static mut [u8],
            ) -> Result<(), (ErrorCode, &'static mut [u8])> {
                self.set_public_key_buf(public_key);
                Ok(())
            }

            /// Return the public key supplied by `import_public_key()` or
            /// `generate()`.
            ///
            /// On success the return value is `Ok(())` with the buffer that was
            /// originally passed in to hold the key.
            ///
            /// On failure the possible ErrorCodes are:
            ///     - `NODEVICE`: The key does not exist
            fn pub_key(&self) -> Result<&'static mut [u8], ErrorCode> {
                self.public_key_buf.take().ok_or(ErrorCode::NODEVICE)
            }

            /// Report the length of the public key in bytes, as returned from `pub_key()`.
            /// A value of 0 indicates that the key does not exist.
            fn len(&self) -> usize {
                self.public_key_buf.map(|p| p.len()).unwrap_or(0)
            }
        }
    }
}

ecdsa_driver! {
    driver = OtCryptoEcdsaP256,
    curve = P256,
    hil = EcdsaP256,
    curve_type = CURVE_TYPE_P256,
    key_mode = KEY_MODE_ECDSA_P256,
    verify_job = EcdsaVerifyP256Job,
    otbn_operation = EcdsaVerifyP256,
}
ecdsa_driver! {
    driver = OtCryptoEcdsaP384,
    curve = P384,
    hil = EcdsaP384,
    curve_type = CURVE_TYPE_P384,
    key_mode = KEY_MODE_ECDSA_P384,
    verify_job = EcdsaVerifyP384Job,
    otbn_operation = EcdsaVerifyP384,
}

impl<'a, A: Alarm<'a>> SetHashMode for OtCryptoEcdsaP256<'a, A> {
    fn set_hash_mode(&self, hash_mode: HashMode) -> Result<(), ErrorCode> {
        Ok(self.hash_mode.set(hash_mode))
    }
}

impl<'a, A: Alarm<'a>> SetHashMode for OtCryptoEcdsaP384<'a, A> {
    fn set_hash_mode(&self, hash_mode: HashMode) -> Result<(), ErrorCode> {
        match hash_mode {
            // 256-bit digests are too short for P-384.
            HashMode::Sha256 | HashMode::Sha3_256 => Err(ErrorCode::INVAL),
            hash_mode => Ok(self.hash_mode.set(hash_mode)),
        }
    }
}

/// Helper that groups bytes in the `u8` slice into `u32` and writes them to
/// `dest`. Zero-pads the last element of `dest` if `src.len()` is not a
/// multiple of 4. Any indices in `dest` that do not overlap with the length of
/// `src` are not changed.
///
/// # Panics
///
/// If `dest` is not long enough to hold all of the bytes in `src`.
fn memcpy_u8_u32(src: &[u8], dest: &mut [u32]) {
    let mut i = 0;
    while i < src.len() {
        dest[i / 4] = u32::from_ne_bytes([
            src[i],
            *src.get(i + 1).unwrap_or(&0),
            *src.get(i + 2).unwrap_or(&0),
            *src.get(i + 3).unwrap_or(&0),
        ]);
        i += 4;
    }
}

/// Tests for ECDSA with cryptolib
#[cfg(feature = "test_cryptolib")]
pub mod tests {
    use super::*;
    use core::cell::Cell;

    // Project wycheproof ECDSA secp256r1 SHA-256, test case #1
    // (https://github.com/C2SP/wycheproof/blob/master/testvectors/ecdsa_secp256r1_sha256_test.json#L32)

    /// Public value `X` from the test vector (little-endian).
    const P256_X: [u8; 32] = [
        0x38, 0x28, 0x73, 0x6c, 0xdf, 0xc4, 0xc8, 0x69, 0x60, 0x8, 0xf7, 0x19, 0x99, 0x26, 0x3,
        0x29, 0xad, 0x8b, 0x12, 0x28, 0x78, 0x46, 0xfe, 0xdc, 0xed, 0xe3, 0xba, 0x12, 0x5, 0xb1,
        0x27, 0x29,
    ];
    /// Public value `Y` from the test vector (little-endian).
    const P256_Y: [u8; 32] = [
        0x3e, 0x51, 0x41, 0x73, 0x4e, 0x97, 0x1a, 0x8d, 0x55, 0x1, 0x50, 0x68, 0xd9, 0xb3, 0x66,
        0x67, 0x60, 0xf4, 0x60, 0x8a, 0x49, 0xb1, 0x1f, 0x92, 0xe5, 0x0, 0xac, 0xea, 0x64, 0x79,
        0x78, 0xc7,
    ];

    /// SHA-256 hash of the hex byte string `313233343030`, from the test vector.
    const P256_DIGEST: [u8; 32] = [
        0xbb, 0x5a, 0x52, 0xf4, 0x2f, 0x9c, 0x92, 0x61, 0xed, 0x43, 0x61, 0xf5, 0x94, 0x22, 0xa1,
        0xe3, 0x00, 0x36, 0xe7, 0xc3, 0x2b, 0x27, 0x0c, 0x88, 0x07, 0xa4, 0x19, 0xfe, 0xca, 0x60,
        0x50, 0x23,
    ];

    /// Signuature value `R`, decoded from the DER sequence in the test vector (little-endian).
    const P256_R: [u8; 32] = [
        0x18, 0x2e, 0x5c, 0xbd, 0xf9, 0x6a, 0xcc, 0xb8, 0x59, 0xe8, 0xee, 0xa1, 0x85, 0xd, 0xe5,
        0xff, 0x6e, 0x43, 0xa, 0x19, 0xd1, 0xd9, 0xa6, 0x80, 0xec, 0xd5, 0x94, 0x6b, 0xbe, 0xa8,
        0xa3, 0x2b,
    ];
    /// Signuature value `S`, decoded from the DER sequence in the test vector (little-endian).
    const P256_S: [u8; 32] = [
        0x76, 0xdd, 0xfa, 0xe6, 0x79, 0x7f, 0xa6, 0x77, 0x7c, 0xaa, 0xb9, 0xfa, 0x10, 0xe7, 0x5f,
        0x52, 0xe7, 0xa, 0x4e, 0x6c, 0xeb, 0x11, 0x7b, 0x3c, 0x5b, 0x2f, 0x44, 0x5d, 0x85, 0xb,
        0xd6, 0x4c,
    ];

    /// Public value `X` from the test vector (little-endian).
    const P384_X: [u8; 48] = [
        0xaa, 0x38, 0x21, 0x75, 0xc5, 0x8c, 0xd9, 0x72, 0x97, 0x38, 0x74, 0x82, 0x88, 0xff, 0x47,
        0xf, 0xf2, 0xb2, 0xbd, 0xe4, 0xfe, 0xd9, 0xbf, 0xd9, 0xe7, 0x80, 0x72, 0xeb, 0x71, 0xad,
        0x6c, 0x97, 0xd, 0xff, 0xb, 0xac, 0xfd, 0x9f, 0x3f, 0x54, 0x6a, 0x27, 0x89, 0x10, 0xda,
        0x7d, 0xa5, 0x2d,
    ];

    /// Public value `Y` from the test vector (little-endian).
    const P384_Y: [u8; 48] = [
        0x4f, 0x17, 0x6, 0x44, 0x27, 0x14, 0xf0, 0x88, 0x4d, 0x9e, 0x1f, 0x9, 0xd6, 0xb0, 0x7,
        0x74, 0xbc, 0x1b, 0x57, 0xf6, 0x78, 0x2e, 0x96, 0xad, 0xf8, 0x76, 0x7d, 0x19, 0x36, 0x18,
        0x3b, 0x88, 0x34, 0x5e, 0x71, 0x70, 0xf8, 0x9d, 0xc4, 0x5e, 0xe2, 0xf3, 0xdc, 0x69, 0x4d,
        0x5, 0x6d, 0x4b,
    ];

    /// SHA-384 hash of the hex byte string `313233343030`, from the test vector.
    const P384_DIGEST: [u8; 48] = [
        0xf9, 0xb1, 0x27, 0xf0, 0xd8, 0x1e, 0xbc, 0xd1, 0x7b, 0x7b, 0xa0, 0xea, 0x13, 0x1c, 0x66,
        0xd, 0x34, 0xb, 0x5, 0xce, 0x55, 0x7c, 0x82, 0x16, 0xe, 0xf, 0x79, 0x3d, 0xe0, 0x7d, 0x38,
        0x17, 0x90, 0x23, 0x94, 0x28, 0x71, 0xac, 0xb7, 0x0, 0x2d, 0xfa, 0xfd, 0xff, 0xfc, 0x8d,
        0xea, 0xce,
    ];

    /// Signuature value `R`, decoded from the DER sequence in the test vector (little-endian).
    const P384_R: [u8; 48] = [
        0xd7, 0x48, 0xc5, 0xf1, 0x94, 0x33, 0xbd, 0xa, 0x32, 0xa0, 0x45, 0xcc, 0xe4, 0x4a, 0x8e,
        0xba, 0x83, 0x30, 0x11, 0xe3, 0x28, 0xca, 0xf2, 0xda, 0x19, 0xfe, 0x1b, 0x4b, 0xb4, 0x26,
        0x1e, 0x66, 0x25, 0x4, 0x7c, 0x55, 0xae, 0x12, 0xb6, 0xe6, 0x6f, 0x47, 0xb5, 0xf6, 0xbe,
        0xa, 0xb3, 0x12,
    ];

    /// Signuature value `S`, decoded from the DER sequence in the test vector (little-endian).
    const P384_S: [u8; 48] = [
        0xf1, 0xb9, 0x82, 0xcd, 0xee, 0x54, 0xff, 0x9c, 0xaa, 0xc7, 0x8a, 0xb5, 0x9, 0x62, 0xbf,
        0x5, 0x38, 0x51, 0x9, 0xe, 0x24, 0x30, 0xca, 0x95, 0x39, 0xa0, 0x4c, 0x8d, 0x3a, 0xee,
        0x74, 0x25, 0x8c, 0x3b, 0x41, 0xd5, 0x85, 0xf4, 0xc, 0x90, 0xf8, 0xf8, 0xd2, 0xc1, 0x9f,
        0xda, 0x40, 0x18,
    ];

    macro_rules! test_ecdsa_verify {

        {$test_func:ident, $curve:ident, $curve_name:expr, $driver:ident, $client:ident, $hash_mode:ident, $digest:ident, $r:ident, $s:ident, $x:ident, $y:ident} => {
            pub struct $client {
                expected: Cell<bool>,
                hash_buf: TakeCell<'static, [u8; $curve::HASH_LEN]>,
                sig_buf: TakeCell<'static, [u8; $curve::SIG_LEN]>,
                hash_buf_2: TakeCell<'static, [u8; $curve::HASH_LEN]>,
                sig_buf_2: TakeCell<'static, [u8; $curve::SIG_LEN]>,
            }

            impl $client {
                pub fn new(
                    hash_buf: &'static mut [u8; $curve::HASH_LEN],
                    sig_buf: &'static mut [u8; $curve::SIG_LEN],
                    hash_buf_2: &'static mut [u8; $curve::HASH_LEN],
                    sig_buf_2: &'static mut [u8; $curve::SIG_LEN],
                ) -> Self {
                    Self {
                        expected: Cell::new(true),
                        hash_buf: TakeCell::new(hash_buf),
                        sig_buf: TakeCell::new(sig_buf),
                        hash_buf_2: TakeCell::new(hash_buf_2),
                        sig_buf_2: TakeCell::new(sig_buf_2),
                    }
                }
            }

            impl ClientVerify<{ $curve::HASH_LEN }, { $curve::SIG_LEN }> for $client {
                fn verification_done(
                    &self,
                    result: Result<bool, ErrorCode>,
                    _hash: &'static mut [u8; $curve::HASH_LEN],
                    _signature: &'static mut [u8; $curve::SIG_LEN],
                ) {
                    // Check the verification outcome
                    assert_eq!(
                        self.expected.get(),
                        result.expect("ECDSA verification failed due to an error."),
                        "ECDSA verification failed with incorrect validation result.",
                    );
                    if self.expected.get() {
                        // Postitive test done; reset for negative test.
                        self.expected.set(false);
                        kernel::debug!("Testing invalid signature.");
                    } else {
                        // Negative test done; reset for next curve.
                        self.expected.set(false);
                        kernel::debug!("ECDSA {} verification test successful.", $curve_name);
                    }
                }
            }

            pub fn $test_func<'a, A: Alarm<'a>>(
                driver: &'a $driver<'a, A>,
                test_client: &'a $client,
                pub_key_buf: &'static mut [u8; 2 * $curve::COORD_LEN],
            ) {
                kernel::debug!(
                    "Testing ECDSA {}.\n\
                     Testing valid signature.",
                    $curve_name,
                );

                // ** Positive test: check verification succeeds. **

                let hash_buf = test_client
                    .hash_buf
                    .take()
                    .expect("Failed to take `hash_buf`");
                let sig_buf = test_client
                    .sig_buf
                    .take()
                    .expect("Failed to take `sig_buf`");

                // Set the digest value.
                hash_buf[..].clone_from_slice(&$digest);

                // Set the signature.
                let len = sig_buf.len();
                sig_buf[..len / 2].clone_from_slice(&$r);
                sig_buf[len / 2..].clone_from_slice(&$s);

                // Set the public key ([x | y]).
                let len = pub_key_buf.len();
                pub_key_buf[..len / 2].clone_from_slice(&$x);
                pub_key_buf[len / 2..].clone_from_slice(&$y);
                driver
                    .import_public_key(pub_key_buf)
                    .expect("Failed to import public key");

                // Set hash mode
                driver.set_hash_mode(HashMode::$hash_mode).expect("Failed to set hash mode");

                // Run ECDSA.
                driver
                    .verify(hash_buf, sig_buf)
                    .expect("ECDSA verification failed-fast with an error");

                // ** Negative test: use invalid signature, check verification fails. **

                let hash_buf = test_client
                    .hash_buf_2
                    .take()
                    .expect("Failed to take `hash_buf_2`");
                let sig_buf = test_client
                    .sig_buf_2
                    .take()
                    .expect("Failed to take `sig_buf_2`");

                // Set the digest value.
                hash_buf[..].clone_from_slice(&$digest);

                // Set the signature, changing one byte.
                let len = sig_buf.len();
                sig_buf[..len / 2].clone_from_slice(&$r);
                sig_buf[len / 2..].clone_from_slice(&$s);
                // Index was arbitrarily chosen.
                sig_buf[22] = sig_buf[22].wrapping_add(1);

                // Set hash mode
                driver.set_hash_mode(HashMode::$hash_mode).expect("Failed to set hash mode");

                // Run ECDSA.
                //
                // At this point, both jobs are scheduled, but we do not expect the
                // first job is complete at this time.
                driver
                    .verify(hash_buf, sig_buf)
                    .expect("ECDSA verification failed-fast with an error");
            }
        }
    }
    test_ecdsa_verify! {test_ecdsa_p256_verify, P256, "P-256", OtCryptoEcdsaP256, EcdsaP256TestClient, Sha256, P256_DIGEST, P256_R, P256_S, P256_X, P256_Y}
    test_ecdsa_verify! {test_ecdsa_p384_verify, P384, "P-384", OtCryptoEcdsaP384, EcdsaP384TestClient, Sha384, P384_DIGEST, P384_R, P384_S, P384_X, P384_Y}
}
