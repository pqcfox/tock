// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.  Confidential information of zeroRISC Inc. All rights
// reserved.

// HILs for performing hash functions in a synchronous, oneshot manner.

use crate::hil::key_transport::SecurityLevel;
use crate::processbuffer::{ReadOnlyProcessBufferRef, ReadWriteProcessBufferRef};
use crate::ErrorCode;
use core::mem::size_of;

macro_rules! oneshot_digest_hil {
    {$trait:ident, $width:expr} => {
        /// Performs a digest in a oneshot, synchronous manner.
        pub trait $trait {
            fn digest(input: &ReadOnlyProcessBufferRef<'_>, digest: &mut [u32; $width / size_of::<u32>()]) -> Result<(), ErrorCode>;
        }
    }
}

macro_rules! oneshot_shake_hil {
    {$trait:ident} => {
        /// Performs a SHAKE XOF digest in a oneshot, synchronous manner. The
        /// XOF output length is determined by `digest.len()`.
        ///
        /// Implementations are permitted to enforce certain alignment
        /// requirements on `digest` and return `Err(ErrorCode::NOMEM)` if they
        /// are violated.  For example, an implementation may require
        /// `digest.len()` must be divisible by 4 and `digest.ptr()` must be
        /// 4-byte aligned.
        ///
        /// # Safety
        ///
        /// The implementation is responsible for checking that `digest` does
        /// not overlap with `input`. If it does, it should return
        /// `ErrorCode::NOMEM`.
        pub trait $trait {
            fn digest(input: &ReadOnlyProcessBufferRef<'_>, digest: &mut ReadWriteProcessBufferRef<'_>) -> Result<(), ErrorCode>;
        }
    }
}

macro_rules! oneshot_cshake_hil {
    {$trait:ident} => {
        /// Performs a cSHAKE XOF digest in a oneshot, synchronous manner. The
        /// XOF output length is determined by `digest.len()`.
        ///
        /// Implementations are permitted to enforce certain alignment
        /// requirements on `digest` and return `Err(ErrorCode::NOMEM)` or
        /// `Err(ErrorCode::SIZE)` if they are violated. For example, an
        /// implementation may require `digest.len()` must be divisible by 4 and
        /// `digest.ptr()` must be 4-byte aligned.
        ///
        /// # Safety
        ///
        /// The implementation is responsible for checking that `digest` does
        /// not overlap with any of `input`, `function_name`, or
        /// `customization_string`. If it does, it should return
        /// `ErrorCode::NOMEM`.
        pub trait $trait {
            fn digest(
                input: &ReadOnlyProcessBufferRef<'_>,
                function_name: &ReadOnlyProcessBufferRef<'_>,
                customization_string: &ReadOnlyProcessBufferRef<'_>,
                digest: &mut ReadWriteProcessBufferRef<'_>,
            ) -> Result<(), ErrorCode>;
        }
    }
}

macro_rules! oneshot_hmac_hil {
    {$trait:ident, $width:expr} => {
        /// Performs a digest in a oneshot, synchronous manner.
        pub trait $trait {
            fn digest(key: &ReadOnlyProcessBufferRef<'_>, input: &ReadOnlyProcessBufferRef<'_>, security_level: SecurityLevel, digest: &mut [u32; $width / size_of::<u32>()]) -> Result<(), ErrorCode>;
        }
    }
}

macro_rules! oneshot_kmac_hil {
    {$trait:ident} => {
        /// Computes a KMAC message authentication code in a oneshot,
        /// synchronous manner. The required output length is determined by
        /// `digest.len()`.
        ///
        /// Implementations are permitted to enforce certain alignment
        /// requirements on `digest` and return `Err(ErrorCode::NOMEM)` if they
        /// are violated. For example, an implementation may require
        /// `digest.len()` must be divisible by 4 and `digest.ptr()` must be
        /// 4-byte aligned.
        ///
        /// # Safety
        ///
        /// The implementation is responsible for checking that `digest` does
        /// not overlap with any of `input`, `function_name`, or
        /// `customization_string`. If it does, it should return
        /// `ErrorCode::NOMEM`.
        pub trait $trait {
            fn digest(
                key: &ReadOnlyProcessBufferRef<'_>,
                input: &ReadOnlyProcessBufferRef<'_>,
                customization_string: &ReadOnlyProcessBufferRef<'_>,
                security_level: SecurityLevel,
                digest: &mut ReadWriteProcessBufferRef<'_>,
            ) -> Result<(), ErrorCode>;
        }
    }
}

// Oneshot hash functions
oneshot_digest_hil! {Sha256, 32}
oneshot_digest_hil! {Sha384, 48}
oneshot_digest_hil! {Sha512, 64}
oneshot_digest_hil! {Sha3_224, 28}
oneshot_digest_hil! {Sha3_256, 32}
oneshot_digest_hil! {Sha3_384, 48}
oneshot_digest_hil! {Sha3_512, 64}
// Oneshot SHAKE XOFs
oneshot_shake_hil! {Shake128}
oneshot_shake_hil! {Shake256}
// Oneshot cSHAKE XOFs
oneshot_cshake_hil! {Cshake128}
oneshot_cshake_hil! {Cshake256}
// Oneshot HMACs
oneshot_hmac_hil! {HmacSha256, 32}
oneshot_hmac_hil! {HmacSha384, 48}
oneshot_hmac_hil! {HmacSha512, 64}
// Oneshot KMACs
oneshot_kmac_hil! {Kmac128}
oneshot_kmac_hil! {Kmac256}
