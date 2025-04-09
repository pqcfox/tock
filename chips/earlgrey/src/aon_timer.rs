// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use kernel::utilities::StaticRef;
pub use lowrisc::aon_timer::AonTimer;
use lowrisc::registers::aon_timer_regs::AonTimerRegisters;

use crate::registers::top_earlgrey::AON_TIMER_AON_BASE_ADDR;

pub const AON_TIMER_BASE: StaticRef<AonTimerRegisters> =
    unsafe { StaticRef::new(AON_TIMER_AON_BASE_ADDR as *const AonTimerRegisters) };

pub static mut AON_TIMER: AonTimer<'static> = AonTimer::new(AON_TIMER_BASE);
