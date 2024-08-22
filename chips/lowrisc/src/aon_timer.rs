// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! AON/Watchdog Timer Driver

use crate::registers::aon_timer_regs::{
    AonTimerRegisters, ALERT_TEST, INTR_STATE, INTR_TEST, WDOG_BARK_THOLD, WDOG_BITE_THOLD,
    WDOG_CTRL, WDOG_REGWEN, WKUP_COUNT, WKUP_CTRL, WKUP_THOLD,
};
use core::fmt::Write;
use kernel::hil::reset_managment::ResetManagment;
use kernel::hil::retention_ram::{CreatorRetentionRam, OwnerRetentionRam};
use kernel::hil::uart::{TransmitSynch, Uart};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::target_test::{self, TargetTests};
use kernel::utilities::StaticRef;
use kernel::{debug, platform, ErrorCode};
pub struct AonTimer<'a> {
    registers: StaticRef<AonTimerRegisters>,
    wakeup_notification: OptionalCell<&'a dyn Fn()>,
    bark_notification: OptionalCell<&'a dyn Fn()>,
    aon_clk_freq: u32, //Hz, this differs for FPGA/Verilator
}

impl<'a> AonTimer<'a> {
    pub const fn new(register_base: usize, aon_clk_freq: u32) -> AonTimer<'a> {
        AonTimer {
            // SAFETY: We keed a reference here to the register base.
            registers: unsafe { StaticRef::new(register_base as *const AonTimerRegisters) },
            wakeup_notification: OptionalCell::empty(),
            bark_notification: OptionalCell::empty(),
            aon_clk_freq,
        }
    }

    fn wakeup_set_prescaler_and_enable(&self, prescaler: u32) -> Result<(), ErrorCode> {
        if prescaler >= 4096 {
            return Err(ErrorCode::INVAL);
        }
        self.registers
            .wkup_ctrl
            .write(WKUP_CTRL::PRESCALER.val(prescaler) + WKUP_CTRL::ENABLE::SET);
        Ok(())
    }

    pub fn wakeup_disable(&self) {
        self.registers.wkup_ctrl.write(WKUP_CTRL::ENABLE::CLEAR);
    }

    // Enable wakeup after a number of milliseconds. This can fail if the ms number is out of range.
    pub fn wakeup_enable_after_ms(&self, ms: u32) -> Result<(), ErrorCode> {
        let wakeup_cycles = self.ms_to_cycles(ms);

        self.wakeup_disable();

        self.reset_wkup();

        self.registers
            .wkup_thold
            .write(WKUP_THOLD::THRESHOLD.val(wakeup_cycles));

        self.wakeup_set_prescaler_and_enable(0)
    }

    /// Reset  wake up timer count value.
    pub fn reset_wkup(&self) {
        self.registers.wkup_count.set(0x00);
    }

    /// Get the ms remaining until wakeup will happen.
    pub fn get_ms_to_wkup(&self) -> u32 {
        self.cycles_to_ms(
            // The wakeup register addition can not overflow because of the possible range of the prescaler register.
            self.registers
                .wkup_thold
                .read(WKUP_THOLD::THRESHOLD)
                .saturating_sub(
                    self.registers
                        .wkup_count
                        .read(WKUP_COUNT::COUNT)
                        .saturating_mul(self.registers.wkup_ctrl.read(WKUP_CTRL::PRESCALER) + 1),
                ),
        )
    }

    /// Function to register a callback for the wakeup event.
    pub fn register_wakeup_callback(&self, callback: Option<&'a dyn Fn()>) {
        self.wakeup_notification.insert(callback);
    }

    /// Reset watch dog timer count value.
    fn reset_wdog(&self) {
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
    pub fn set_wdog_bite_thresh_ms(&self, ms: u32) {
        // Watchdog period may need to be revised with kernel changes/updates
        // since the watchdog is `tickled()` at the start of every kernel loop
        // see: https://github.com/tock/tock/blob/eb3f7ce59434b7ac1b77ef1ab7dd2afad1a62ac5/kernel/src/kernel.rs#L448
        let bite_cycles = self.ms_to_cycles(ms);

        self.registers
            .wdog_bite_thold
            .write(WDOG_BITE_THOLD::THRESHOLD.val(bite_cycles));
    }

    /// Program the desired thresholds in WKUP_THOLD, WDOG_BARK_THOLD and WDOG_BITE_THOLD
    fn set_wdog_bark_thresh_ms(&self, ms: u32) {
        // Watchdog period may need to be revised with kernel changes/updates
        // since the watchdog is `tickled()` at the start of every kernel loop
        // see: https://github.com/tock/tock/blob/eb3f7ce59434b7ac1b77ef1ab7dd2afad1a62ac5/kernel/src/kernel.rs#L448
        let bark_cycles = self.ms_to_cycles(ms);

        self.registers
            .wdog_bark_thold
            .write(WDOG_BARK_THOLD::THRESHOLD.val(bark_cycles));
    }

    /// Function to register a callback for the watchdog bark event.
    pub fn register_watchdog_bark_callback(&self, callback: Option<&'a dyn Fn()>) {
        self.bark_notification.insert(callback);
    }

    // Reset watch dog timer
    fn wdog_pet(&self) {
        self.registers.wdog_count.set(0x00);
    }

    /// Temporarily disable the watchdog without resetting the counter register.
    fn wdog_suspend(&self) {
        self.registers.wdog_ctrl.write(WDOG_CTRL::ENABLE::CLEAR);
    }

    /// Resume the watchdog and continue from where the counter register left off.
    fn wdog_resume(&self) {
        self.registers.wdog_ctrl.write(WDOG_CTRL::ENABLE::SET);
    }

    /// Locks further config to WDOG until next system reset
    fn lock_wdog_conf(&self) {
        self.registers.wdog_regwen.write(WDOG_REGWEN::REGWEN::SET)
    }

    /// Convert miliseconds to clock cycles
    fn ms_to_cycles(&self, ms: u32) -> u32 {
        // 250kHZ CW310 or 125kHz Verilator (as specified in chip config)
        ms.saturating_mul(self.aon_clk_freq).saturating_div(1000)
    }

    /// Convert clock cycles to miliseconds
    fn cycles_to_ms(&self, ms: u32) -> u32 {
        // 250kHZ CW310 or 125kHz Verilator (as specified in chip config)
        ms.saturating_mul(1000).saturating_div(self.aon_clk_freq)
    }

    /// Function for handling interrupts related to wakeup and watchdog barks.
    pub fn handle_interrupt(&self) {
        let regs = self.registers;
        let intr = self.registers.intr_state.extract();

        if intr.is_set(INTR_STATE::WKUP_TIMER_EXPIRED) {
            // Wake up timer has expired, sw must ack and clear
            regs.wkup_cause.set(0x00);
            regs.wkup_count.set(0x00); // To avoid re-triggers
            self.reset_wkup();
            // RW1C, clear the interrupt
            regs.intr_state.write(INTR_STATE::WKUP_TIMER_EXPIRED::SET);
            self.wakeup_notification.map(|a| a());
        }

        if intr.is_set(INTR_STATE::WDOG_TIMER_BARK) {
            // Clear the bark (RW1C) and pet doggo
            regs.intr_state.write(INTR_STATE::WDOG_TIMER_BARK::SET);
            self.wdog_pet();
            self.bark_notification.map(|a| a());
        }
    }

    pub fn test(
        &self,
        reset_manager: &dyn ResetManagment<ResetInfo = [u32; 19]>,
        uart: &dyn TransmitSynch,
        creator_ram: &dyn CreatorRetentionRam<Data = u32, ID = usize>,
        owner_ram: &dyn OwnerRetentionRam<Data = u32, ID = usize>,
    ) -> bool {
        let mut test_runner = target_test::TestRunner::new();
        let binding = |foo: &str| uart.transmit_sync(foo.as_bytes());
        test_runner.set_print_func(&binding);
        test_runner
            .write_str("Starting aon_timer self-test \r\n")
            .unwrap();
        let mut test_cycle: u32 = 0;
        match creator_ram.read(1) {
            Ok(1) => {
                test_runner
                    .write_str("Reset reason from API is PURES! Resetting run counter! \r\n")
                    .unwrap();
                owner_ram.write(1, 1).unwrap();
                test_cycle = 0;
            }
            Ok(x) => {
                test_runner
                    .write_fmt(format_args!("Reset reason from API is {} \r\n", x))
                    .unwrap();
                test_cycle = owner_ram.read(1).unwrap();
                owner_ram.write(1, test_cycle + 1).unwrap();
            }
            _ => test_runner
                .write_str("Wrong init state, can't read reset reason yet!  \r\n")
                .unwrap(),
        }

        match test_cycle {
            0 => {
                test_runner.assert_function(
                    "Test wakeup prescaler boundries check negative case!",
                    || self.wakeup_set_prescaler_and_enable(4096) == Err(ErrorCode::INVAL),
                );
                test_runner.assert_function(
                    "Test wakeup prescaler boundries check OK case!",
                    || {
                        self.wakeup_set_prescaler_and_enable(4095) == Ok(())
                            && self.registers.wkup_ctrl.read(WKUP_CTRL::PRESCALER) == 4095
                            && self.registers.wkup_ctrl.read(WKUP_CTRL::ENABLE) == 1
                    },
                );
                test_runner.assert_function("Test wakeup_disable!", || {
                    self.wakeup_disable();
                    self.registers.wkup_ctrl.read(WKUP_CTRL::ENABLE) == 0
                });

                test_runner
                    .assert_function("Test wakeup prescaler boundries check OK case!", || {
                        self.wakeup_set_prescaler_and_enable(0) == Ok(())
                    });

                test_runner.assert_function("Enable wakeup fail because of boundaries!", || {
                    self.wakeup_enable_after_ms(1000) == Err(ErrorCode::INVAL)
                });

                test_runner.assert_function("Enable wakeup fail because of boundaries!", || {
                    self.wakeup_enable_after_ms(1000) == Err(ErrorCode::INVAL)
                });
            }
            1 => {
                // runner.assert_function("We woke up after sleep!", || {
                //     self.get_owner_rram_data(10) == 0x5A
                // });
            }
            _ => {}
        }

        debug!("Ending aon_timer self-test");
        test_runner.is_test_failed
    }
}

impl<'a> platform::watchdog::WatchDog for AonTimer<'a> {
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
        self.reset_wdog();

        // 2. Set thresholds.
        self.set_wdog_bark_thresh_ms(500);
        self.set_wdog_bite_thresh_ms(1000);

        // 3. Commence guard duty and don't count it in sleep.
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
