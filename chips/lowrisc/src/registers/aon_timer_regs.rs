// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright lowRISC contributors (OpenTitan project).

// Generated register constants for aon_timer.
// Original reference file: hw/ip/aon_timer/data/aon_timer.hjson
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::registers::{register_bitfields, register_structs};
/// Number of alerts
pub const AON_TIMER_PARAM_NUM_ALERTS: u32 = 1;
/// Register width
pub const AON_TIMER_PARAM_REG_WIDTH: u32 = 32;

register_structs! {
    pub AonTimerRegisters {
        /// Alert Test Register
        (0x0000 => pub alert_test: ReadWrite<u32, ALERT_TEST::Register>),
        /// Wakeup Timer Control register
        (0x0004 => pub wkup_ctrl: ReadWrite<u32, WKUP_CTRL::Register>),
        /// Wakeup Timer Threshold Register (bits 63 - 32)
        (0x0008 => pub wkup_thold_hi: ReadWrite<u32, WKUP_THOLD_HI::Register>),
        /// Wakeup Timer Threshold Register (bits 31 - 0)
        (0x000c => pub wkup_thold_lo: ReadWrite<u32, WKUP_THOLD_LO::Register>),
        /// Wakeup Timer Count Register (bits 63 - 32)
        (0x0010 => pub wkup_count_hi: ReadWrite<u32, WKUP_COUNT_HI::Register>),
        /// Wakeup Timer Count Register (bits 31 - 0)
        (0x0014 => pub wkup_count_lo: ReadWrite<u32, WKUP_COUNT_LO::Register>),
        /// Watchdog Timer Write Enable Register
        (0x0018 => pub wdog_regwen: ReadWrite<u32, WDOG_REGWEN::Register>),
        /// Watchdog Timer Control register
        (0x001c => pub wdog_ctrl: ReadWrite<u32, WDOG_CTRL::Register>),
        /// Watchdog Timer Bark Threshold Register
        (0x0020 => pub wdog_bark_thold: ReadWrite<u32, WDOG_BARK_THOLD::Register>),
        /// Watchdog Timer Bite Threshold Register
        (0x0024 => pub wdog_bite_thold: ReadWrite<u32, WDOG_BITE_THOLD::Register>),
        /// Watchdog Timer Count Register
        (0x0028 => pub wdog_count: ReadWrite<u32, WDOG_COUNT::Register>),
        /// Interrupt State Register
        (0x002c => pub intr_state: ReadWrite<u32, INTR_STATE::Register>),
        /// Interrupt Test Register
        (0x0030 => pub intr_test: ReadWrite<u32, INTR_TEST::Register>),
        /// Wakeup request status
        (0x0034 => pub wkup_cause: ReadWrite<u32, WKUP_CAUSE::Register>),
        (0x0038 => @END),
    }
}

register_bitfields![u32,
    pub ALERT_TEST [
        FATAL_FAULT OFFSET(0) NUMBITS(1) [],
    ],
    pub WKUP_CTRL [
        ENABLE OFFSET(0) NUMBITS(1) [],
        PRESCALER OFFSET(1) NUMBITS(12) [],
    ],
    pub WKUP_THOLD_HI [
        THRESHOLD_HI OFFSET(0) NUMBITS(32) [],
    ],
    pub WKUP_THOLD_LO [
        THRESHOLD_LO OFFSET(0) NUMBITS(32) [],
    ],
    pub WKUP_COUNT_HI [
        COUNT_HI OFFSET(0) NUMBITS(32) [],
    ],
    pub WKUP_COUNT_LO [
        COUNT_LO OFFSET(0) NUMBITS(32) [],
    ],
    pub WDOG_REGWEN [
        REGWEN OFFSET(0) NUMBITS(1) [],
    ],
    pub WDOG_CTRL [
        ENABLE OFFSET(0) NUMBITS(1) [],
        PAUSE_IN_SLEEP OFFSET(1) NUMBITS(1) [],
    ],
    pub WDOG_BARK_THOLD [
        THRESHOLD OFFSET(0) NUMBITS(32) [],
    ],
    pub WDOG_BITE_THOLD [
        THRESHOLD OFFSET(0) NUMBITS(32) [],
    ],
    pub WDOG_COUNT [
        COUNT OFFSET(0) NUMBITS(32) [],
    ],
    pub INTR_STATE [
        WKUP_TIMER_EXPIRED OFFSET(0) NUMBITS(1) [],
        WDOG_TIMER_BARK OFFSET(1) NUMBITS(1) [],
    ],
    pub INTR_TEST [
        WKUP_TIMER_EXPIRED OFFSET(0) NUMBITS(1) [],
        WDOG_TIMER_BARK OFFSET(1) NUMBITS(1) [],
    ],
    pub WKUP_CAUSE [
        CAUSE OFFSET(0) NUMBITS(1) [],
    ],
];

// End generated register constants for aon_timer
