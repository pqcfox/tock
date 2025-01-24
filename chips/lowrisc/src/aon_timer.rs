// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! AON/Watchdog Timer Driver

use crate::registers::aon_timer_regs::{
    AonTimerRegisters, INTR_STATE, WDOG_BARK_THOLD, WDOG_BITE_THOLD, WDOG_CTRL, WDOG_REGWEN,
    WKUP_COUNT_HI, WKUP_COUNT_LO, WKUP_CTRL, WKUP_THOLD_HI, WKUP_THOLD_LO,
};

use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::StaticRef;
use kernel::{platform, ErrorCode};

#[cfg(feature = "test_aon_timer")]
use {
    core::fmt::Write,
    kernel::{
        hil::{
            retention_ram::{CreatorRetentionRam, OwnerRetentionRam},
            uart::TransmitSynch,
        },
        utilities::target_test,
    },
};

// Note that, when comparing to upstream Tock, the named lifetime of this
// struct has been replaced everywhere with 'static.
pub struct AonTimer<'a> {
    registers: StaticRef<AonTimerRegisters>,
    wakeup_notification: OptionalCell<&'a dyn Fn()>,
    bark_notification: OptionalCell<&'a dyn Fn()>,
    aon_clk_freq: OptionalCell<u32>, //Hz, this differs for FPGA/Verilator
}

impl<'a> AonTimer<'a> {
    pub const fn new(register_base: usize) -> AonTimer<'a> {
        AonTimer {
            // SAFETY: We need a reference here to the register base.
            registers: unsafe { StaticRef::new(register_base as *const AonTimerRegisters) },
            wakeup_notification: OptionalCell::empty(),
            bark_notification: OptionalCell::empty(),
            aon_clk_freq: OptionalCell::empty(),
        }
    }

    pub fn set_clk_freq(&self, freq: u32) {
        self.aon_clk_freq.insert(Some(freq));
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
    pub fn wakeup_enable_after_ms(&self, ms: u64) -> Result<(), ErrorCode> {
        let wakeup_cycles = self.ms_to_cycles(ms)?;

        self.wakeup_disable();

        self.reset_wkup();

        // PANIC: These `unwrap` calls cannot panic because the partial values are constructed to
        // be in the range of a `u32`.
        self.registers.wkup_thold_lo.write(
            WKUP_THOLD_LO::THRESHOLD_LO
                .val(u32::try_from(wakeup_cycles & u64::from(u32::MAX)).unwrap()),
        );
        self.registers.wkup_thold_hi.write(
            WKUP_THOLD_HI::THRESHOLD_HI.val(u32::try_from(wakeup_cycles >> u32::BITS).unwrap()),
        );

        self.wakeup_set_prescaler_and_enable(0)
    }

    /// Reset  wake up timer count value.
    pub fn reset_wkup(&self) {
        self.registers.wkup_count_lo.set(0x00);
        self.registers.wkup_count_hi.set(0x00);
    }

    /// Get the ms remaining until wakeup will happen.
    ///
    /// Returns Ok(ms) if aon_clk_freq has been set via set_clk_freq,
    /// otherwise returns Err(ErrorCode::FAIL).
    pub fn get_ms_to_wkup(&self) -> Result<u64, ErrorCode> {
        let wkup_count_lo = self.registers.wkup_count_lo.read(WKUP_COUNT_LO::COUNT_LO);
        let wkup_count_hi = self.registers.wkup_count_hi.read(WKUP_COUNT_HI::COUNT_HI);
        let wkup_count = u64::from(wkup_count_lo)
            + (u64::from(wkup_count_hi) << u32::BITS).saturating_mul(u64::from(
                self.registers.wkup_ctrl.read(WKUP_CTRL::PRESCALER) + 1,
            ));
        let wkup_thold_lo = self
            .registers
            .wkup_thold_lo
            .read(WKUP_THOLD_LO::THRESHOLD_LO);
        let wkup_thold_hi = self
            .registers
            .wkup_thold_hi
            .read(WKUP_THOLD_HI::THRESHOLD_HI);
        let wkup_thold = u64::from(wkup_thold_lo) + (u64::from(wkup_thold_hi) << u32::BITS);
        // The wakeup register addition can not overflow because of the
        // possible range of the prescaler register.
        self.cycles_to_ms(wkup_thold.saturating_sub(wkup_count))
    }

    /// Function to register a callback for the wakeup event.
    pub fn register_wakeup_callback(&self, callback: Option<&'a dyn Fn()>) {
        self.wakeup_notification.insert(callback);
    }

    /// Reset watch dog timer count value.
    pub fn reset_wdog(&self) {
        self.registers.wdog_count.set(0x00);
    }

    /// Start the watchdog counter with pause in sleep
    /// i.e wdog timer is paused when system is sleeping
    pub fn wdog_start_count(&self, count_in_sleep: bool) {
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
    pub fn set_wdog_bite_thresh_ms(&self, ms: u32) -> Result<(), ErrorCode> {
        // Watchdog period may need to be revised with kernel changes/updates
        // since the watchdog is `tickled()` at the start of every kernel loop
        // see: https://github.com/tock/tock/blob/eb3f7ce59434b7ac1b77ef1ab7dd2afad1a62ac5/kernel/src/kernel.rs#L448
        let bite_cycles = match u32::try_from(self.ms_to_cycles(u64::from(ms))?) {
            Ok(c) => c,
            // Value passed was too large for a 32-bit register.
            Err(_) => return Err(ErrorCode::SIZE),
        };

        self.registers
            .wdog_bite_thold
            .write(WDOG_BITE_THOLD::THRESHOLD.val(bite_cycles));

        Ok(())
    }

    /// Program the desired thresholds in WKUP_THOLD, WDOG_BARK_THOLD and WDOG_BITE_THOLD
    pub fn set_wdog_bark_thresh_ms(&self, ms: u32) -> Result<(), ErrorCode> {
        // Watchdog period may need to be revised with kernel changes/updates
        // since the watchdog is `tickled()` at the start of every kernel loop
        // see: https://github.com/tock/tock/blob/eb3f7ce59434b7ac1b77ef1ab7dd2afad1a62ac5/kernel/src/kernel.rs#L448
        let bark_cycles = match u32::try_from(self.ms_to_cycles(u64::from(ms))?) {
            Ok(c) => c,
            // Value passed was too large for a 32-bit register.
            Err(_) => return Err(ErrorCode::SIZE),
        };

        self.registers
            .wdog_bark_thold
            .write(WDOG_BARK_THOLD::THRESHOLD.val(bark_cycles));

        Ok(())
    }

    /// Function to register a callback for the watchdog bark event.
    pub fn register_watchdog_bark_callback(&self, callback: Option<&'a dyn Fn()>) {
        self.bark_notification.insert(callback);
    }

    // Reset watch dog timer
    pub fn wdog_pet(&self) {
        self.registers.wdog_count.set(0x00);
    }

    /// Temporarily disable the watchdog without resetting the counter register.
    pub fn wdog_suspend(&self) {
        self.registers.wdog_ctrl.write(WDOG_CTRL::ENABLE::CLEAR);
    }

    /// Resume the watchdog and continue from where the counter register left off.
    pub fn wdog_resume(&self) {
        self.registers.wdog_ctrl.write(WDOG_CTRL::ENABLE::SET);
    }

    /// Locks further config to WDOG until next system reset
    pub fn lock_wdog_conf(&self) {
        self.registers.wdog_regwen.write(WDOG_REGWEN::REGWEN::SET)
    }

    /// Convert miliseconds to clock cycles
    ///
    /// Returns Ok(cycles) if aon_clk_freq has been set via set_clk_freq,
    /// otherwise returns Err(ErrorCode::FAIL).
    fn ms_to_cycles(&self, ms: u64) -> Result<u64, ErrorCode> {
        // 250kHZ CW310 or 125kHz Verilator (as specified in chip config)
        self.aon_clk_freq
            .map(|freq| ms.saturating_mul(u64::from(freq)).saturating_div(1000))
            .ok_or(ErrorCode::FAIL)
    }

    /// Convert clock cycles to miliseconds
    ///
    /// Returns Ok(ms) if aon_clk_freq has been set via set_clk_freq,
    /// otherwise returns Err(ErrorCode::FAIL).
    fn cycles_to_ms(&self, cycles: u64) -> Result<u64, ErrorCode> {
        // 250kHZ CW310 or 125kHz Verilator (as specified in chip config)
        self.aon_clk_freq
            .map(|freq| cycles.saturating_mul(1000).saturating_div(u64::from(freq)))
            .ok_or(ErrorCode::FAIL)
    }

    /// Function for handling interrupts related to wakeup and watchdog barks.
    pub fn handle_interrupt(&self) {
        let regs = self.registers;
        let intr = self.registers.intr_state.extract();
        if intr.is_set(INTR_STATE::WKUP_TIMER_EXPIRED) {
            // Wake up timer has expired, sw must ack and clear
            regs.wkup_cause.set(0x00);
            // To avoid re-triggers
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

    #[cfg(feature = "test_aon_timer")]
    pub fn test(
        &self,
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
            }
            1 => {
                // runner.assert_function("We woke up after sleep!", || {
                //     self.get_owner_rram_data(10) == 0x5A
                // });
            }
            _ => {}
        }
        if test_runner.is_test_failed {
            test_runner
                .write_str("aon_timer pre-kernel self-test FAILED\r\n")
                .unwrap();
        } else {
            test_runner
                .write_str("aon_timer pre-kernel self-test PASSED\r\n")
                .unwrap();
        }
        test_runner.is_test_failed
    }
}

#[cfg(feature = "test_aon_timer")]
pub mod tests {
    use super::AonTimer;
    use core::cell::Cell;
    use kernel::debug;
    use kernel::hil::time::Alarm;
    use kernel::hil::time::AlarmClient;
    use kernel::hil::time::ConvertTicks;

    pub struct Tests<'a, A: Alarm<'a>> {
        aon_timer: &'a AonTimer<'a>,
        alarm: &'a A,
        cycles: Cell<u32>,
    }

    static mut WAKEUP_CALLED: bool = false;
    static mut BARK_CALLED: bool = false;

    fn wakeup_callback() {
        unsafe {
            WAKEUP_CALLED = true;
        }
    }

    fn bark_callback() {
        unsafe {
            BARK_CALLED = true;
        }
        debug!("Wdog bark!!!");
    }

    impl<'a, A: Alarm<'a>> Tests<'a, A> {
        pub fn new(aon_timer: &'a AonTimer<'a>, alarm: &'a A) -> Self {
            Self {
                aon_timer,
                alarm,
                cycles: Cell::new(0),
            }
        }

        pub fn start_alarm(&self, ms: u32) {
            self.alarm
                .set_alarm(self.alarm.now(), self.alarm.ticks_from_ms(ms));
        }

        pub fn cyclic_tests(&self) {
            match self.cycles.get() {
                0 => {
                    debug!("Set bark interval at 99ms and see that we get the bark before 100ms ");
                    self.aon_timer
                        .register_watchdog_bark_callback(Some(&self::bark_callback));
                    unsafe {
                        BARK_CALLED = false;
                    }
                    self.aon_timer.reset_wdog();
                    let _ = self.aon_timer.set_wdog_bark_thresh_ms(99);
                    // Bite threshold must be set here when not using test ROM,
                    // otherwise the chip will reset when the count starts.
                    let _ = self.aon_timer.set_wdog_bite_thresh_ms(1000);
                    self.aon_timer.wdog_start_count(true);
                    self.start_alarm(100);
                }
                1 => {
                    unsafe {
                        assert!(BARK_CALLED == true);
                    }
                    debug!("Set bark interval at 110ms and see that we do NOT get the bark before 100ms ");
                    self.aon_timer.reset_wdog();
                    let _ = self.aon_timer.set_wdog_bark_thresh_ms(110);
                    // Bite threshold must be set here when not using test ROM,
                    // otherwise the chip will reset when the count starts.
                    let _ = self.aon_timer.set_wdog_bite_thresh_ms(1000);
                    unsafe {
                        BARK_CALLED = false;
                    }
                    self.start_alarm(100);
                }
                2 => {
                    unsafe {
                        assert!(BARK_CALLED == false);
                    }
                    debug!("Cleanup wdog bark settings ");
                    self.aon_timer.reset_wdog();
                    self.aon_timer.register_watchdog_bark_callback(None);
                    let _ = self.aon_timer.set_wdog_bark_thresh_ms(500);
                    debug!(
                        "Set wakeup interval at 90 ms and see that we get the wakeup before 100ms "
                    );
                    assert!(self.aon_timer.wakeup_enable_after_ms(90) == Ok(()));
                    self.aon_timer
                        .register_wakeup_callback(Some(&wakeup_callback));
                    unsafe {
                        WAKEUP_CALLED = false;
                    }
                    self.start_alarm(100);
                }
                3 => {
                    unsafe {
                        assert!(WAKEUP_CALLED == true);
                    }
                    debug!(
                        "Set wakeup interval at 110 ms and see that we do NOT get the wakeup before 100ms "
                    );
                    assert!(self.aon_timer.wakeup_enable_after_ms(110) == Ok(()));
                    self.aon_timer.reset_wkup();
                    unsafe {
                        WAKEUP_CALLED = false;
                    }
                    self.start_alarm(100);
                }
                4 => {
                    unsafe {
                        assert!(WAKEUP_CALLED == false);
                    }
                    debug!("Cleanup wakeup callback.");
                    self.aon_timer.reset_wkup();
                    self.aon_timer.register_wakeup_callback(None);
                    self.aon_timer.wakeup_disable();
                    self.aon_timer.reset_wkup();
                    debug!("aon_timer tests passed OK!");
                }
                _ => {}
            }
            self.cycles.set(self.cycles.get() + 1);
        }
    }

    impl<'a, A: Alarm<'a>> AlarmClient for Tests<'a, A> {
        fn alarm(&self) {
            self.cyclic_tests();
        }
    }
}

#[cfg(feature = "test_aon_timer")]
impl<'a> platform::watchdog::WatchDog for AonTimer<'a> {
    /// Blank implementation for tests to that the kernel interactions do not interfere with the tests.
    fn setup(&self) {}

    fn tickle(&self) {}

    fn suspend(&self) {}

    fn resume(&self) {}
}

#[cfg(not(feature = "test_aon_timer"))]
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
        let _ = self.set_wdog_bark_thresh_ms(500);
        let _ = self.set_wdog_bite_thresh_ms(1000);

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
