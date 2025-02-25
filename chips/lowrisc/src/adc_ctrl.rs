// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan ADC controller driver (stub)

// TODO: Implement this

use crate::registers::adc_ctrl_regs::{AdcCtrlRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

pub struct AdcCtrl<'a> {
    base: StaticRef<AdcCtrlRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum AdcCtrlInterrupt {
    /// ADC match or measurement event has occurred
    MatchPending,
}

impl<'a> AdcCtrl<'a> {
    /// Constructs a new SPI device driver.
    pub fn new(base: StaticRef<AdcCtrlRegisters>) -> AdcCtrl<'a> {
        AdcCtrl {
            base,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for ADC Controller interrupts.
    pub fn handle_interrupt(&self, interrupt: AdcCtrlInterrupt) {
        match interrupt {
            AdcCtrlInterrupt::MatchPending => {
                self.base.intr_state.modify(INTR::MATCH_PENDING::SET);
                // TODO: handle this interrupt
            }
        }
    }
}
