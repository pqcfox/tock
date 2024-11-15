// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use crate::ffi::cryptolib::integrity::IntegrityUnblindedChecksum;
use crate::ffi::cryptolib::mux::{CryptolibMux, OtbnJob, OtbnOperation};
use crate::ffi::hardened::HardenedBool;
use crate::ffi::status::Status;
use core::mem::size_of;
use core::ptr::{addr_of, addr_of_mut};
use cryptolib_ecc::{
    otcrypto_const_word32_buf_t as OtCryptoConstWord32Buf,
    otcrypto_ecc_curve_t as OtCryptoEccCurve,
    otcrypto_ecc_curve_type_kOtcryptoEccCurveTypeNistP256 as CURVE_TYPE_P256,
    otcrypto_ecdsa_verify_async_finalize, otcrypto_ecdsa_verify_async_start,
    otcrypto_hash_digest_t as OtCryptoHashDigest,
    otcrypto_hash_mode_kOtcryptoHashModeSha256 as HASH_MODE_SHA256,
    otcrypto_key_mode_kOtcryptoKeyModeEcdsa as KEY_MODE_ECDSA,
    otcrypto_unblinded_key_t as OtCryptoUnblindedKey,
};
use kernel::hil::public_key_crypto::ecc::EcdsaP256;
use kernel::hil::public_key_crypto::ecc::EllipticCurve;
use kernel::hil::public_key_crypto::ecc::P256;
use kernel::hil::public_key_crypto::keys::PubKeyMut;
use kernel::hil::public_key_crypto::signature::ClientVerify;
use kernel::hil::time::Alarm;
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::ErrorCode;

/// Size in bytes of an OTBN wide data register
pub const WDR_SIZE: usize = 32;

/// OTBN utility that verifies an ECDSA P-256 signature based on the
/// public key derived from a private key sideloaded by the KeyManager
/// driver.

// TODO: when #![feature(generic_const_exprs)] is stabilized, we can
// make the curve a generic parameter to this type.
pub struct OtCryptoEcdsaP256<'a, A: Alarm<'a>> {
    cryptolib_mux: &'a CryptolibMux<'a, A>,
    verify_client: OptionalCell<&'a dyn ClientVerify<{ P256::HASH_LEN }, { P256::SIG_LEN }>>,
    /// Public key [x | y]
    public_key_buf: TakeCell<'static, [u8]>,
    /// A self-reference, used for populating the `parent` field of the job
    /// struct, since the HIL traits give a reference with too weak a lifetime.
    self_reference: OptionalCell<&'a OtCryptoEcdsaP256<'a, A>>,
    ecdsa_verify_p256_timeout: A::Ticks,
}

impl<'a, A: Alarm<'a>> OtCryptoEcdsaP256<'a, A> {
    pub fn new(
        cryptolib_mux: &'a CryptolibMux<'a, A>,
        ecdsa_verify_p256_timeout: A::Ticks,
    ) -> OtCryptoEcdsaP256<'a, A> {
        OtCryptoEcdsaP256 {
            cryptolib_mux,
            verify_client: OptionalCell::empty(),
            public_key_buf: TakeCell::empty(),
            self_reference: OptionalCell::empty(),
            ecdsa_verify_p256_timeout,
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
        hash: &'static mut [u8; P256::HASH_LEN],
        signature: &'static mut [u8; P256::SIG_LEN],
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
pub struct EcdsaVerifyP256Job<'a, A: Alarm<'a>> {
    /// Driver responsible for the job
    parent: &'a OtCryptoEcdsaP256<'a, A>,
    /// Unaligned hash buffer; the client expects this back.
    unaligned_hash: TakeCell<'static, [u8; P256::HASH_LEN]>,
    /// Unaligned signature buffer; the client expects this back.
    unaligned_signature: TakeCell<'static, [u8; P256::SIG_LEN]>,
    /// 4-byte aligned public key buffer
    public_key: [u32; 2 * P256::COORD_LEN / size_of::<u32>()],
    /// 4-byte aligned hash buffer
    hash: [u32; P256::HASH_LEN / size_of::<u32>()],
    /// 4-byte aligned signature buffer
    signature: [u32; P256::SIG_LEN / size_of::<u32>()],
}

impl<'a, A: Alarm<'a>> OtbnJob<'a, A> for EcdsaVerifyP256Job<'a, A> {
    /// Initialize an ECDSA verify cryptolib operation.
    fn setup(&mut self) -> Result<(), ErrorCode> {
        // SAFETY: The pointers to the public key, digest, and signature are
        // only valid within the context of this function. However, this is
        // sufficient because `integrity_unblinded_checksum` retains no state
        // and `otcrypto_ecdsa_verify_async_start` copies the data directly to
        // OTBN memory and discards the pointers to them afterwards.
        unsafe {
            let mut public_key = OtCryptoUnblindedKey {
                key_mode: KEY_MODE_ECDSA,
                key_length: self.public_key.len() * size_of::<u32>(),
                key: self.public_key.as_mut_ptr(),
                checksum: 0xFFFF, // placeholder value
            };
            let message_digest = OtCryptoHashDigest {
                mode: HASH_MODE_SHA256,
                data: self.hash.as_mut_ptr(),
                // Hash length in 32-bit words
                len: self.hash.len(),
            };
            let signature = OtCryptoConstWord32Buf {
                data: self.signature.as_ptr(),
                len: self.signature.len(),
            };
            public_key.populate_checksum();
            let elliptic_curve = OtCryptoEccCurve {
                curve_type: CURVE_TYPE_P256,
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
        &self.parent.cryptolib_mux
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
                            curve_type: CURVE_TYPE_P256,
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

impl<'a, A: Alarm<'a>> EcdsaP256<'a> for OtCryptoEcdsaP256<'a, A> {
    /// Set the client instance which will receive the `verification_done()`
    /// callback.
    fn set_verify_client(
        &self,
        client: &'a dyn ClientVerify<{ P256::HASH_LEN }, { P256::SIG_LEN }>,
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
        hash: &'static mut [u8; P256::HASH_LEN],
        signature: &'static mut [u8; P256::SIG_LEN],
    ) -> Result<
        (),
        (
            ErrorCode,
            &'static mut [u8; P256::HASH_LEN],
            &'static mut [u8; P256::SIG_LEN],
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
        // PANIC: We explicitly checked the `None` case separately to avoid a
        // lifetime conflict.
        self.public_key_buf
            .map(|public_key| {
                let mut state = EcdsaVerifyP256Job {
                    parent,
                    unaligned_hash: TakeCell::empty(),
                    unaligned_signature: TakeCell::empty(),
                    hash: [0u32; P256::HASH_LEN / size_of::<u32>()],
                    signature: [0u32; P256::SIG_LEN / size_of::<u32>()],
                    public_key: [0u32; 2 * P256::COORD_LEN / size_of::<u32>()],
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
                        OtbnOperation::EcdsaVerifyP256(state),
                        self.ecdsa_verify_p256_timeout,
                    )
                    .map_err(|(e, op)| {
                        match op {
                            OtbnOperation::EcdsaVerifyP256(state) => (
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
                            // TODO: uncomment this when other operations are added.
                            // _ => unreachable!(),
                        }
                    })
            })
            .unwrap()
    }
}

impl<'a, A: Alarm<'a>> PubKeyMut for OtCryptoEcdsaP256<'a, A> {
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
    const X: [u8; 32] = [
        0x38, 0x28, 0x73, 0x6c, 0xdf, 0xc4, 0xc8, 0x69, 0x60, 0x8, 0xf7, 0x19, 0x99, 0x26, 0x3,
        0x29, 0xad, 0x8b, 0x12, 0x28, 0x78, 0x46, 0xfe, 0xdc, 0xed, 0xe3, 0xba, 0x12, 0x5, 0xb1,
        0x27, 0x29,
    ];
    /// Public value `Y` from the test vector (little-endian).
    const Y: [u8; 32] = [
        0x3e, 0x51, 0x41, 0x73, 0x4e, 0x97, 0x1a, 0x8d, 0x55, 0x1, 0x50, 0x68, 0xd9, 0xb3, 0x66,
        0x67, 0x60, 0xf4, 0x60, 0x8a, 0x49, 0xb1, 0x1f, 0x92, 0xe5, 0x0, 0xac, 0xea, 0x64, 0x79,
        0x78, 0xc7,
    ];

    /// SHA-256 hash of the hex byte string `313233343030`, from the test vector.
    const DIGEST: [u8; 32] = [
        0xbb, 0x5a, 0x52, 0xf4, 0x2f, 0x9c, 0x92, 0x61, 0xed, 0x43, 0x61, 0xf5, 0x94, 0x22, 0xa1,
        0xe3, 0x00, 0x36, 0xe7, 0xc3, 0x2b, 0x27, 0x0c, 0x88, 0x07, 0xa4, 0x19, 0xfe, 0xca, 0x60,
        0x50, 0x23,
    ];

    /// Signuature value `R`, decoded from the DER sequence in the test vector (little-endian).
    const R: [u8; 32] = [
        0x18, 0x2e, 0x5c, 0xbd, 0xf9, 0x6a, 0xcc, 0xb8, 0x59, 0xe8, 0xee, 0xa1, 0x85, 0xd, 0xe5,
        0xff, 0x6e, 0x43, 0xa, 0x19, 0xd1, 0xd9, 0xa6, 0x80, 0xec, 0xd5, 0x94, 0x6b, 0xbe, 0xa8,
        0xa3, 0x2b,
    ];
    /// Signuature value `S`, decoded from the DER sequence in the test vector (little-endian).
    const S: [u8; 32] = [
        0x76, 0xdd, 0xfa, 0xe6, 0x79, 0x7f, 0xa6, 0x77, 0x7c, 0xaa, 0xb9, 0xfa, 0x10, 0xe7, 0x5f,
        0x52, 0xe7, 0xa, 0x4e, 0x6c, 0xeb, 0x11, 0x7b, 0x3c, 0x5b, 0x2f, 0x44, 0x5d, 0x85, 0xb,
        0xd6, 0x4c,
    ];

    pub struct EcdsaP256TestClient {
        expected: Cell<bool>,
        hash_buf: TakeCell<'static, [u8; P256::HASH_LEN]>,
        sig_buf: TakeCell<'static, [u8; P256::SIG_LEN]>,
        hash_buf_2: TakeCell<'static, [u8; P256::HASH_LEN]>,
        sig_buf_2: TakeCell<'static, [u8; P256::SIG_LEN]>,
    }

    impl EcdsaP256TestClient {
        pub fn new(
            hash_buf: &'static mut [u8; P256::HASH_LEN],
            sig_buf: &'static mut [u8; P256::SIG_LEN],
            hash_buf_2: &'static mut [u8; P256::HASH_LEN],
            sig_buf_2: &'static mut [u8; P256::SIG_LEN],
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

    impl ClientVerify<{ P256::HASH_LEN }, { P256::SIG_LEN }> for EcdsaP256TestClient {
        fn verification_done(
            &self,
            result: Result<bool, ErrorCode>,
            _hash: &'static mut [u8; P256::HASH_LEN],
            _signature: &'static mut [u8; P256::SIG_LEN],
        ) {
            // Check the verification outcome
            assert_eq!(
                self.expected.get(),
                result.expect("ECDSA P-256 verification failed due to an error."),
                "ECDSA P-256 verification failed with incorrect validation result.",
            );
            if self.expected.get() {
                // Postitive test done; reset for negative test.
                self.expected.set(false);
                kernel::debug!("Testing invalid signature.");
            } else {
                // Negative test done; tests complete.
                kernel::debug!("ECDSA P-256 verification test PASSED.");
            }
        }
    }

    pub fn test_ecdsa_p256_verify<'a, A: Alarm<'a>>(
        driver: &'a OtCryptoEcdsaP256<'a, A>,
        test_client: &'a EcdsaP256TestClient,
        pub_key_buf: &'static mut [u8; 2 * P256::COORD_LEN],
    ) {
        kernel::debug!(
            "Starting ECDSA kernel test.\n\
             Testing valid signature."
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
        hash_buf[..].clone_from_slice(&DIGEST);

        // Set the signature.
        let len = sig_buf.len();
        sig_buf[..len / 2].clone_from_slice(&R);
        sig_buf[len / 2..].clone_from_slice(&S);

        // Set the public key ([x | y]).
        let len = pub_key_buf.len();
        pub_key_buf[..len / 2].clone_from_slice(&X);
        pub_key_buf[len / 2..].clone_from_slice(&Y);
        driver
            .import_public_key(pub_key_buf)
            .expect("Failed to import public key");

        // Run ECDSA.
        driver
            .verify(hash_buf, sig_buf)
            .expect("ECDSA P-256 verification failed-fast with an error");

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
        hash_buf[..].clone_from_slice(&DIGEST);

        // Set the signature, changing one byte.
        let len = sig_buf.len();
        sig_buf[..len / 2].clone_from_slice(&R);
        sig_buf[len / 2..].clone_from_slice(&S);
        // Index was arbitrarily chosen.
        sig_buf[22] = sig_buf[22].wrapping_add(1);

        // Run ECDSA.
        //
        // At this point, both jobs are scheduled, but we do not expect the
        // first job is complete at this time.
        driver
            .verify(hash_buf, sig_buf)
            .expect("ECDSA P-256 verification failed-fast with an error");
    }
}
