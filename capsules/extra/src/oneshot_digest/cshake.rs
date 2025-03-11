// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

//! Capsules for the cSHAKE extensible output function (XOF).
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
//! Compute a hash digest of the input message using the cSHAKE
//! extendable-output function (XOF).
//!
//! Arguments:
//! None
//!
//! Read-only allow parameters:
//! + 0: The buffer containing the input message. Can be any length,
//!   even empty.
//! + 1: Buffer containing the cSHAKE customization string. May be empty.
//! + 2: Buffer containing the name of the cSHAKE function to execute. May be empty.
//!
//! Read-write allow parameters:
//! + 0: The buffer to contain the digest. The length of the buffer allowed
//!   determines the value of the `required_length` parameter passed to the XOF.
//!
//! Returns:
//!   + OK: Operation completed successfully
//!   + ErrorCode::FAIL: Operation failed for an internal reason.
//!   + ErrorCode::NOMEM: If the read-write allow buffer overlaps with a
//!     read-only allow buffer or an alignment requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::SIZE: If a size divisibility requirement on the digest buffer
//!     was not met.
//!   + ErrorCode::RESERVE: If one of the allow parameters was not provided.

use super::utils::{require_ro_buffer, require_rw_buffer};
use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::oneshot_digest::{Cshake128, Cshake256};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::{ErrorCode, ProcessId};

macro_rules! oneshot_cshake {
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
            /// Command handler for `cshake` (command #1).
            fn command_cshake(
                &self,
                calling_process: ProcessId,
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
                                            |mut digest| <D as $hil>::digest(
                                                &input,
                                                &customization,
                                                &function_name,
                                                &mut digest,
                                            )
                                        )
                                    },
                                )
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
                    command::CSHAKE => self.command_cshake(calling_process),
                    _ => Err(ErrorCode::NOSUPPORT),
                })
            }

            fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
                self.grant.enter(processid, |_, _| {})
            }
        }
    }
}
oneshot_cshake! {
    capsule = OneshotCshake128,
    hil = Cshake128,
}
oneshot_cshake! {
    capsule = OneshotCshake256,
    hil = Cshake256,
}

// Capsule syscall interface parameters

/// Read-only allow IDs
mod ro_allow {
    /// Read-only allow count
    pub const COUNT: u8 = 3;

    /// Read-only allow for the input message.
    pub const INPUT: usize = 0;
    /// Read-only allow for the customization string for cSHAKE and KMAC.
    pub const CUSTOMIZATION: usize = 1;
    /// Read-only allow for the function name for cSHAKE.
    pub const FUNCTION_NAME: usize = 2;
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
    pub const CSHAKE: usize = 1;
}

mod upcall {
    /// Upcall count
    pub const COUNT: u8 = 0;
}
