// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! AON/Watchdog Timer Driver

use crate::registers::aon_timer_regs::{
    AonTimerRegisters, ALERT_TEST, INTR_STATE, WDOG_BARK_THOLD, WDOG_BITE_THOLD, WDOG_CTRL,
    WDOG_REGWEN, WKUP_CTRL,
};
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::StaticRef;
use kernel::{platform, ErrorCode};

/// Peripheral base address for aon_timer_aon in top earlgrey.
///
/// This should be used with #mmio_region_from_addr to access the memory-mapped
/// registers associated with the peripheral (usually via a DIF).
pub const AON_TIMER_AON_BASE_ADDR: usize = 0x40470000;

pub const AON_TIMER_BASE: StaticRef<AonTimerRegisters> =
    unsafe { StaticRef::new(AON_TIMER_AON_BASE_ADDR as *const AonTimerRegisters) };

pub struct AonTimer {
    registers: StaticRef<AonTimerRegisters>,
    aon_clk_freq: u32, //Hz, this differs for FPGA/Verilator
}

impl AonTimer {
    pub const fn new(aon_clk_freq: u32) -> AonTimer {
        AonTimer {
            registers: AON_TIMER_BASE,
            aon_clk_freq,
        }
    }
    fn wakeup_set_enable(&self, prescaler: u32, enable: bool) -> Result<(), ErrorCode> {
        if prescaler >= 4096 {
            return Err(ErrorCode::INVAL);
        }
        match (prescaler, enable) {
            (_, true) => self
                .registers
                .wkup_ctrl
                .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::SET),
            (_, false) => self
                .registers
                .wkup_ctrl
                .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::CLEAR),
        }
        Ok(())
    }

    fn wakeup_set_enable(&self, prescaler: u32, enable: bool) -> Result<(), ErrorCode> {
        if prescaler >= 4096 {
            return Err(ErrorCode::INVAL);
        }
        match (prescaler, enable) {
            (_, true) => self
                .registers
                .wkup_ctrl
                .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::SET),
            (_, false) => self
                .registers
                .wkup_ctrl
                .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::CLEAR),
        }
        Ok(())
    }

    fn wakeup_set_enable(&self, prescaler: u32, enable: bool) -> Result<(), ErrorCode> {
        if prescaler >= 4096 {
            return Err(ErrorCode::INVAL);
        }
        match (prescaler, enable) {
            (_, true) => self
                .registers
                .wkup_ctrl
                .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::SET),
            (_, false) => self
                .registers
                .wkup_ctrl
                .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::CLEAR),
        }
        Ok(())
    }

    /// Reset both watch dog and wake up timer count values.
    fn reset_timers(&self) {
        self.registers.wkup_count.set(0x00);
        self.registers.wdog_count.set(0x00);
    }

    /// Start the watchdog counter with pause in sleep
    /// i.e wdog timer is paused when system is sleeping
    fn wdog_start_count(&self, count_in_sleep: bool) {
        match count_in_sleep {
            true => self
                .registers
                .wdog_ctrl
                .write(WDOG_CTRL::ENABLE::SET + WDOG_CTRL::PAUSE_IN_SLEEP::CLEAR),
            false => self
                .registers
                .wdog_ctrl
                .write(WDOG_CTRL::ENABLE::SET + WDOG_CTRL::PAUSE_IN_SLEEP::SET),
        }
    }

    /// Program the desired thresholds in WKUP_THOLD, WDOG_BARK_THOLD and WDOG_BITE_THOLD
    fn set_wdog_thresh(&self) {
        // Watchdog period may need to be revised with kernel changes/updates
        // since the watchdog is `tickled()` at the start of every kernel loop
        // see: https://github.com/tock/tock/blob/eb3f7ce59434b7ac1b77ef1ab7dd2afad1a62ac5/kernel/src/kernel.rs#L448
        let bark_cycles = self.ms_to_cycles(500);
        // ~1000ms bite period
        let bite_cycles = bark_cycles.saturating_mul(2);

        self.registers
            .wdog_bark_thold
            .write(WDOG_BARK_THOLD::THRESHOLD.val(bark_cycles));
        self.registers
            .wdog_bite_thold
            .write(WDOG_BITE_THOLD::THRESHOLD.val(bite_cycles));
    }

    // Reset watch dog timer
    fn wdog_pet(&self) {
        self.registers.wdog_count.set(0x00);
    }

    fn wdog_suspend(&self) {
        self.registers.wdog_ctrl.write(WDOG_CTRL::ENABLE::CLEAR);
    }

    fn wdog_resume(&self) {
        self.registers.wdog_ctrl.write(WDOG_CTRL::ENABLE::SET);
    }

    /// Locks further config to WDOG until next system reset
    fn lock_wdog_conf(&self) {
        self.registers.wdog_regwen.write(WDOG_REGWEN::REGWEN::SET)
    }

    /// Convert microseconds to cycles
    fn ms_to_cycles(&self, ms: u32) -> u32 {
        // 250kHZ CW130 or 125kHz Verilator (as specified in chip config)
        ms.saturating_mul(self.aon_clk_freq).saturating_div(1000)
    }

    fn reset_wkup_count(&self) {
        self.registers.wkup_count.set(0x00);
    }

    pub fn handle_interrupt(&self) {
        let regs = self.registers;
        let intr = self.registers.intr_state.extract();

        if intr.is_set(INTR_STATE::WKUP_TIMER_EXPIRED) {
            // Wake up timer has expired, sw must ack and clear
            regs.wkup_cause.set(0x00);
            regs.wkup_count.set(0x00); // To avoid re-triggers
            self.reset_wkup_count();
            // RW1C, clear the interrupt
            regs.intr_state.write(INTR_STATE::WKUP_TIMER_EXPIRED::SET);
        }

        if intr.is_set(INTR_STATE::WDOG_TIMER_BARK) {
            // Clear the bark (RW1C) and pet doggo
            regs.intr_state.write(INTR_STATE::WDOG_TIMER_BARK::SET);
            self.wdog_pet();
        }
    }
}

impl platform::watchdog::WatchDog for AonTimer {
    /// The always-on timer will run on a ~125KHz (Verilator) or ~250kHz clock.
    /// The timers themselves are 32b wide, giving a maximum timeout
    /// window of roughly ~6 hours. For the wakeup timer, the pre-scaler
    /// extends the maximum timeout to ~1000 days.
    ///
    /// The AON HW_IP has a watchdog and a wake-up timer (counts independantly of eachother),
    /// although struct `AonTimer` implements the wakeup timer functionality,
    /// we only start and use the watchdog in the code below.
    fn setup(&self) {
        // 1. Clear Timers
        self.reset_timers();

        // 2. Set thresholds.
        self.set_wdog_thresh();

        // 3. Commence gaurd duty and don't count it in sleep.
        self.wdog_start_count(false);

        // 4. Lock watchdog config
        // Preventing firmware from accidentally or maliciously disabling the watchdog,
        // until next system reset.
        self.lock_wdog_conf();
    }

    fn tickle(&self) {
        // Nothing to worry about, good dog...
        self.wdog_pet();
    }

    fn suspend(&self) {
        self.wdog_suspend();
    }

    fn resume(&self) {
        self.wdog_resume();
    }
}
