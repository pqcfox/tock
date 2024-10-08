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
//! + The signature algorithm to use
//!   + 0: ECDSA
//! + Signature algorithm parameter, if applicable
//!   + If algorithm = ECDSA, the curve type:
//!     + 0: P-256
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
//! + CommandReturn::failure(ErrorCode::INVAL): Either no public key was set, or the current
//! public key is for an incompatible algorithm.
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
use kernel::hil::public_key_crypto::ecc::{
    CurveParams, EcdsaP256 as EcdsaP256Trait, EcdsaP256Client, EllipticCurve, P256,
};
use kernel::hil::public_key_crypto::keys::PubKeyMut;
use kernel::hil::public_key_crypto::signature::SignatureVerify;
use kernel::processbuffer::ReadableProcessBuffer;
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::ErrorCode;
use kernel::ProcessId;

/// The maximum digest length for any supported curve
const HASH_LEN: usize = 32;
/// The maximum signature length for any supported curve
const SIG_LEN: usize = 64;

pub struct AsymmetricCrypto<'a, PubKey, EcdsaP256> {
    public_key: &'a PubKey,
    p256_ecdsa: &'a EcdsaP256,
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

// Algorithm identifiers

/// Identifier for ECDSA as an asymmetric algorithm.
const ALG_ECDSA: usize = 0;

// Algorithm parameter identifiers

const ECDSA_PARAM_P256: usize = 0;

pub enum Algorithm {
    Ecdsa(CurveParams),
}

impl Algorithm {
    /// Constructs an `Algorithm` identifier from its components
    fn from_raw_parts(algorithm_id: usize, param_id: usize) -> Option<Algorithm> {
        match algorithm_id {
            ALG_ECDSA => match param_id {
                ECDSA_PARAM_P256 => Some(Algorithm::Ecdsa(P256::curve_params())),
                _ => None,
            },
            _ => None,
        }
    }

    /// Checks that the paramters to a `verify` operation are the
    /// correct lengths for this `Algorithm`.
    fn check_verify_lengths(
        &self,
        hash_len: usize,
        signature_len: usize,
        pub_key_len: usize,
    ) -> Result<(), ErrorCode> {
        if match self {
            // OVERFLOW: We need to check the public key is twice the
            // length of a curve coordinate. To prevent a rogue
            // implementation of `EllipticCurve` causing an overflow
            // when multiplying `COORD_LEN` by 2, we instead check the
            // lowest-order bit separately and check the rest by
            // bit-shift.
            Algorithm::Ecdsa(curve) => {
                curve.hash_len == hash_len
                    && curve.sig_len == signature_len
                    && pub_key_len & 1 == 0
                    && curve.coord_len == pub_key_len >> 1
            }
        } {
            Ok(())
        } else {
            Err(ErrorCode::SIZE)
        }
    }
}

impl<'a, PubKey, EcdsaP256> AsymmetricCrypto<'a, PubKey, EcdsaP256> {
    /// Creates a new `AsymmetricCrypto` capsule
    pub fn new(
        public_key: &'a PubKey,
        p256_ecdsa: &'a EcdsaP256,
        hash_buf: &'static mut [u8; HASH_LEN],
        signature_buf: &'static mut [u8; SIG_LEN],
        grant: Grant<
            (),
            UpcallCount<{ upcall::COUNT }>,
            AllowRoCount<{ RO_ALLOW_COUNT }>,
            AllowRwCount<{ RW_ALLOW_COUNT }>,
        >,
    ) -> AsymmetricCrypto<'a, PubKey, EcdsaP256> {
        AsymmetricCrypto {
            public_key,
            p256_ecdsa,
            hash_buf: TakeCell::new(hash_buf),
            signature_buf: TakeCell::new(signature_buf),
            owning_process: OptionalCell::empty(),
            grant,
        }
    }
}

impl<'a, PubKey, P256Verifier> AsymmetricCrypto<'a, PubKey, P256Verifier>
where
    PubKey: PubKeyMut,
    P256Verifier: SignatureVerify<'a, { P256::HASH_LEN }, { P256::SIG_LEN }>,
{
    /// Verify the provided signature on the provided digest. Uses the
    /// grant owned by this capsule as the source of the arguments.
    fn command_verify(
        &self,
        algorithm_id: usize,
        param_id: usize,
        calling_process: ProcessId,
    ) -> Result<(), ErrorCode> {
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
                            if data.len() > hash_buf.len() {
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
                            if data.len() > signature_buf.len() {
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
                            if data.len() > pub_key_buf.len() {
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
                self.verify(
                    Algorithm::from_raw_parts(algorithm_id, param_id).ok_or(ErrorCode::INVAL)?,
                    hash_buf,
                    signature_buf,
                    pub_key_buf,
                )
                .map_err(|(err, hash, sig, pub_key)| {
                    // Verify operation failed to start; reset the state
                    self.hash_buf.put(Some(hash));
                    self.signature_buf.put(Some(sig));
                    // Ignore errors here; there's no way we can recover if we are unable to return
                    // the public key buffer to the underlying implementation during
                    // error-handling.
                    let _ = pub_key.map(|pub_key| {
                        let _ = self.public_key.import_public_key(pub_key);
                    });
                    err
                })
            }
        }
    }

    /// Verify the provided signature on the provided digest.
    pub fn verify(
        &self,
        algorithm: Algorithm,
        hash_buf: &'static mut [u8; HASH_LEN],
        signature_buf: &'static mut [u8; SIG_LEN],
        pub_key_buf: &'static mut [u8],
    ) -> Result<
        (),
        (
            ErrorCode,
            &'static mut [u8; HASH_LEN],
            &'static mut [u8; SIG_LEN],
            // This error is set if extracting the public key after an operation failure itself
            // fails.
            Result<&'static mut [u8], ErrorCode>,
        ),
    > {
        match algorithm.check_verify_lengths(hash_buf.len(), signature_buf.len(), pub_key_buf.len())
        {
            Err(err) => Err((err, hash_buf, signature_buf, Ok(pub_key_buf))),
            Ok(()) => {
                // Import the public key
                if let Err((err, pub_key_buf)) = self.public_key.import_public_key(pub_key_buf) {
                    return Err((err, hash_buf, signature_buf, Ok(pub_key_buf)));
                }
                // Verify the signature
                match algorithm {
                    Algorithm::Ecdsa(curve) => match curve.oid {
                        P256::OID => self.p256_ecdsa.verify(hash_buf, signature_buf).map_err(
                            |(err, hash_buf, sig_buf)| {
                                // If initializing the verify
                                // operation fails, export the public
                                // key to return the buffer to the
                                // caller.
                                (err, hash_buf, sig_buf, self.public_key.pub_key())
                            },
                        ),
                        _ => Err((
                            ErrorCode::INVAL,
                            hash_buf,
                            signature_buf,
                            self.public_key.pub_key(),
                        )),
                    },
                }
            }
        }
    }
}

impl<'a, PubKey, P256Verifier> AsymmetricCrypto<'a, PubKey, P256Verifier>
where
    PubKey: PubKeyMut,
{
    /// Extract the public key buffer from the underlying
    /// implementation. If calling this API from another capsule, you
    /// should call this function to obtain the public key buffer,
    /// write the public key material to it, and then pass it back to
    /// this capsule via one of the other APIs (e.g. `verify`).
    pub fn export_raw_public_key(&self) -> Result<&'static mut [u8], ErrorCode> {
        self.public_key.pub_key()
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

/// Attestation commands, as described in the API definition at the
/// top of this file.
enum Command {
    DriverExistence,
    Verify,
}

impl TryFrom<usize> for Command {
    type Error = ();

    fn try_from(id: usize) -> Result<Command, Self::Error> {
        match id {
            0 => Ok(Command::DriverExistence),
            1 => Ok(Command::Verify),
            _ => Err(()),
        }
    }
}

impl<'a, PubKey, EcdsaP256> SyscallDriver for AsymmetricCrypto<'a, PubKey, EcdsaP256>
where
    PubKey: PubKeyMut,
    EcdsaP256: EcdsaP256Trait<'a>,
{
    fn command(
        &self,
        command_num: usize,
        data1: usize,
        data2: usize,
        calling_process: ProcessId,
    ) -> CommandReturn {
        let cmd = Command::try_from(command_num);
        CommandReturn::from(match cmd {
            Ok(Command::DriverExistence) => Ok(()),
            Ok(Command::Verify) => self.command_verify(data1, data2, calling_process),
            Err(()) => Err(ErrorCode::NOSUPPORT),
        })
    }

    fn allocate_grant(&self, processid: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(processid, |_, _| {})
    }
}

// Interrupt handler for completion of signature verification
// operations.
impl<'a, PubKey, EcdsaP256> EcdsaP256Client for AsymmetricCrypto<'a, PubKey, EcdsaP256> {
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

impl<'a, PubKey, EcdsaP256> AsymmetricCrypto<'a, PubKey, EcdsaP256> {
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
