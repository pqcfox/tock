// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan EDN driver (stub).

// TODO: implement this

use crate::registers::edn_regs::{EdnRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

pub struct Edn<'a> {
    base: StaticRef<EdnRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum EdnInterrupt {
    /// Asserted when a software CSRNG request has completed.
    EdnCmdReqDone,
    /// Asserted when a FIFO error occurs.
    EdnFatalErr,
}

impl<'a> Edn<'a> {
    /// Constructs a new EDN driver.
    pub fn new(base: StaticRef<EdnRegisters>) -> Edn<'a> {
        Edn {
            base,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for EDN interrupts.
    pub fn handle_interrupt(&self, interrupt: EdnInterrupt) {
        match interrupt {
            EdnInterrupt::EdnCmdReqDone => {
                self.base.intr_state.modify(INTR::EDN_CMD_REQ_DONE::SET);
                // TODO: handle this
            }
            EdnInterrupt::EdnFatalErr => {
                self.base.intr_state.modify(INTR::EDN_FATAL_ERR::SET);
                // TODO: handle this
            }
        }
    }
}
