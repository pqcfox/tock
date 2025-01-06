// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

//! Capsule for oneshot digest operations (hashes and message authentication
//! codes).
//!
//! Command interface
//! -----------------
//!
//! ### Command number 0
//!
//! Check the existence of the driver on the platform.
//!
//! Arguments: none
//!
//! Returns: always CommandReturn::success()
//!
//!
//! ### Command number 1
//!
//! Compute a hash digest of the input message.
//!
//! Arguments:
//! 1. The digest algorithm to use
//!   + 0: SHA-256 (32 byte digest)
//!   + 1: SHA-384 (48-byte digest)
//!   + 2: SHA-512 (64-byte digest)
//!   + 3: SHA3-224 (28-byte digest)
//!   + 4: SHA3-256 (32-byte digest)
//!   + 5: SHA3-384 (48-byte digest)
//!   + 6: SHA3-512 (64-byte digest)
//! 2. Ignored.
//!
//! Read-only allow parameters:
//! + 0: The buffer containing the input message. Can be any length, even empty.
//!
//! Read-write allow parameters:
//! + 0: The buffer to contain the digest. Length must be exactly the size
//! specified above for the algorithm chosen, otherwise ErrorCode::SIZE is
//! returned.
//!
//! Returns:
//!   + OK: Operation completed successfully
//!   + ErrorCode::INVAL: If the first argument parameter is outside the range
//!     0..=6.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.
//!   + ErrorCode::SIZE: If the digest buffer was an incorrect size.
//!
//!
//! ### Command number 2
//!
//! Compute a hash digest of the input message using the SHAKE extendable-output
//! function (XOF).
//!
//! Arguments:
//! 1. The digest algorithm to use
//!   + 0: SHAKE128
//!   + 1: SHAKE256
//! 2. Ignored.
//!
//! Read-only allow parameters:
//! + 0: The buffer containing the input message. Can be any length,
//!   even empty.
//!
//! Read-write allow parameters:
//! + 0: The buffer to contain the digest. The length of the buffer allowed
//!   determines the value of the `required_length` parameter passed to the XOF.
//!
//! Returns:
//!   + OK: Operation completed successfully
//!   + ErrorCode::INVAL: If the first argument parameter is outside the range
//!     0..=1.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.\
//!   + ErrorCode::NOMEM: If the read-write allow buffer overlaps with a
//!     read-only allow buffer or an alignment requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::SIZE: If a size divisibility requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!
//!
//! ### Command number 3
//!
//! Compute a hash digest of the input message using the cSHAKE
//! extendable-output function (XOF).
//!
//! Arguments:
//! 1. The digest algorithm to use
//!   + 0: cSHAKE128
//!   + 1: cSHAKE256
//! 2. Ignored.
//!
//! Read-only allow parameters:
//! + 0: The buffer containing the input message. Can be any length,
//!   even empty.
//! + 2: Buffer containing the cSHAKE customization string. May be empty.
//! + 3: Buffer containing the name of the cSHAKE function to execute. May be empty.
//!
//! Read-write allow parameters:
//! + 0: The buffer to contain the digest. The length of the buffer allowed
//!   determines the value of the `required_length` parameter passed to the XOF.
//!
//! Returns:
//!   + OK: Operation completed successfully
//!   + ErrorCode::INVAL: If the first argument parameter is outside the range
//!     0..=1.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::NOMEM: If the read-write allow buffer overlaps with a
//!     read-only allow buffer or an alignment requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::SIZE: If a size divisibility requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.
//!
//!
//! ### Command number 4
//!
//! Compute a message authentication code (MAC) of the input message with the
//! provided key using the hash-based message authentication code (HMAC)
//! algorithm.
//!
//! Arguments:
//! 1. The digest algorithm to use
//!   + 0: HMAC SHA-256 (32-byte tag)
//!   + 1: HMAC SHA-384 (48-byte tag)
//!   + 2: HMAC SHA-512 (64-byte tag)
//! 2. The key security level. Usage is implementation-dependent.
//!   + 0: Low
//!   + 1: Medium
//!   + 2: High
//!
//! Read-only allow parameters:
//! + 0: The buffer containing the input message. Can be any length,
//!   even empty.
//! + 1: The buffer containing the (unblinded) key material.
//!
//! Read-write allow parameters:
//! + 0: The buffer to contain the MAC tag. Length must be exactly the size
//! specified above for the algorithm chosen, otherwise ErrorCode::SIZE is
//! returned.
//!
//! Returns:
//!   + OK: Operation completed successfully
//!   + ErrorCode::INVAL: If either argument parameter is outside the range 0..=2.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.
//!   + ErrorCode::SIZE: If the digest buffer was an incorrect size.
//!
//!
//! ### Command number 5
//!
//! Compute a message authentication code (MAC) of the input message with the
//! provided key using the Keccak message authentication code (KMAC)
//! algorithm.
//!
//! Arguments:
//! 1. The digest algorithm to use
//!   + 0: KMAC-128
//!   + 1: KMAC-256
//! 2. The key security level. Usage is implementation-dependent.
//!   + 0: Low
//!   + 1: Medium
//!   + 2: High
//!
//! Read-only allow parameters:
//! + 0: The buffer containing the input message. Can be any length,
//!   even empty.
//! + 1: The buffer containing the (unblinded) key material.
//! + 2: Buffer containing the KMAC customization string. May be empty.
//!
//! Read-write allow parameters:
//! + 0: The buffer to contain the MAC tag. Length must be exactly the size
//! specified above for the algorithm chosen, otherwise ErrorCode::SIZE is
//! returned.
//!
//! Returns:
//!   + OK: Operation completed successfully
//!   + ErrorCode::INVAL: If the first argument parameter is outside the range
//!   0..=1, or the second argument parameter is outside the range 0..=2.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::NOMEM: If the read-write allow buffer overlaps with a
//!     read-only allow buffer or an alignment requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::SIZE: If a size divisibility requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.
//!

use core::mem::size_of;
use kernel::grant::GrantKernelData;
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::key_transport::SecurityLevel;
use kernel::hil::oneshot_digest::{
    Cshake128 as Cshake128Hil, Cshake256 as Cshake256Hil, HmacSha256 as HmacSha256Hil,
    HmacSha384 as HmacSha384Hil, HmacSha512 as HmacSha512Hil, Kmac128 as Kmac128Hil,
    Kmac256 as Kmac256Hil, Sha256 as Sha256Hil, Sha384 as Sha384Hil, Sha3_224 as Sha3_224Hil,
    Sha3_256 as Sha3_256Hil, Sha3_384 as Sha3_384Hil, Sha3_512 as Sha3_512Hil, Sha512 as Sha512Hil,
    Shake128 as Shake128Hil, Shake256 as Shake256Hil,
};
use kernel::processbuffer::{
    ReadOnlyProcessBufferRef, ReadWriteProcessBufferRef, ReadableProcessBuffer,
    WriteableProcessBuffer,
};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::{ErrorCode, ProcessId};

/// Capsule for performing oneshot hash and MAC digests in a synchronous manner.
pub struct OneshotDigest<D> {
    _digest: D,
    grant: Grant<
        (),
        UpcallCount<{ upcall::COUNT }>,
        AllowRoCount<{ ro_allow::COUNT }>,
        AllowRwCount<{ rw_allow::COUNT }>,
    >,
}

impl<D> OneshotDigest<D> {
    /// Instantiates a new `OneshotDigest` capsule.
    pub fn new(
        digest: D,
        grant: Grant<
            (),
            UpcallCount<{ upcall::COUNT }>,
            AllowRoCount<{ ro_allow::COUNT }>,
            AllowRwCount<{ rw_allow::COUNT }>,
        >,
    ) -> OneshotDigest<D> {
        OneshotDigest {
            _digest: digest,
            grant,
        }
    }
}

/// Helper that executes a closure using a read-only allow if it exists,
/// with an optional length check.
fn require_ro_buffer<T>(
    grant: &GrantKernelData<'_>,
    allow_id: usize,
    required_length: Option<usize>,
    f: impl Fn(ReadOnlyProcessBufferRef<'_>) -> Result<T, ErrorCode>,
) -> Result<T, ErrorCode> {
    let buffer = grant
        .get_readonly_processbuffer(allow_id)
        .map_err(|_| ErrorCode::RESERVE)?;
    if let Some(req) = required_length {
        if req != buffer.len() {
            return Err(ErrorCode::SIZE);
        }
    }
    f(buffer)
}

/// Helper that executes a closure using a read-only allow if it exists,
/// with an optional length check.
fn require_rw_buffer<T>(
    grant: &GrantKernelData<'_>,
    allow_id: usize,
    required_length: Option<usize>,
    f: impl Fn(ReadWriteProcessBufferRef<'_>) -> Result<T, ErrorCode>,
) -> Result<T, ErrorCode> {
    let buffer = grant
        .get_readwrite_processbuffer(allow_id)
        .map_err(|_| ErrorCode::RESERVE)?;
    if let Some(req) = required_length {
        if req != buffer.len() {
            return Err(ErrorCode::SIZE);
        }
    }
    f(buffer)
}

impl<D> OneshotDigest<D>
where
    D: Sha256Hil + Sha384Hil + Sha512Hil + Sha3_224Hil + Sha3_256Hil + Sha3_384Hil + Sha3_512Hil,
{
    /// Command handler for `hash` (command #1).
    fn command_hash(
        &self,
        calling_process: ProcessId,
        algorithm_id: usize,
    ) -> Result<(), ErrorCode> {
        let digest_length = match algorithm_id {
            command::hash::algorithm::SHA256 => command::hash::SHA256_DIGEST_LENGTH,
            command::hash::algorithm::SHA384 => command::hash::SHA384_DIGEST_LENGTH,
            command::hash::algorithm::SHA512 => command::hash::SHA512_DIGEST_LENGTH,
            command::hash::algorithm::SHA3_224 => command::hash::SHA3_224_DIGEST_LENGTH,
            command::hash::algorithm::SHA3_256 => command::hash::SHA3_256_DIGEST_LENGTH,
            command::hash::algorithm::SHA3_384 => command::hash::SHA3_384_DIGEST_LENGTH,
            command::hash::algorithm::SHA3_512 => command::hash::SHA3_512_DIGEST_LENGTH,
            _ => return Err(ErrorCode::INVAL),
        };
        self.grant.enter(calling_process, |_, kernel_data| {
            require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                require_rw_buffer(
                    kernel_data,
                    rw_allow::DIGEST,
                    Some(digest_length),
                    |digest| {
                        let mut digest_buf =
                            [0u32; command::hash::MAX_DIGEST_LENGTH / size_of::<u32>()];
                        let digest_buf_slice = &mut digest_buf[..digest_length / size_of::<u32>()];
                        // PANIC: `digest_buf_slice` was set to be the correct
                        // length for the requested algorithm.
                        match algorithm_id {
                            command::hash::algorithm::SHA256 => <D as Sha256Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            command::hash::algorithm::SHA384 => <D as Sha384Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            command::hash::algorithm::SHA512 => <D as Sha512Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            command::hash::algorithm::SHA3_224 => <D as Sha3_224Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            command::hash::algorithm::SHA3_256 => <D as Sha3_256Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            command::hash::algorithm::SHA3_384 => <D as Sha3_384Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            command::hash::algorithm::SHA3_512 => <D as Sha3_512Hil>::digest(
                                &input,
                                digest_buf_slice.try_into().unwrap(),
                            ),
                            _ => return Err(ErrorCode::INVAL), // Unreachable
                        }?;
                        copy_digest(digest_buf_slice, &digest)
                    },
                )
            })
        })?
    }
}

impl<D> OneshotDigest<D>
where
    D: Shake128Hil + Shake256Hil,
{
    /// Command handler for `shake` (command #2).
    fn command_shake(
        &self,
        calling_process: ProcessId,
        algorithm_id: usize,
    ) -> Result<(), ErrorCode> {
        self.grant.enter(calling_process, |_, kernel_data| {
            require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                require_rw_buffer(kernel_data, rw_allow::DIGEST, None, |mut digest| {
                    match algorithm_id {
                        command::shake::algorithm::SHAKE128 => {
                            <D as Shake128Hil>::digest(&input, &mut digest)
                        }
                        command::shake::algorithm::SHAKE256 => {
                            <D as Shake256Hil>::digest(&input, &mut digest)
                        }
                        _ => Err(ErrorCode::INVAL),
                    }
                })
            })
        })?
    }
}

impl<D> OneshotDigest<D>
where
    D: Cshake128Hil + Cshake256Hil,
{
    /// Command handler for `cshake` (command #3).
    fn command_cshake(
        &self,
        calling_process: ProcessId,
        algorithm_id: usize,
    ) -> Result<(), ErrorCode> {
        self.grant.enter(calling_process, |_, kernel_data| {
            require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                require_ro_buffer(
                    kernel_data,
                    ro_allow::CUSTOMIZATION,
                    None,
                    |customization| {
                        require_ro_buffer(
                            kernel_data,
                            ro_allow::FUNCTION_NAME,
                            None,
                            |function_name| {
                                require_rw_buffer(
                                    kernel_data,
                                    rw_allow::DIGEST,
                                    None,
                                    |mut digest| match algorithm_id {
                                        command::cshake::algorithm::CSHAKE128 => {
                                            <D as Cshake128Hil>::digest(
                                                &input,
                                                &customization,
                                                &function_name,
                                                &mut digest,
                                            )
                                        }
                                        command::cshake::algorithm::CSHAKE256 => {
                                            <D as Cshake256Hil>::digest(
                                                &input,
                                                &customization,
                                                &function_name,
                                                &mut digest,
                                            )
                                        }
                                        _ => Err(ErrorCode::INVAL),
                                    },
                                )
                            },
                        )
                    },
                )
            })
        })?
    }
}

impl<D> OneshotDigest<D>
where
    D: HmacSha256Hil + HmacSha384Hil + HmacSha512Hil,
{
    /// Command handler for `hmac` (command #4).
    fn command_hmac(
        &self,
        calling_process: ProcessId,
        algorithm_id: usize,
        security_level: usize,
    ) -> Result<(), ErrorCode> {
        let tag_length = match algorithm_id {
            command::hmac::algorithm::HMAC_SHA256 => command::hmac::HMAC_SHA256_TAG_LENGTH,
            command::hmac::algorithm::HMAC_SHA384 => command::hmac::HMAC_SHA384_TAG_LENGTH,
            command::hmac::algorithm::HMAC_SHA512 => command::hmac::HMAC_SHA512_TAG_LENGTH,
            _ => return Err(ErrorCode::INVAL),
        };
        let security_level = match security_level {
            command::hmac::security_level::LOW => SecurityLevel::Low,
            command::hmac::security_level::MEDIUM => SecurityLevel::Medium,
            command::hmac::security_level::HIGH => SecurityLevel::High,
            _ => return Err(ErrorCode::INVAL),
        };
        self.grant.enter(calling_process, |_, kernel_data| {
            require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                require_ro_buffer(kernel_data, ro_allow::KEY, None, |key| {
                    require_rw_buffer(kernel_data, rw_allow::DIGEST, Some(tag_length), |digest| {
                        let mut tag_buf = [0u32; command::hmac::MAX_TAG_LENGTH / size_of::<u32>()];
                        let tag_buf_slice = &mut tag_buf[..tag_length / size_of::<u32>()];
                        // PANIC: `tag_buf_slice` was set to be the correct length for the
                        // requested algorithm.
                        match algorithm_id {
                            command::hmac::algorithm::HMAC_SHA256 => <D as HmacSha256Hil>::digest(
                                &key,
                                &input,
                                security_level,
                                tag_buf_slice.try_into().unwrap(),
                            ),
                            command::hmac::algorithm::HMAC_SHA384 => <D as HmacSha384Hil>::digest(
                                &key,
                                &input,
                                security_level,
                                tag_buf_slice.try_into().unwrap(),
                            ),
                            command::hmac::algorithm::HMAC_SHA512 => <D as HmacSha512Hil>::digest(
                                &key,
                                &input,
                                security_level,
                                tag_buf_slice.try_into().unwrap(),
                            ),
                            _ => return Err(ErrorCode::INVAL), // Unreachable
                        }?;
                        copy_digest(tag_buf_slice, &digest)
                    })
                })
            })
        })?
    }
}

impl<D> OneshotDigest<D>
where
    D: Kmac128Hil + Kmac256Hil,
{
    /// Command handler for `kmac` (command #5).
    fn command_kmac(
        &self,
        calling_process: ProcessId,
        algorithm_id: usize,
        security_level: usize,
    ) -> Result<(), ErrorCode> {
        let security_level = match security_level {
            command::kmac::security_level::LOW => SecurityLevel::Low,
            command::kmac::security_level::MEDIUM => SecurityLevel::Medium,
            command::kmac::security_level::HIGH => SecurityLevel::High,
            _ => return Err(ErrorCode::INVAL),
        };
        self.grant.enter(calling_process, |_, kernel_data| {
            require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                require_ro_buffer(kernel_data, ro_allow::KEY, None, |key| {
                    require_ro_buffer(
                        kernel_data,
                        ro_allow::CUSTOMIZATION,
                        None,
                        |customization| {
                            require_rw_buffer(kernel_data, rw_allow::DIGEST, None, |mut digest| {
                                match algorithm_id {
                                    command::kmac::algorithm::KMAC128 => <D as Kmac128Hil>::digest(
                                        &key,
                                        &input,
                                        &customization,
                                        security_level,
                                        &mut digest,
                                    ),
                                    command::kmac::algorithm::KMAC256 => <D as Kmac256Hil>::digest(
                                        &key,
                                        &input,
                                        &customization,
                                        security_level,
                                        &mut digest,
                                    ),
                                    _ => Err(ErrorCode::INVAL),
                                }
                            })
                        },
                    )
                })
            })
        })?
    }
}

impl<D> SyscallDriver for OneshotDigest<D>
where
    D: Sha256Hil
        + Sha384Hil
        + Sha512Hil
        + Sha3_224Hil
        + Sha3_256Hil
        + Sha3_384Hil
        + Sha3_512Hil
        + Shake128Hil
        + Shake256Hil
        + Cshake128Hil
        + Cshake256Hil
        + HmacSha256Hil
        + HmacSha384Hil
        + HmacSha512Hil
        + Kmac128Hil
        + Kmac256Hil,
{
    fn command(
        &self,
        command_num: usize,
        data1: usize,
        data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        CommandReturn::from(match command_num {
            command::EXISTS => Ok(()),
            command::HASH => self.command_hash(calling_process, data1),
            command::SHAKE => self.command_shake(calling_process, data1),
            command::CSHAKE => self.command_cshake(calling_process, data1),
            command::HMAC => self.command_hmac(calling_process, data1, data2),
            command::KMAC => self.command_kmac(calling_process, data1, data2),
            _ => Err(ErrorCode::NOSUPPORT),
        })
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(processid, |_, _| {})
    }
}

/// Helper function that copies a stack-allocated buffer to a grant-allocated buffer.
fn copy_digest(src: &[u32], dest: &ReadWriteProcessBufferRef<'_>) -> Result<(), ErrorCode> {
    dest.mut_enter(|dest| {
        let mut chunks = dest.chunks(size_of::<u32>());
        for val in src {
            let chunk = match chunks.next() {
                Some(chunk) => chunk,
                // Should never happen, fixed width is checked prior to digest operation
                None => return Err(ErrorCode::SIZE),
            };
            chunk.copy_from_slice(&val.to_ne_bytes())
        }
        Ok(())
    })
    .unwrap_or(Err(ErrorCode::RESERVE))
}

// Capsule syscall interface parameters

/// Driver number
pub const DRIVER_NUM: usize = capsules_core::driver::NUM::OneshotDigest as usize;

/// Read-only allow IDs
mod ro_allow {
    /// Read-only allow count
    pub const COUNT: u8 = 4;

    /// Read-only allow for the input message.
    pub const INPUT: usize = 0;
    /// Read-only allow for the key material for MACs.
    pub const KEY: usize = 1;
    /// Read-only allow for the customization string for cSHAKE and KMAC.
    pub const CUSTOMIZATION: usize = 2;
    /// Read-only allow for the function name for cSHAKE.
    pub const FUNCTION_NAME: usize = 3;
}

/// Read-write allow IDs
mod rw_allow {
    /// Read-write allow count
    pub const COUNT: u8 = 1;

    /// Read-write allow for the digest/tag. Some algorithms enforce a size
    /// requirement on this buffer. See the command documentation at the top of
    /// this file for details.
    pub const DIGEST: usize = 0;
}

/// Command IDs and parameters
mod command {
    /// Command ID to check this driver exists.
    pub const EXISTS: usize = 0;
    /// Command ID for fixed-width hash functions.
    pub const HASH: usize = 1;
    /// Command ID for the SHAKE extendable-output function.
    pub const SHAKE: usize = 2;
    /// Command ID for the cSHAKE extendable-output function.
    pub const CSHAKE: usize = 3;
    /// Command ID for the hash-based message authentication code (HMAC)
    /// algorithm.
    pub const HMAC: usize = 4;
    /// Command ID for the Keccak message authentication code (KMAC) algorithm.
    pub const KMAC: usize = 5;

    /// Parameter IDs for `hash` command.
    pub mod hash {
        /// Algorithm parameter values (parameter 1).
        pub mod algorithm {
            /// SHA-256 (32-byte digest)
            pub const SHA256: usize = 0;
            /// SHA-384 (48-byte digest)
            pub const SHA384: usize = 1;
            /// SHA-512 (64-byte digest)
            pub const SHA512: usize = 2;
            /// SHA3-224 (28-byte digest)
            pub const SHA3_224: usize = 3;
            /// SHA3-256 (32-byte digest)
            pub const SHA3_256: usize = 4;
            /// SHA3-384 (48-byte digest)
            pub const SHA3_384: usize = 5;
            /// SHA3-512 (64-byte digest)
            pub const SHA3_512: usize = 6;
        }

        /// Maximum digest length across all supported algorithms
        pub const MAX_DIGEST_LENGTH: usize = 64;

        /// SHA-256 digest length
        pub const SHA256_DIGEST_LENGTH: usize = 32;
        /// SHA-384 digest length
        pub const SHA384_DIGEST_LENGTH: usize = 48;
        /// SHA-512 digest length
        pub const SHA512_DIGEST_LENGTH: usize = 64;
        /// SHA3-224 digest length
        pub const SHA3_224_DIGEST_LENGTH: usize = 28;
        /// SHA3-256 digest length
        pub const SHA3_256_DIGEST_LENGTH: usize = 32;
        /// SHA3-384 digest length
        pub const SHA3_384_DIGEST_LENGTH: usize = 48;
        /// SHA3-512 digest length
        pub const SHA3_512_DIGEST_LENGTH: usize = 64;
    }

    /// Parameter IDs for `shake` command.
    pub mod shake {
        /// Algorithm parameter values (parameter 1).
        pub mod algorithm {
            /// SHAKE-128
            pub const SHAKE128: usize = 0;
            /// SHAKE-256
            pub const SHAKE256: usize = 1;
        }
    }

    /// Parameter IDs for `cshake` command.
    pub mod cshake {
        /// Algorithm parameter values (parameter 1).
        pub mod algorithm {
            /// SHAKE-128
            pub const CSHAKE128: usize = 0;
            /// SHAKE-256
            pub const CSHAKE256: usize = 1;
        }
    }

    /// Parameter IDs for `hmac` command.
    pub mod hmac {
        /// Algorithm parameter values (parameter 1).
        pub mod algorithm {
            /// HMAC with SHA-256 (32-byte tag)
            pub const HMAC_SHA256: usize = 0;
            /// HMAC with HMAC_SHA-384 (48-byte tag)
            pub const HMAC_SHA384: usize = 1;
            /// HMAC with HMAC_SHA-512 (64-byte tag)
            pub const HMAC_SHA512: usize = 2;
        }
        /// Security level values (implementation-dependent)
        pub mod security_level {
            /// Low security level
            pub const LOW: usize = 0;
            /// Medium security level
            pub const MEDIUM: usize = 1;
            /// High security level
            pub const HIGH: usize = 2;
        }

        /// Maximum tag length across all widths
        pub const MAX_TAG_LENGTH: usize = 64;

        /// HMAC SHA-256 tag length
        pub const HMAC_SHA256_TAG_LENGTH: usize = 32;
        /// HMAC SHA-384 tag length
        pub const HMAC_SHA384_TAG_LENGTH: usize = 48;
        /// HMAC SHA-512 tag length
        pub const HMAC_SHA512_TAG_LENGTH: usize = 64;
    }

    /// Parameter IDs for `hmac` command.
    pub mod kmac {
        /// Algorithm parameter values (parameter 1).
        pub mod algorithm {
            /// KMAC-128
            pub const KMAC128: usize = 0;
            /// KMAC-256
            pub const KMAC256: usize = 1;
        }
        /// Security level values (implementation-dependent)
        pub mod security_level {
            /// Low security level
            pub const LOW: usize = 0;
            /// Medium security level
            pub const MEDIUM: usize = 1;
            /// High security level
            pub const HIGH: usize = 2;
        }
    }
}

mod upcall {
    /// Upcall count
    pub const COUNT: u8 = 0;
}
