// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! A capsule for performing asymmetric cryptography
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
//! Return value: always CommandReturn::success()
//!
//! ### Command number 1
//!
//! Verify a signature on the the provided digest using the provided public key.
//! The expected public key format depends on the signature algorithm:
//! + ECDSA: [x | y], with no additional padding. Both x and y should be
//! in little-endian encoding.
//!
//! Arguments:
//! + Hash algorithm used to compute the message digest.
//!   + 0: SHA-256
//!   + 1: SHA-384
//!   + 2: SHA-512
//!   + 3: SHA3-256
//!   + 4: SHA3-384
//!   + 5: SHA3-512
//!
//! Read-only allow inputs:
//! + 0: Message digest
//! + 1: Signature
//! + 2: Public key.
//!
//! Return value:
//! + CommandReturn::failure(ErrorCode::SIZE): if the length of any of the allow paramters
//! does not match the indicated algorithm.
//! + CommandReturn::failure(ErrorCode::BUSY): if the underlying cryptographic hardware
//! + reported a busy status.
//! + CommandReturn::failure(ErrorCode::INVAL): Either no public key was set, the current
//! public key is for an incompatible algorithm, or the specified hash mode is not supported
//! by the associated curve.
//! + CommandReturn::failure(ErrorCode::FAIL): if the operation failed for any other reason.
//! + CommandReturn::success(): the operation has been initiated.
//!
//! Subscribe interface
//! -------------------
//!
//! ### Subscribe 0
//!
//! Register a callback indicating a signature verify operation
//! (command #1) has completed.
//!
//! Callback arguments:
//!
//! 1. Operation result:
//!     + ErrorCode::FAIL: Operation failed.
//!     + OK: KeyMangager Operation completed successfully.
//! 2. error code (relevant only if `Operation result` is ErrorCode::FAIL): an error
//! describing the failure
//!     + 1: Signature was not valid.
//!     + 2: An OTBN error occurred.
//! 3. Always 0

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::public_key_crypto::ecc::{HashMode, SetHashMode};
use kernel::hil::public_key_crypto::keys::PubKeyMut;
use kernel::hil::public_key_crypto::signature::{ClientVerify, SignatureVerify};
use kernel::processbuffer::ReadableProcessBuffer;
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::ErrorCode;
use kernel::ProcessId;

/// ECDSA P-256 driver number
pub const DRIVER_NUM_P256: usize = capsules_core::driver::NUM::EcdsaP256 as usize;
/// ECDSA P-384 driver number
pub const DRIVER_NUM_P384: usize = capsules_core::driver::NUM::EcdsaP384 as usize;

pub struct AsymmetricCrypto<
    'a,
    const HASH_LEN: usize,
    const SIG_LEN: usize,
    SigVerify: PubKeyMut + SetHashMode + SignatureVerify<'a, HASH_LEN, SIG_LEN>,
> {
    verifier: &'a SigVerify,
    hash_buf: TakeCell<'static, [u8; HASH_LEN]>,
    signature_buf: TakeCell<'static, [u8; SIG_LEN]>,
    grant: Grant<
        (),
        UpcallCount<{ upcall::COUNT }>,
        AllowRoCount<{ RO_ALLOW_COUNT }>,
        AllowRwCount<{ RW_ALLOW_COUNT }>,
    >,
    owning_process: OptionalCell<ProcessId>,
}

impl<
        'a,
        const HASH_LEN: usize,
        const SIG_LEN: usize,
        SigVerify: PubKeyMut + SetHashMode + SignatureVerify<'a, HASH_LEN, SIG_LEN>,
    > AsymmetricCrypto<'a, HASH_LEN, SIG_LEN, SigVerify>
{
    /// Creates a new `AsymmetricCrypto` capsule
    pub fn new(
        verifier: &'a SigVerify,
        hash_buf: &'static mut [u8; HASH_LEN],
        signature_buf: &'static mut [u8; SIG_LEN],
        grant: Grant<
            (),
            UpcallCount<{ upcall::COUNT }>,
            AllowRoCount<{ RO_ALLOW_COUNT }>,
            AllowRwCount<{ RW_ALLOW_COUNT }>,
        >,
    ) -> AsymmetricCrypto<'a, HASH_LEN, SIG_LEN, SigVerify> {
        AsymmetricCrypto {
            verifier,
            hash_buf: TakeCell::new(hash_buf),
            signature_buf: TakeCell::new(signature_buf),
            owning_process: OptionalCell::empty(),
            grant,
        }
    }

    /// Verify the provided signature on the provided digest. Uses the
    /// grant owned by this capsule as the source of the arguments.
    fn command_verify(
        &self,
        calling_process: ProcessId,
        hash_mode: usize,
    ) -> Result<(), ErrorCode> {
        // Set hash mode, failing-fast if the value is invalid.
        self.verifier.set_hash_mode(match hash_mode {
            command::verify::hash_mode::SHA256 => HashMode::Sha256,
            command::verify::hash_mode::SHA384 => HashMode::Sha384,
            command::verify::hash_mode::SHA512 => HashMode::Sha512,
            command::verify::hash_mode::SHA3_256 => HashMode::Sha3_256,
            command::verify::hash_mode::SHA3_384 => HashMode::Sha3_384,
            command::verify::hash_mode::SHA3_512 => HashMode::Sha3_512,
            _ => return Err(ErrorCode::INVAL),
        })?;
        // Get hash buffer
        let hash_buf = match self.hash_buf.take() {
            Some(hash_buf) => hash_buf,
            // No `hash_buf` := BUSY
            None => return Err(ErrorCode::BUSY),
        };
        // Get signature buffer
        let signature_buf = match self.signature_buf.take() {
            Some(signature_buf) => signature_buf,
            // No `signature_buf` := BUSY
            None => {
                // Reset `hash_buf` so we can try again
                self.hash_buf.put(Some(hash_buf));
                return Err(ErrorCode::BUSY);
            }
        };
        // Get public key buffer
        let pub_key_buf = match self.export_raw_public_key() {
            Ok(buf) => buf,
            Err(err) => {
                // No public key buffer available. Reset the state so we can try again.
                self.hash_buf.put(Some(hash_buf));
                self.signature_buf.put(Some(signature_buf));
                return Err(err);
            }
        };
        // We have all the buffers we need; enter grant
        let mut grant_result = Ok(());
        // Ignore grant entry failures, since there is no resaonable way to handle them.
        let _ = self.grant.enter(calling_process, |_, kernel_data| {
            let mut try_copy_values = || -> Result<(), ErrorCode> {
                // Copy hash from grant to `hash_buf`
                kernel_data
                    // CAST: `RoAllowId` explicitly defines the value conversion.
                    .get_readonly_processbuffer(RoAllowId::Digest as usize)
                    .and_then(|allowed_buffer| {
                        allowed_buffer.enter(|data| {
                            if data.len() != hash_buf.len() {
                                return Err(ErrorCode::SIZE);
                            }
                            // PANIC: the length of `data` has been checked.
                            data.copy_to_slice_or_err(&mut hash_buf[..data.len()])
                        })
                    })
                    .unwrap_or(Err(ErrorCode::FAIL))?;
                // Copy signature from grant to `signature_buf`
                kernel_data
                    // CAST: `RoAllowId` explicitly defines the value conversion.
                    .get_readonly_processbuffer(RoAllowId::Signature as usize)
                    .and_then(|allowed_buffer| {
                        allowed_buffer.enter(|data| {
                            if data.len() != signature_buf.len() {
                                return Err(ErrorCode::SIZE);
                            }
                            // PANIC: the length of `data` has been checked.
                            data.copy_to_slice_or_err(&mut signature_buf[..data.len()])
                        })
                    })
                    .unwrap_or(Err(ErrorCode::FAIL))?;
                // Copy public key from grant to `pub_key_buf`
                kernel_data
                    // CAST: `RoAllowId` explicitly defines the value conversion.
                    .get_readonly_processbuffer(RoAllowId::PublicKey as usize)
                    .and_then(|allowed_buffer| {
                        allowed_buffer.enter(|data| {
                            if data.len() != pub_key_buf.len() {
                                return Err(ErrorCode::SIZE);
                            }
                            // PANIC: the length of `data` has been checked.
                            data.copy_to_slice_or_err(&mut pub_key_buf[..data.len()])
                        })
                    })
                    .unwrap_or(Err(ErrorCode::FAIL))
            };
            // Ignore this clippy lint that complains about using a closure only once; the closure
            // exists to make the `?` operator available, which makes handling the allow errors
            // much neater.
            #[allow(clippy::redundant_closure_call)]
            {
                grant_result = try_copy_values();
            }
        });
        match grant_result {
            Err(err) => {
                // Error with one of the allows; reset the state
                // and propagate the error.
                self.hash_buf.put(Some(hash_buf));
                self.signature_buf.put(Some(signature_buf));
                Err(err)
            }
            Ok(()) => {
                // Everything is good; pass the buffers down to the underlying driver.
                self.owning_process.set(calling_process);
                self.verify(hash_buf, signature_buf, pub_key_buf).map_err(
                    |(err, hash, sig, pub_key)| {
                        // Verify operation failed to start; reset the state
                        self.hash_buf.put(Some(hash));
                        self.signature_buf.put(Some(sig));
                        // Ignore errors here; there's no way we can recover if we are unable to return
                        // the public key buffer to the underlying implementation during
                        // error-handling.
                        let _ = pub_key.map(|pub_key| {
                            let _ = self.verifier.import_public_key(pub_key);
                        });
                        err
                    },
                )
            }
        }
    }

    /// Verify the provided signature on the provided digest.
    pub fn verify(
        &self,
        hash_buf: &'static mut [u8; HASH_LEN],
        signature_buf: &'static mut [u8; SIG_LEN],
        pub_key_buf: &'static mut [u8],
    ) -> Result<
        (),
        (
            ErrorCode,
            &'static mut [u8; HASH_LEN],
            &'static mut [u8; SIG_LEN],
            // This error is set if extracting the public key after an operation
            // failure itself fails.
            Result<&'static mut [u8], ErrorCode>,
        ),
    > {
        // Import the public key
        if let Err((err, pub_key_buf)) = self.verifier.import_public_key(pub_key_buf) {
            return Err((err, hash_buf, signature_buf, Ok(pub_key_buf)));
        }
        // Verify the signature
        self.verifier
            .verify(hash_buf, signature_buf)
            .map_err(|(err, hash_buf, sig_buf)| {
                // If initializing the verify operation fails, export the public
                // key to return the buffer to the caller.
                (err, hash_buf, sig_buf, self.verifier.pub_key())
            })
    }

    /// Extract the public key buffer from the underlying
    /// implementation. If calling this API from another capsule, you
    /// should call this function to obtain the public key buffer,
    /// write the public key material to it, and then pass it back to
    /// this capsule via one of the other APIs (e.g. `verify`).
    pub fn export_raw_public_key(&self) -> Result<&'static mut [u8], ErrorCode> {
        self.verifier.pub_key()
    }

    /// Called when a signature verification operation completes
    fn schedule_verify_done_upcall(&self, process_id: ProcessId, status: Result<bool, ErrorCode>) {
        // Grant errors are ignored, since there is no reasonable way
        // to handle them.
        let args = match status {
            Ok(true) => {
                // Verification succeeded
                (0, 0, 0)
            }
            Ok(false) => {
                // Verification failed
                (ErrorCode::FAIL as usize, 1, 0)
            }
            // CAST: `ErrorCode` defines the value conversion explicitly
            Err(e) => (e as usize, 2, 0),
        };
        let _ = self.grant.enter(process_id, |_, kernel_data| {
            // Scheduling errors are ignored, since there is no reasonable way to handle them.
            let _ = kernel_data.schedule_upcall(upcall::UpcallId::VerifyDone.to_usize(), args);
        });
    }
}

/// Read-only allow count
const RO_ALLOW_COUNT: u8 = 3;
/// Read-write allow count
const RW_ALLOW_COUNT: u8 = 0;

/// Read-only buffer identifier
#[repr(usize)]
enum RoAllowId {
    /// Digest for verification
    Digest = 0,
    /// A signature to verify
    Signature = 1,
    /// A public key used to verify a `Signature`
    PublicKey = 2,
}

mod upcall {
    pub const COUNT: u8 = 1;

    /// Upcall identifiers. Matches the subscribe numbers in the API
    /// definition at the top of this file.
    #[repr(usize)]
    pub enum UpcallId {
        VerifyDone,
    }

    impl UpcallId {
        /// Convert the ID to usize
        pub const fn to_usize(self) -> usize {
            // CAST: the enum defines the conversion explicitly
            self as usize
        }
    }
}

mod command {
    /// Command ID to check this driver exists.
    pub const EXISTS: usize = 0;
    /// Command ID for signature verification.
    pub const VERIFY: usize = 1;

    /// Parameter IDs for `verify` command.
    pub mod verify {
        /// Hash mode paramter values (parameter 1).
        pub mod hash_mode {
            /// SHA-256
            pub const SHA256: usize = 0;
            /// SHA-384
            pub const SHA384: usize = 1;
            /// SHA-512
            pub const SHA512: usize = 2;
            /// SHA3-256
            pub const SHA3_256: usize = 3;
            /// SHA3-384
            pub const SHA3_384: usize = 4;
            /// SHA3-512
            pub const SHA3_512: usize = 5;
        }
    }
}

impl<
        'a,
        const HASH_LEN: usize,
        const SIG_LEN: usize,
        SigVerify: PubKeyMut + SetHashMode + SignatureVerify<'a, HASH_LEN, SIG_LEN>,
    > SyscallDriver for AsymmetricCrypto<'a, HASH_LEN, SIG_LEN, SigVerify>
{
    fn command(
        &self,
        command_num: usize,
        data1: usize,
        _data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        CommandReturn::from(match command_num {
            command::EXISTS => Ok(()),
            command::VERIFY => self.command_verify(calling_process, data1),
            _ => Err(ErrorCode::NOSUPPORT),
        })
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(processid, |_, _| {})
    }
}

// Interrupt handler for completion of signature verification
// operations.
impl<
        'a,
        const HASH_LEN: usize,
        const SIG_LEN: usize,
        SigVerify: PubKeyMut + SetHashMode + SignatureVerify<'a, HASH_LEN, SIG_LEN>,
    > ClientVerify<HASH_LEN, SIG_LEN> for AsymmetricCrypto<'a, HASH_LEN, SIG_LEN, SigVerify>
{
    fn verification_done(
        &self,
        result: Result<bool, ErrorCode>,
        hash: &'static mut [u8; HASH_LEN],
        signature: &'static mut [u8; SIG_LEN],
    ) {
        self.owning_process
            .map(|owner_id| self.schedule_verify_done_upcall(owner_id, result));
        self.hash_buf.put(Some(hash));
        self.signature_buf.put(Some(signature));
    }
}
