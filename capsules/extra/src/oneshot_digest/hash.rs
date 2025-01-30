// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

//! Capsules for oneshot hash functions (SHA-2 and SHA-3).
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
//! None
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
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.
//!   + ErrorCode::SIZE: If the digest buffer was an incorrect size.

use super::utils::{copy_digest, require_ro_buffer, require_rw_buffer};
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::oneshot_digest::{Sha256, Sha384, Sha3_224, Sha3_256, Sha3_384, Sha3_512, Sha512};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::{ErrorCode, ProcessId};

macro_rules! oneshot_hash {
    {
        capsule = $capsule:ident,
        hil = $hil:ident,
        digest_length = $digest_length:expr,
    } => {
        #[doc = concat!("Capsule for performing oneshot `", stringify!($hil), "` digests in a synchronous manner.")]
        pub struct $capsule<D> {
            _digest: D,
            grant: Grant<
                (),
            UpcallCount<{ upcall::COUNT }>,
            AllowRoCount<{ ro_allow::COUNT }>,
            AllowRwCount<{ rw_allow::COUNT }>,
            >,
        }

        impl<D> $capsule<D> {
            #[doc = concat!("Instantiates a new `", stringify!($capsule), "` capsule.")]
            pub fn new(
                digest: D,
                grant: Grant<
                    (),
                UpcallCount<{ upcall::COUNT }>,
                AllowRoCount<{ ro_allow::COUNT }>,
                AllowRwCount<{ rw_allow::COUNT }>,
                >,
            ) -> $capsule<D> {
                $capsule {
                    _digest: digest,
                    grant,
                }
            }
        }

        impl<D: $hil> $capsule<D>  {
            /// Command handler for `hash` (command #1).
            fn command_hash(
                &self,
                calling_process: ProcessId,
            ) -> Result<(), ErrorCode> {
                self.grant.enter(calling_process, |_, kernel_data| {
                    require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                        require_rw_buffer(
                            kernel_data,
                            rw_allow::DIGEST,
                            Some($digest_length),
                            |digest| {
                                let mut digest_buf =
                                    [0u32; $digest_length / core::mem::size_of::<u32>()];
                                <D as $hil>::digest(
                                    &input,
                                    &mut digest_buf,
                                )?;
                                copy_digest(&digest_buf, &digest)
                            },
                        )
                    })
                })?
            }
        }

        impl<D: $hil> SyscallDriver for $capsule<D>  {
            fn command(
                &self,
                command_num: usize,
                _data1: usize,
                _data2: usize,
                calling_process: ProcessId,
            ) -> CommandReturn {
                CommandReturn::from(match command_num {
                    command::EXISTS => Ok(()),
                    command::HASH => self.command_hash(calling_process),
                    _ => Err(ErrorCode::NOSUPPORT),
                })
            }

            fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
                self.grant.enter(processid, |_, _| {})
            }
        }
    }
}
oneshot_hash! {
    capsule = OneshotSha256,
    hil = Sha256,
    digest_length = command::hash::SHA256_DIGEST_LENGTH,
}
oneshot_hash! {
    capsule = OneshotSha384,
    hil = Sha384,
    digest_length = command::hash::SHA384_DIGEST_LENGTH,
}
oneshot_hash! {
    capsule = OneshotSha512,
    hil = Sha512,
    digest_length = command::hash::SHA512_DIGEST_LENGTH,
}
oneshot_hash! {
    capsule = OneshotSha3_224,
    hil = Sha3_224,
    digest_length = command::hash::SHA3_224_DIGEST_LENGTH,
}
oneshot_hash! {
    capsule = OneshotSha3_256,
    hil = Sha3_256,
    digest_length = command::hash::SHA3_256_DIGEST_LENGTH,
}
oneshot_hash! {
    capsule = OneshotSha3_384,
    hil = Sha3_384,
    digest_length = command::hash::SHA3_384_DIGEST_LENGTH,
}
oneshot_hash! {
    capsule = OneshotSha3_512,
    hil = Sha3_512,
    digest_length = command::hash::SHA3_512_DIGEST_LENGTH,
}

// Capsule syscall interface parameters

/// Read-only allow IDs
mod ro_allow {
    /// Read-only allow count
    pub const COUNT: u8 = 1;

    /// Read-only allow for the input message.
    pub const INPUT: usize = 0;
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

    /// Parameter IDs for `hash` command.
    pub mod hash {
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
}

mod upcall {
    /// Upcall count
    pub const COUNT: u8 = 0;
}
