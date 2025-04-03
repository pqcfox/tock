// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan Entropy Source driver (stub).

use crate::registers::entropy_src_regs::{EntropySrcRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

pub struct EntropySrc<'a> {
    base: StaticRef<EntropySrcRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum EntropySrcInterrupt {
    /// Asserted when entropy source bits are available for firmware for
    /// consumption via `ENTROPY_DATA` register.
    EsEntropyValid,
    /// Asserted whenever the main state machine is in the alert state, e.g.,
    /// due to health tests failing and reaching the threshold value configured
    /// in `ALERT_THRESHOLD`.
    EsHealthTestFailed,
    /// Asserted when the observe FIFO has filled to the configured threshold
    /// level (see `OBSERVE_FIFO_THRESH`).
    EsObserveFifoReady,
    /// Asserted when an fatal error condition is met, e.g., upon FIFO errors,
    /// or if an illegal state machine state is reached
    EsFatalErr,
}

impl<'a> EntropySrc<'a> {
    /// Constructs a new Entropy Source driver.
    pub fn new(base: StaticRef<EntropySrcRegisters>) -> EntropySrc<'a> {
        EntropySrc {
            base,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for Entropy Source interrupts.
    pub fn handle_interrupt(&self, interrupt: EntropySrcInterrupt) {
        match interrupt {
            EntropySrcInterrupt::EsEntropyValid => {
                self.base.intr_state.modify(INTR::ES_ENTROPY_VALID::SET);
                // TODO: handle this
            }
            EntropySrcInterrupt::EsHealthTestFailed => {
                self.base
                    .intr_state
                    .modify(INTR::ES_HEALTH_TEST_FAILED::SET);
                // TODO: handle this
            }
            EntropySrcInterrupt::EsObserveFifoReady => {
                self.base
                    .intr_state
                    .modify(INTR::ES_OBSERVE_FIFO_READY::SET);
                // TODO: handle this
            }
            EntropySrcInterrupt::EsFatalErr => {
                self.base.intr_state.modify(INTR::ES_FATAL_ERR::SET);
                // TODO: handle this
            }
        }
    }
}
