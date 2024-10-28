// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! RV Core Ibex Driver

use crate::registers::rv_core_ibex_regs::{RvCoreIbexRegisters, NMI_STATE};

use kernel::utilities::registers::interfaces::Writeable;
use kernel::utilities::StaticRef;

// The Ibex-specific mcause value for an external NMI, as detailed in the
// Ibex Reference Guide, section "Exceptions and Interrupts".
pub const IBEX_EXTERNAL_NMI_MCAUSE: usize = 0x8000001F;

pub struct RvCoreIbex {
    registers: StaticRef<RvCoreIbexRegisters>,
}

impl RvCoreIbex {
    pub const fn new(register_base: usize) -> RvCoreIbex {
        RvCoreIbex {
            // SAFETY: we need a reference here to the register base.
            registers: unsafe { StaticRef::new(register_base as *const RvCoreIbexRegisters) },
        }
    }

    /// Clear the watchdog NMI bit in NMI_STATE to stop the watchdog from re-firing
    pub fn clear_wdog_nmi(&self) {
        self.registers.nmi_state.write(NMI_STATE::WDOG::SET);
    }
}
