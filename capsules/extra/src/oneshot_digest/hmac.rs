// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

//! Capsules for oneshot hash-based message authentication codes (HMACs).
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
//! Compute a message authentication code (MAC) of the input message with the
//! provided key using the hash-based message authentication code (HMAC)
//! algorithm.
//!
//! Arguments:
//! 1. The key security level. Usage is implementation-dependent.
//!   + 0: Low
//!   + 1: Medium
//!   + 2: High
//! 2. Ignored.
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
//!   + ErrorCode::INVAL: If the security level parameter is outside the
//!     range 0..=2.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.
//!   + ErrorCode::SIZE: If the digest buffer was an incorrect size.

use super::utils::{copy_digest, require_ro_buffer, require_rw_buffer};
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::key_transport::SecurityLevel;
use kernel::hil::oneshot_digest::{HmacSha256, HmacSha384, HmacSha512};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::{ErrorCode, ProcessId};

macro_rules! oneshot_hmac {
    {
        capsule = $capsule:ident,
        hil = $hil:ident,
        tag_length = $tag_length:expr,
    } => {
        #[doc = concat!("Capsule for computing oneshot `", stringify!($hil), "` tag in a synchronous manner.")]
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
            /// Command handler for `hmac` (command #4).
            fn command_hmac(
                &self,
                calling_process: ProcessId,
                security_level: usize,
            ) -> Result<(), ErrorCode> {
                let security_level = match security_level {
                    command::hmac::security_level::LOW => SecurityLevel::Low,
                    command::hmac::security_level::MEDIUM => SecurityLevel::Medium,
                    command::hmac::security_level::HIGH => SecurityLevel::High,
                    _ => return Err(ErrorCode::INVAL),
                };
                self.grant.enter(calling_process, |_, kernel_data| {
                    require_ro_buffer(kernel_data, ro_allow::INPUT, None, |input| {
                        require_ro_buffer(kernel_data, ro_allow::KEY, None, |key| {
                            require_rw_buffer(kernel_data, rw_allow::DIGEST, Some($tag_length), |digest| {
                                let mut tag_buf = [0u32; $tag_length / core::mem::size_of::<u32>()];
                                <D as $hil>::digest(
                                    &key,
                                    &input,
                                    security_level,
                                    &mut tag_buf,
                                )?;
                                copy_digest(&tag_buf, &digest)
                            })
                        })
                    })
                })?
            }
        }

        impl<D: $hil> SyscallDriver for $capsule<D>  {
            fn command(
                &self,
                command_num: usize,
                data1: usize,
                _data2: usize,
                calling_process: ProcessId,
            ) -> CommandReturn {
                kernel::debug!("in hmac: command = {}", command_num);
                CommandReturn::from(match command_num {
                    command::EXISTS => Ok(()),
                    command::HMAC => self.command_hmac(calling_process, data1),
                    _ => Err(ErrorCode::NOSUPPORT),
                })
            }

            fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
                self.grant.enter(processid, |_, _| {})
            }
        }
    }
}
oneshot_hmac! {
    capsule = OneshotHmacSha256,
    hil = HmacSha256,
    tag_length = command::hmac::HMAC_SHA256_TAG_LENGTH,
}
oneshot_hmac! {
    capsule = OneshotHmacSha384,
    hil = HmacSha384,
    tag_length = command::hmac::HMAC_SHA384_TAG_LENGTH,
}
oneshot_hmac! {
    capsule = OneshotHmacSha512,
    hil = HmacSha512,
    tag_length = command::hmac::HMAC_SHA512_TAG_LENGTH,
}

// Capsule syscall interface parameters

/// Read-only allow IDs
mod ro_allow {
    /// Read-only allow count
    pub const COUNT: u8 = 2;

    /// Read-only allow for the input message.
    pub const INPUT: usize = 0;
    /// Read-only allow for the key material for MACs.
    pub const KEY: usize = 1;
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
    pub const HMAC: usize = 1;

    /// Parameter IDs for `hmac` command.
    pub mod hmac {
        /// Security level values (implementation-dependent)
        pub mod security_level {
            /// Low security level
            pub const LOW: usize = 0;
            /// Medium security level
            pub const MEDIUM: usize = 1;
            /// High security level
            pub const HIGH: usize = 2;
        }

        /// HMAC SHA-256 tag length
        pub const HMAC_SHA256_TAG_LENGTH: usize = 32;
        /// HMAC SHA-384 tag length
        pub const HMAC_SHA384_TAG_LENGTH: usize = 48;
        /// HMAC SHA-512 tag length
        pub const HMAC_SHA512_TAG_LENGTH: usize = 64;
    }
}

mod upcall {
    /// Upcall count
    pub const COUNT: u8 = 0;
}
