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

/// NIST curve P-256 (OID 1.2.840.10045.3.1)
/// Source: `<https://oidref.com/1.2.840.10045.3.1>`
pub struct P256;
impl EllipticCurve for P256 {
    const OID: &'static str = "1.2.840.10045.3.1";
    const HASH_LEN: usize = 32;
    const SIG_LEN: usize = 64;
    const COORD_LEN: usize = 32;
}

// TODO: if `#![feature(generic_const_items)]` is stabilized, we could
// implement a generic version of this trait:
//
// `trait Ecdsa<'a, Curve> where Curve: EllipticCurve`

/// HIL for backends that implement the Elliptic-curve Digital
/// Signature Algorithm (ECDSA) using a particular elliptic curve.
pub trait EcdsaP256<'a> {
    /// Set the client instance which will receive the `verification_done()`
    /// callback.
    fn set_verify_client(
        &self,
        client: &'a dyn ClientVerify<{ P256::HASH_LEN }, { P256::SIG_LEN }>,
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
        hash: &'static mut [u8; P256::HASH_LEN],
        signature: &'static mut [u8; P256::SIG_LEN],
    ) -> Result<
        (),
        (
            ErrorCode,
            &'static mut [u8; P256::HASH_LEN],
            &'static mut [u8; P256::SIG_LEN],
        ),
    >;
}

// Blanket `impl` of `SignatureVerify` for implementations of `EcdsaP256`.
impl<'a, Impl> SignatureVerify<'a, { P256::HASH_LEN }, { P256::SIG_LEN }> for Impl
where
    Impl: EcdsaP256<'a>,
{
    /// Set the client instance which will receive the `verification_done()`
    /// callback.
    fn set_verify_client(
        &self,
        client: &'a dyn ClientVerify<{ P256::HASH_LEN }, { P256::SIG_LEN }>,
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
        EcdsaP256::verify(self, hash, signature)
    }
}

/// This trait provides callbacks for when the verification has completed.
pub trait EcdsaP256Client {
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
        hash: &'static mut [u8; P256::HASH_LEN],
        signature: &'static mut [u8; P256::SIG_LEN],
    );
}

impl<Impl> ClientVerify<{ P256::HASH_LEN }, { P256::SIG_LEN }> for Impl
where
    Impl: EcdsaP256Client,
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
        hash: &'static mut [u8; P256::HASH_LEN],
        signature: &'static mut [u8; P256::SIG_LEN],
    ) {
        EcdsaP256Client::verification_done(self, result, hash, signature)
    }
}
