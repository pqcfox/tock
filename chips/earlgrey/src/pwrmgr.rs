// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Power Mangement for LowRISC

use crate::registers::pwrmgr_regs::{PwrmgrRegisters, CFG_CDC_SYNC, CONTROL};
use crate::registers::top_earlgrey::PWRMGR_AON_BASE_ADDR;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::StaticRef;

pub(crate) const PWRMGR_BASE: StaticRef<PwrmgrRegisters> =
    unsafe { StaticRef::new(PWRMGR_AON_BASE_ADDR as *const PwrmgrRegisters) };

pub struct PwrMgr {
    registers: StaticRef<PwrmgrRegisters>,
}

impl PwrMgr {
    pub const fn new(base: StaticRef<PwrmgrRegisters>) -> PwrMgr {
        PwrMgr { registers: base }
    }

    pub fn check_clock_propagation(&self) -> bool {
        let regs = self.registers;

        if regs.cfg_cdc_sync.read(CFG_CDC_SYNC::SYNC) == 0 {
            return true;
        }

        false
    }

    pub fn handle_interrupt(&self) {
        let regs = self.registers;

        // Disable power saving
        regs.control.write(CONTROL::LOW_POWER_HINT::CLEAR);

        // Propagate changes to slow clock domain
        regs.cfg_cdc_sync.write(CFG_CDC_SYNC::SYNC::SET);
    }

    pub fn enable_low_power(&self) {
        let regs = self.registers;

        if regs.control.read(CONTROL::LOW_POWER_HINT) != 1 {
            // Next WFI should trigger low power entry
            // Leave the IO clock enabled as we need to get interrupts
            // regs.control.write(
            //     CONTROL::LOW_POWER_HINT::SET
            //         + CONTROL::CORE_CLK_EN::CLEAR
            //         + CONTROL::IO_CLK_EN::SET
            //         + CONTROL::MAIN_PD_N::CLEAR,
            // );

            // Propagate changes to slow clock domain
            regs.cfg_cdc_sync.write(CFG_CDC_SYNC::SYNC::SET);
        }
    }
}
