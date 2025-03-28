// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! OpenTitan Sensor Control driver (stub)

// TODO: Implement this

use crate::registers::sensor_ctrl_regs::{SensorCtrlRegisters, INTR};
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::StaticRef;

pub(crate) const SENSOR_CTRL_BASE: StaticRef<SensorCtrlRegisters> = unsafe {
    StaticRef::new(
        crate::registers::top_earlgrey::SENSOR_CTRL_AON_BASE_ADDR as *const SensorCtrlRegisters,
    )
};

pub struct SensorCtrl<'a> {
    base: StaticRef<SensorCtrlRegisters>,
    _temp: core::marker::PhantomData<&'a ()>,
}

#[derive(Clone, Copy)]
pub enum SensorCtrlInterrupt {
    /// io power status has changed
    IoStatusChange,
    /// ast init status has changed
    InitStatusChange,
}

impl<'a> SensorCtrl<'a> {
    /// Constructs a new SPI device driver.
    pub fn new() -> SensorCtrl<'a> {
        SensorCtrl {
            base: SENSOR_CTRL_BASE,
            _temp: core::marker::PhantomData,
        }
    }

    /// Handler for Sensor Control interrupts.
    pub fn handle_interrupt(&self, interrupt: SensorCtrlInterrupt) {
        match interrupt {
            SensorCtrlInterrupt::IoStatusChange => {
                self.base.intr_state.modify(INTR::IO_STATUS_CHANGE::SET);
                // TODO: handle this interrupt
            }
            SensorCtrlInterrupt::InitStatusChange => {
                self.base.intr_state.modify(INTR::INIT_STATUS_CHANGE::SET);
                // TODO: handle this interrupt
            }
        }
    }
}
