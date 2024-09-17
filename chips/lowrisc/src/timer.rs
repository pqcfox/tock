// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Timer driver.
use crate::registers::rv_timer_regs::{
    RvTimerRegisters, CFG0, COMPARE_LOWER0_0, COMPARE_UPPER0_0, CTRL, INTR_ENABLE0, INTR_STATE0,
    TIMER_V_LOWER0, TIMER_V_UPPER0,
};
use kernel::hil::time::{self, Ticks64};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::{register_bitfields, register_structs, ReadWrite, WriteOnly};
use kernel::utilities::StaticRef;
use kernel::{debug, ErrorCode};
use rv32i::machine_timer::MachineTimer;

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

pub enum SetClkResult {
    SetPrecise,
    SetImprecise,
    Error,
}

impl<'a> RvTimer<'a> {
    pub fn new(register_base: usize, clock_frequency: u32) -> Self {
        let timer_base = unsafe { &(*(register_base as *const RvTimerRegisters)) };

        Self {
            registers: unsafe { StaticRef::new(register_base as *const RvTimerRegisters) },
            peripherial_clock_frequency: clock_frequency,
            alarm_client: OptionalCell::empty(),
            overflow_client: OptionalCell::empty(),
            mtimer: unsafe {
                MachineTimer::new(
                    &*(&timer_base.compare_lower0_0
                        as *const ReadWrite<u32, COMPARE_LOWER0_0::Register>
                        as *const ReadWrite<u32>),
                    &*(&timer_base.compare_upper0_0
                        as *const ReadWrite<u32, COMPARE_UPPER0_0::Register>
                        as *const ReadWrite<u32>),
                    &*(&timer_base.timer_v_lower0 as *const ReadWrite<u32, TIMER_V_LOWER0::Register>
                        as *const ReadWrite<u32>),
                    &*(&timer_base.timer_v_upper0 as *const ReadWrite<u32, TIMER_V_UPPER0::Register>
                        as *const ReadWrite<u32>),
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
            (Some(x), Some(y)) => (x - 1, SetClkResult::SetImprecise),
            _ => (0, SetClkResult::Error),
        };
        if prescaler_target > 0xFFF {
            op_return = SetClkResult::Error;
        } else {
            self.registers
                .cfg0
                .write(CFG0::PRESCALE.val(prescaler_target as u32) + CFG0::STEP.val(1u32));
        }

        return op_return;
    }

    pub fn set_now_tick(&self, ticks: u64) {
        self.registers.timer_v_lower0.set(ticks as u32);
        self.registers.timer_v_upper0.set((ticks >> 16) as u32);
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

    pub fn service_interrupt(&self) {
        self.isr_disable();
        self.registers.intr_state0[0].write(INTR_STATE0::IS_0::SET);
        self.alarm_client.map(|client| {
            client.alarm();
        });
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
        Err(ErrorCode::BUSY)
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
        self.registers.intr_enable0[0].write(INTR_ENABLE0::IE_0::SET);

        self.mtimer.set_alarm(reference, dt)
    }

    fn get_alarm(&self) -> Self::Ticks {
        self.mtimer.get_alarm()
    }

    fn disarm(&self) -> Result<(), ErrorCode> {
        self.registers.intr_enable0[0].write(INTR_ENABLE0::IE_0::CLEAR);

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
            debug!("Cyclic alarm!");
            debug!("Now time is: {}", self.alarm.ticks_to_ms(self.alarm.now()));

            self.start_alarm(1000);
            self.cycles.set(self.cycles.get() + 1);
        }
    }

    impl<'a, A: Alarm<'a>> AlarmClient for Tests<'a, A> {
        fn alarm(&self) {
            self.cyclic_tests();
        }
    }
}
