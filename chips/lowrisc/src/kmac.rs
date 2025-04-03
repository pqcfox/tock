// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan KMAC driver (stub).

use crate::registers::kmac_regs::{KmacRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

pub struct Kmac<'a> {
    base: StaticRef<KmacRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum KmacInterrupt {
    /// KMAC/SHA3 absorbing has been completed
    KmacDone,
    /// The message FIFO is empty.
    ///
    /// This interrupt is raised only if the message FIFO is actually writable
    /// by software, i.e., if all of the following conditions are met:
    ///
    /// i) The KMAC block is not exercised by a hardware application interface.
    /// ii) The SHA3 block is in the Absorb state.
    /// iii) Software has not yet written the Process command to finish the
    /// absorption process.
    ///
    /// For the interrupt to be raised, the message FIFO must also have been
    /// full previously.  Otherwise, the hardware empties the FIFO faster than
    /// software can fill it and there is no point in interrupting the software
    /// to inform it about the message FIFO being empty.
    FifoEmpty,
    /// KMAC/SHA3 error occurred. ERR_CODE register shows the details
    KmacErr,
}

impl<'a> Kmac<'a> {
    /// Constructs a new KMAC driver.
    pub fn new(base: StaticRef<KmacRegisters>) -> Kmac<'a> {
        Kmac {
            base,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for KMAC interrupts.
    pub fn handle_interrupt(&self, interrupt: KmacInterrupt) {
        match interrupt {
            KmacInterrupt::KmacDone => {
                self.base.intr_state.modify(INTR::KMAC_DONE::SET);
                // TODO: handle this
            }
            KmacInterrupt::FifoEmpty => {
                self.base.intr_state.modify(INTR::FIFO_EMPTY::SET);
                // TODO: handle this
            }
            KmacInterrupt::KmacErr => {
                self.base.intr_state.modify(INTR::KMAC_ERR::SET);
                // TODO: handle this
            }
        }
    }
}
