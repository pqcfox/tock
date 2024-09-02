// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Timer driver.
use crate::registers::rv_timer_regs::{RvTimerRegisters, CFG0, CTRL, INTR_ENABLE0, INTR_STATE0};
use kernel::hil::time::{self, Ticks64};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::{register_bitfields, register_structs, ReadWrite, WriteOnly};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;
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

register_structs! {
    pub TimerRegisters {
        (0x000 => _reserved),
        (0x110 => value_low: ReadWrite<u32>),
        (0x114 => value_high: ReadWrite<u32>),
        (0x118 => compare_low: ReadWrite<u32>),
        (0x11C => compare_high: ReadWrite<u32>),
        (0x120 => @END),
    }
}
impl<'a> RvTimer<'a> {
    pub fn new(register_base: usize, clock_frequency: u32) -> Self {
        let timer_base = unsafe { &(*(register_base as *const TimerRegisters)) };

        Self {
            registers: unsafe { StaticRef::new(register_base as *const RvTimerRegisters) },
            peripherial_clock_frequency: clock_frequency,
            alarm_client: OptionalCell::empty(),
            overflow_client: OptionalCell::empty(),
            mtimer: MachineTimer::new(
                &timer_base.compare_low,
                &timer_base.compare_high,
                &timer_base.value_low,
                &timer_base.value_high,
            ),
        }
    }

    pub fn setup(&self) {
        let prescale: u16 = ((self.peripherial_clock_frequency / 10_000) - 1) as u16; // 10Khz

        let regs = self.registers;
        // Set proper prescaler and the like
        regs.cfg0
            .write(CFG0::PRESCALE.val(prescale as u32) + CFG0::STEP.val(1u32));
        regs.compare_upper0_0.set(0);
        regs.timer_v_lower0.set(0xFFFF_0000);
        regs.intr_enable0[0].write(INTR_ENABLE0::IE_0::CLEAR);
        regs.ctrl[0].write(CTRL::ACTIVE_0::SET);
    }

    pub fn service_interrupt(&self) {
        let regs = self.registers;
        regs.intr_enable0[0].write(INTR_ENABLE0::IE_0::CLEAR);
        regs.intr_state0[0].write(INTR_STATE0::IS_0::SET);
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
