// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Timer driver.
use crate::registers::rv_timer_regs::{
    RvTimerRegisters, CFG0, COMPARE_LOWER0_0, COMPARE_UPPER0_0, CTRL, INTR_ENABLE0, INTR_STATE0,
    TIMER_V_LOWER0, TIMER_V_UPPER0,
};
use core::ptr::from_ref;

use kernel::hil::time::{self, Ticks64};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::StaticRef;
use kernel::ErrorCode;
use rv32i::machine_timer::MachineTimer;

#[cfg(feature = "test_rv_timer")]
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

/// 10KHz `Frequency`
#[derive(Debug)]
pub struct Freq10KHz;
impl time::Frequency for Freq10KHz {
    fn frequency() -> u32 {
        10_000
    }
}
pub struct RvTimer<'a> {
    registers: StaticRef<RvTimerRegisters>,
    peripherial_clock_frequency: u32,
    alarm_client: OptionalCell<&'a dyn time::AlarmClient>,
    overflow_client: OptionalCell<&'a dyn time::OverflowClient>,
    mtimer: MachineTimer<'a>,
}

#[derive(Clone, Copy)]
pub enum RvTimerInterrupt {
    /// Interrupt status for timer
    ExpiredHart0Timer0,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SetClkResult {
    SetPrecise,
    SetImprecise,
    Error,
}

impl<'a> RvTimer<'a> {
    pub fn new(register_base: StaticRef<RvTimerRegisters>, clock_frequency: u32) -> Self {
        Self {
            registers: register_base,
            peripherial_clock_frequency: clock_frequency,
            alarm_client: OptionalCell::empty(),
            overflow_client: OptionalCell::empty(),
            mtimer: unsafe {
                MachineTimer::new(
                    &*(from_ref::<ReadWrite<u32, COMPARE_LOWER0_0::Register>>(
                        &register_base.compare_lower0_0,
                    ) as *const ReadWrite<u32>),
                    &*(from_ref::<ReadWrite<u32, COMPARE_UPPER0_0::Register>>(
                        &register_base.compare_upper0_0,
                    ) as *const ReadWrite<u32>),
                    &*(from_ref::<ReadWrite<u32, TIMER_V_LOWER0::Register>>(
                        &register_base.timer_v_lower0,
                    ) as *const ReadWrite<u32>),
                    &*(from_ref::<ReadWrite<u32, TIMER_V_UPPER0::Register>>(
                        &register_base.timer_v_upper0,
                    ) as *const ReadWrite<u32>),
                )
            },
        }
    }

    pub fn set_clock_frequency(&self, target_freq: u32) -> SetClkResult {
        let (prescaler_target, mut op_return) = match (
            self.peripherial_clock_frequency.checked_div(target_freq),
            self.peripherial_clock_frequency.checked_rem(target_freq),
        ) {
            (Some(x), Some(0)) => (x - 1, SetClkResult::SetPrecise),
            (Some(x), Some(_)) => (x - 1, SetClkResult::SetImprecise),
            _ => (0, SetClkResult::Error),
        };
        if prescaler_target > 0xFFF {
            op_return = SetClkResult::Error;
        } else {
            self.registers
                .cfg0
                .write(CFG0::PRESCALE.val(prescaler_target) + CFG0::STEP.val(1u32));
        }

        op_return
    }

    pub fn set_now_tick(&self, ticks: u64) {
        self.registers
            .timer_v_lower0
            .set((ticks & 0xFFFF_FFFF) as u32);
        self.registers.timer_v_upper0.set((ticks >> 32) as u32);
    }

    pub fn disable(&self) {
        self.registers.ctrl[0].write(CTRL::ACTIVE_0::CLEAR);
    }

    pub fn enable(&self) {
        self.registers.ctrl[0].write(CTRL::ACTIVE_0::SET);
    }

    pub fn isr_disable(&self) {
        self.registers.intr_enable0[0].write(INTR_ENABLE0::IE_0::CLEAR);
    }

    pub fn isr_enable(&self) {
        self.registers.intr_enable0[0].write(INTR_ENABLE0::IE_0::SET);
    }

    pub fn setup(&self) {
        self.disable();
        self.set_clock_frequency(10_000);
        self.set_now_tick(0);
        self.mtimer.disable_machine_timer();
        self.isr_disable();
        self.enable();
    }

    pub fn handle_interrupt(&self, interrupt: RvTimerInterrupt) {
        match interrupt {
            RvTimerInterrupt::ExpiredHart0Timer0 => {
                self.isr_disable();
                self.registers.intr_state0[0].write(INTR_STATE0::IS_0::SET);
                self.alarm_client.map(|client| {
                    client.alarm();
                });
            }
        }
    }

    #[cfg(feature = "test_rv_timer")]
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
            .write_str("Starting rv_timer self-test \r\n")
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
                test_runner.assert_function("Test timer cfg values for 10_000 hz!", || {
                    self.set_clock_frequency(10_000) == SetClkResult::SetPrecise
                        && self.registers.cfg0.read(CFG0::PRESCALE) == 599
                        && self.registers.cfg0.read(CFG0::STEP) == 1
                });

                test_runner.assert_function("Test timer cfg values for 10_001 hz (it gets rounded and notifies us we're imprecise freq)!", || {
                    self.set_clock_frequency(10_001) == SetClkResult::SetImprecise
                        && self.registers.cfg0.read(CFG0::PRESCALE) == 598
                        && self.registers.cfg0.read(CFG0::STEP) == 1
                });

                test_runner.assert_function(
                    "Test timer cfg values for natural clock frequency!",
                    || {
                        self.set_clock_frequency(self.peripherial_clock_frequency)
                            == SetClkResult::SetPrecise
                            && self.registers.cfg0.read(CFG0::PRESCALE) == 0
                            && self.registers.cfg0.read(CFG0::STEP) == 1
                    },
                );

                test_runner.assert_function(
                    "Test timer cfg values for out-of-range clock frequency!",
                    || {
                        self.set_clock_frequency(1) == SetClkResult::Error
                            && self.registers.cfg0.read(CFG0::PRESCALE) == 0
                            && self.registers.cfg0.read(CFG0::STEP) == 1
                    },
                );

                test_runner.assert_function(
                    "Test timer cfg values for out-of-range clock frequency!",
                    || {
                        self.set_clock_frequency(self.peripherial_clock_frequency / 0xFFFF)
                            == SetClkResult::Error
                            && self.registers.cfg0.read(CFG0::PRESCALE) == 0
                            && self.registers.cfg0.read(CFG0::STEP) == 1
                    },
                );

                test_runner.assert_function("Test set_now_tick!", || {
                    self.set_now_tick(0xFFFF_FFFF_AAAA_AAAA);
                    // The lowest byte is ignored for the purposes of this test to prevent spurious
                    // failures because the clock advances by a few ticks between when the register
                    // is written and read.
                    self.registers.timer_v_lower0.get() & 0xFFFF_FF00 == 0xAAAA_AA00
                        && self.registers.timer_v_upper0.get() == 0xFFFF_FFFF
                });

                test_runner.assert_function("Test enable!", || {
                    self.enable();
                    self.registers.ctrl[0].is_set(CTRL::ACTIVE_0) == true
                });

                test_runner.assert_function("Test disable!", || {
                    self.disable();
                    self.registers.ctrl[0].is_set(CTRL::ACTIVE_0) == false
                });

                test_runner.assert_function("Test isr_enable!", || {
                    self.isr_enable();
                    self.registers.intr_enable0[0].is_set(INTR_ENABLE0::IE_0) == true
                });

                test_runner.assert_function("Test isr_disable!", || {
                    self.isr_disable();
                    self.registers.intr_enable0[0].is_set(INTR_ENABLE0::IE_0) == false
                });
            }
            _ => {}
        }

        if test_runner.is_test_failed {
            test_runner
                .write_str("rv_timer pre-kernel self-test FAILED\r\n")
                .unwrap();
        } else {
            // For `opentitan_test`'s sake, we don't print "PASSED" here because we still have the
            // post-setup tests to run.
            test_runner
                .write_str("Ending rv_timer pre-kernel self-test \r\n")
                .unwrap();
        }
        self.setup();
        test_runner.is_test_failed
    }
}

impl time::Time for RvTimer<'_> {
    type Frequency = Freq10KHz;
    type Ticks = Ticks64;

    fn now(&self) -> Ticks64 {
        self.mtimer.now()
    }
}

impl<'a> time::Counter<'a> for RvTimer<'a> {
    fn set_overflow_client(&self, client: &'a dyn time::OverflowClient) {
        self.overflow_client.set(client);
    }

    fn start(&self) -> Result<(), ErrorCode> {
        Ok(())
    }

    fn stop(&self) -> Result<(), ErrorCode> {
        // RISCV counter can't be stopped...
        Err(ErrorCode::FAIL)
    }

    fn reset(&self) -> Result<(), ErrorCode> {
        // RISCV counter can't be reset
        Err(ErrorCode::FAIL)
    }

    fn is_running(&self) -> bool {
        true
    }
}

impl<'a> time::Alarm<'a> for RvTimer<'a> {
    fn set_alarm_client(&self, client: &'a dyn time::AlarmClient) {
        self.alarm_client.set(client);
    }

    fn set_alarm(&self, reference: Self::Ticks, dt: Self::Ticks) {
        self.isr_enable();
        self.mtimer.set_alarm(reference, dt);
    }

    fn get_alarm(&self) -> Self::Ticks {
        self.mtimer.get_alarm()
    }

    fn disarm(&self) -> Result<(), ErrorCode> {
        self.isr_disable();
        self.mtimer.disarm()
    }

    fn is_armed(&self) -> bool {
        self.registers.intr_enable0[0].is_set(INTR_ENABLE0::IE_0)
    }

    fn minimum_dt(&self) -> Self::Ticks {
        self.mtimer.minimum_dt()
    }
}

#[cfg(feature = "test_rv_timer")]
pub mod tests {
    use core::cell::Cell;
    use kernel::debug;
    use kernel::hil::time::Alarm;
    use kernel::hil::time::AlarmClient;
    use kernel::hil::time::ConvertTicks;
    pub struct Tests<'a, A: Alarm<'a>> {
        alarm: &'a A,
        cycles: Cell<u32>,
    }

    impl<'a, A: Alarm<'a>> Tests<'a, A> {
        pub fn new(alarm: &'a A) -> Self {
            Self {
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
                    self.start_alarm(1000);
                    let ms_time = self.alarm.ticks_to_ms(self.alarm.now());
                    assert!(ms_time == 1000);
                    debug!(
                        "Now time is: {} ms and next alarm will be in 1000ms",
                        ms_time
                    );
                }
                1 => {
                    self.start_alarm(2000);
                    let ms_time = self.alarm.ticks_to_ms(self.alarm.now());
                    assert!(ms_time == 2000);
                    debug!("Now time is: {} and next alarm will be in 2000ms", ms_time);
                }
                2 => {
                    self.start_alarm(200);
                    let ms_time = self.alarm.ticks_to_ms(self.alarm.now());
                    assert!(ms_time == 4000);
                    debug!("Now time is: {} and next alarm will be in 200ms", ms_time);
                }
                3 => {
                    self.start_alarm(100);
                    let ms_time = self.alarm.ticks_to_ms(self.alarm.now());
                    assert!(ms_time == 4200);
                    debug!("Now time is: {} and next alarm will be in 100ms", ms_time);
                }
                4 => {
                    let ms_time = self.alarm.ticks_to_ms(self.alarm.now());
                    assert!(ms_time == 4300);
                    debug!(
                        "Now time is: {} and no next alarm will be triggered",
                        ms_time
                    );
                    debug!("rv_timer tests PASSED")
                }
                _ => panic!("We shoud have stopped alarms by now !"),
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
