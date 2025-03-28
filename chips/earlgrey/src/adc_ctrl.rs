// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use kernel::utilities::StaticRef;
pub use lowrisc::adc_ctrl::AdcCtrl;
use lowrisc::registers::adc_ctrl_regs::AdcCtrlRegisters;

use crate::registers::top_earlgrey::ADC_CTRL_AON_BASE_ADDR;

pub const ADC_CTRL_BASE: StaticRef<AdcCtrlRegisters> =
    unsafe { StaticRef::new(ADC_CTRL_AON_BASE_ADDR as *const AdcCtrlRegisters) };
