// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

//! Capsules for generating Keccak message authentication codes (KMAC).
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
//! ### Command number 1
//!
//! Compute a message authentication code (MAC) of the input message with the
//! provided key using the Keccak message authentication code (KMAC)
//! algorithm.
//!
//! Arguments:
//! 1. The key security level. Usage is implementation-dependent.
//!   + 0: Low
//!   + 1: Medium
//!   + 2: High
//! 2. Ignored
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
//!   + ErrorCode::INVAL: If the security level parameter is outside the
//!     range 0..=2.
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::NOMEM: If the read-write allow buffer overlaps with a
//!     read-only allow buffer or an alignment requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::SIZE: If a size divisibility requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.

use super::utils::{require_ro_buffer, require_rw_buffer};
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::key_transport::SecurityLevel;
use kernel::hil::oneshot_digest::{Kmac128, Kmac256};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::{ErrorCode, ProcessId};

macro_rules! oneshot_kmac {
    {
        capsule = $capsule:ident,
        hil = $hil:ident,
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
            /// Command handler for `kmac` (command #1).
            fn command_kmac(
                &self,
                calling_process: ProcessId,
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
                                        <D as $hil>::digest(
                                            &key,
                                            &input,
                                            &customization,
                                            security_level,
                                            &mut digest,
                                        )
                                    })
                                },
                            )
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
                CommandReturn::from(match command_num {
                    command::EXISTS => Ok(()),
                    command::KMAC => self.command_kmac(calling_process, data1),
                    _ => Err(ErrorCode::NOSUPPORT),
                })
            }

            fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
                self.grant.enter(processid, |_, _| {})
            }
        }
    }
}
oneshot_kmac! {
    capsule = OneshotKmac128,
    hil = Kmac128,
}
oneshot_kmac! {
    capsule = OneshotKmac256,
    hil = Kmac256,
}

// Capsule syscall interface parameters

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
    pub const KMAC: usize = 1;

    /// Parameter IDs for `hmac` command.
    pub mod kmac {
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
