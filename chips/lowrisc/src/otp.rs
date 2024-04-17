// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::create_non_zero_usize;

use crate::registers::otp_ctrl_regs::{
    OtpCtrlRegisters, STATUS
};

use kernel::utilities::StaticRef;
use kernel::utilities::registers::interfaces::Readable;

use core::num::NonZeroUsize;

const NUMBER_ERRORS: NonZeroUsize = create_non_zero_usize!(14);
const MASK_ERRORS: NonZeroUsize = create_non_zero_usize!((1 << NUMBER_ERRORS.get()) - 1);

pub struct Otp {
    registers: StaticRef<OtpCtrlRegisters>
}

impl Otp {
    pub fn new(registers: StaticRef<OtpCtrlRegisters>) -> Self {
        Self {
            registers
        }
    }

    pub fn init(&self) -> Result<(), ()> {
        let status_value = self.registers.status.get();
        // CAST: usize == u32 on RV32I
        if status_value & (MASK_ERRORS.get() as u32) != 0 {
            Err(())
        } else {
            Ok(())
        }
    }
}
