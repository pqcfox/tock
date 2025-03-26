// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::hil::public_key_crypto::signature::{ClientVerify, SignatureVerify};
use crate::ErrorCode;

pub struct CurveParams {
    pub oid: &'static str,
    pub hash_len: usize,
    pub sig_len: usize,
    pub coord_len: usize,
}

/// Marker trait that designates the parameter lengths of elliptic curves.
pub trait EllipticCurve {
    const OID: &'static str;
    const HASH_LEN: usize;
    const SIG_LEN: usize;
    const COORD_LEN: usize;

    fn curve_params() -> CurveParams {
        CurveParams {
            oid: Self::OID,
            hash_len: Self::HASH_LEN,
            sig_len: Self::SIG_LEN,
            coord_len: Self::COORD_LEN,
        }
    }
}

/// NIST curve P-256, a.k.a. secp256r1 (OID 1.2.840.10045.3.1.7)
///
/// Source: `<https://oid-rep.orange-labs.fr/get/1.2.840.10045.3.1.7>`
pub struct P256;
impl EllipticCurve for P256 {
    const OID: &'static str = "1.2.840.10045.3.1.7";
    const HASH_LEN: usize = 32;
    const SIG_LEN: usize = 64;
    const COORD_LEN: usize = 32;
}

/// NIST curve P-384, a.k.a. secp384r1 (OID 1.3.132.0.34)
///
/// Source: `<https://oid-rep.orange-labs.fr/get/1.3.132.0.34>`
pub struct P384;
impl EllipticCurve for P384 {
    const OID: &'static str = "1.3.132.0.34";
    const HASH_LEN: usize = 48;
    const SIG_LEN: usize = 96;
    const COORD_LEN: usize = 48;
}

/// Hash digest modes used for ECDSA.
#[derive(Clone, Copy, Debug)]
pub enum HashMode {
    /// SHA-2 with 256-bit digest.
    Sha256,
    /// SHA-2 with 384-bit digest.
    Sha384,
    /// SHA-2 with 512-bit digest.
    Sha512,
    /// SHA-3 with 256-bit digest.
    Sha3_256,
    /// SHA-3 with 348-bit digest.
    Sha3_384,
    /// SHA-3 with 512-bit digest.
    Sha3_512,
}

pub trait SetHashMode {
    /// Set the hash mode used for the message digest input to ECDSA.
    ///
    /// # Returns
    ///
    /// + Ok(()): if the operation succeeded.
    /// + Err(ErrorCode::INVAL): If the hash mode is not compatible with the
    /// implementation (curve).
    fn set_hash_mode(&self, hash_mode: HashMode) -> Result<(), ErrorCode>;
}

macro_rules! ecdsa_hil {
    {$curve_type:ident, $trait:ident, $verify_client_trait:ident} => {
        /// HIL for backends that implement the Elliptic-curve Digital
        /// Signature Algorithm (ECDSA) using a particular elliptic curve.
        pub trait $trait<'a> {
            /// Set the client instance which will receive the `verification_done()`
            /// callback.
            fn set_verify_client(
                &self,
                client: &'a dyn ClientVerify<{ $curve_type::HASH_LEN }, { $curve_type::SIG_LEN }>,
            );

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
                hash: &'static mut [u8; $curve_type::HASH_LEN],
                signature: &'static mut [u8; $curve_type::SIG_LEN],
            ) -> Result<
                (),
            (
                ErrorCode,
                &'static mut [u8; $curve_type::HASH_LEN],
                &'static mut [u8; $curve_type::SIG_LEN],
            ),
            >;
        }

        // Blanket `impl` of `SignatureVerify` for implementations of ECDSA trait
        impl<'a, Impl> SignatureVerify<'a, { $curve_type::HASH_LEN }, { $curve_type::SIG_LEN }> for Impl
        where
        Impl: $trait<'a>,
        {
            /// Set the client instance which will receive the `verification_done()`
            /// callback.
            fn set_verify_client(
                &self,
                client: &'a dyn ClientVerify<{ $curve_type::HASH_LEN }, { $curve_type::SIG_LEN }>,
            ) {
                Self::set_verify_client(self, client)
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
                hash: &'static mut [u8; $curve_type::HASH_LEN],
                signature: &'static mut [u8; $curve_type::SIG_LEN],
            ) -> Result<
                (),
            (
                ErrorCode,
                &'static mut [u8; $curve_type::HASH_LEN],
                &'static mut [u8; $curve_type::SIG_LEN],
            ),
            > {
                $trait::verify(self, hash, signature)
            }
        }

        /// This trait provides callbacks for when the verification has completed.
        pub trait $verify_client_trait {
            /// Called when the verification is complete.
            ///
            /// If the verification operation encounters an error, result will be a
            /// `Result::Err()` specifying the ErrorCode. Otherwise, result will be a
            /// `Result::Ok` set to `Ok(true)` if the signature was correctly verified
            /// and `Ok(false)` otherwise.
            ///
            /// If verification operation did encounter errors `result` will be `Err()`
            /// with an appropriate `ErrorCode`. Valid `ErrorCode`s include:
            ///
            /// - `CANCEL`: the operation was cancelled.
            /// - `FAIL`: an internal failure.
            fn verification_done(
                &self,
                result: Result<bool, ErrorCode>,
                hash: &'static mut [u8; $curve_type::HASH_LEN],
                signature: &'static mut [u8; $curve_type::SIG_LEN],
            );
        }

        impl<Impl> ClientVerify<{ $curve_type::HASH_LEN }, { $curve_type::SIG_LEN }> for Impl
        where
        Impl: $verify_client_trait,
        {
            /// Called when the verification is complete.
            ///
            /// If the verification operation encounters an error, result will be a
            /// `Result::Err()` specifying the ErrorCode. Otherwise, result will be a
            /// `Result::Ok` set to `Ok(true)` if the signature was correctly verified
            /// and `Ok(false)` otherwise.
            ///
            /// If verification operation did encounter errors `result` will be `Err()`
            /// with an appropriate `ErrorCode`. Valid `ErrorCode`s include:
            ///
            /// - `CANCEL`: the operation was cancelled.
            /// - `FAIL`: an internal failure.
            fn verification_done(
                &self,
                result: Result<bool, ErrorCode>,
                hash: &'static mut [u8; $curve_type::HASH_LEN],
                signature: &'static mut [u8; $curve_type::SIG_LEN],
            ) {
                $verify_client_trait::verification_done(self, result, hash, signature)
            }
        }
    }
}

ecdsa_hil! {P256, EcdsaP256, EcdsaP256VerifyClient}
ecdsa_hil! {P384, EcdsaP384, EcdsaP384VerifyClient}
