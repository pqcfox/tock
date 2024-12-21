// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use crate::ffi::cryptolib::ecc::ecdsa::{EcdsaVerifyP256Job, EcdsaVerifyP384Job};
use crate::otbn::{OtbnRegisters, STATUS};
use capsules_core::virtualizers::timeout_mux::{Job, TimeoutMux};
use kernel::hil::time::Alarm;
use kernel::hil::time::Ticks64;
use kernel::utilities::registers::interfaces::Readable;
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

/// Check frequency for `capsules_core::virtualizers::TimeoutMux` for OTBN
/// scheduling. 1 tick_freq = 10000 cycles at 100 MHz = 100 μs.
pub const OTBN_TIMEOUT_MUX_CHECK_FREQ: kernel::hil::time::Ticks64 = Ticks64::new(4);

/// Multiplexer over cryptolib asynchronous cryptolib operations.
pub struct CryptolibMux<'a, A: Alarm<'a>> {
    otbn_registers: StaticRef<OtbnRegisters>,
    otbn_mux: &'a TimeoutMux<'a, A, OtbnOperation<'a, A>>,
}

impl<'a, A: Alarm<'a>> CryptolibMux<'a, A> {
    pub fn new(
        otbn_registers: StaticRef<OtbnRegisters>,
        otbn_mux: &'a TimeoutMux<'a, A, OtbnOperation<'a, A>>,
    ) -> Self {
        CryptolibMux {
            otbn_registers,
            otbn_mux,
        }
    }

    pub fn submit_otbn_job(
        &self,
        job: OtbnOperation<'a, A>,
        timeout: A::Ticks,
    ) -> Result<(), (ErrorCode, OtbnOperation<'a, A>)> {
        self.otbn_mux.submit_job(job, timeout)
    }
}

/// Handlers required to implement to schedule a job on the OTBN scheduler.
pub trait OtbnJob<'a, A: Alarm<'a>> {
    fn setup(&mut self) -> Result<(), ErrorCode>;
    fn parent(&mut self) -> &'a CryptolibMux<'a, A>;
    fn on_complete(&mut self, status: Result<(), ErrorCode>);
    fn on_timeout(&self);
}

pub enum OtbnOperation<'a, A: Alarm<'a>> {
    EcdsaVerifyP256(EcdsaVerifyP256Job<'a, A>),
    EcdsaVerifyP384(EcdsaVerifyP384Job<'a, A>),
}

impl<'a, A: Alarm<'a>> Job for OtbnOperation<'a, A> {
    fn setup(&mut self) -> Result<(), ErrorCode> {
        match self {
            OtbnOperation::EcdsaVerifyP256(state) => OtbnJob::setup(state),
            OtbnOperation::EcdsaVerifyP384(state) => OtbnJob::setup(state),
        }
    }

    fn status(&mut self) -> Result<bool, ErrorCode> {
        // Check the OTBN status register
        Ok(match self {
            OtbnOperation::EcdsaVerifyP256(state) => OtbnJob::parent(state)
                .otbn_registers
                .status
                .matches_all(STATUS::STATUS::IDLE),
            OtbnOperation::EcdsaVerifyP384(state) => OtbnJob::parent(state)
                .otbn_registers
                .status
                .matches_all(STATUS::STATUS::IDLE),
        })
    }

    fn on_complete(&mut self, status: Result<(), ErrorCode>) {
        match self {
            OtbnOperation::EcdsaVerifyP256(state) => OtbnJob::on_complete(state, status),
            OtbnOperation::EcdsaVerifyP384(state) => OtbnJob::on_complete(state, status),
        }
    }

    fn on_timeout(&self) {
        match self {
            OtbnOperation::EcdsaVerifyP256(state) => OtbnJob::on_timeout(state),
            OtbnOperation::EcdsaVerifyP384(state) => OtbnJob::on_timeout(state),
        }
    }
}
