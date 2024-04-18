// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::registers::pattgen_regs::{PattgenRegisters, CTRL, SIZE};

use kernel::hil::pattgen::PattGen as PattGenHIL;
use kernel::utilities::registers::interfaces::{ReadWriteable, Writeable};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

use core::num::NonZeroUsize;

pub struct PattGen {
    registers: StaticRef<PattgenRegisters>,
}

impl PattGen {
    pub fn new(registers: StaticRef<PattgenRegisters>) -> Self {
        Self { registers }
    }

    fn set_predivider0(&self, predivider: usize) {
        // CAST: usize == u32 on RV32I
        self.registers.prediv_ch0.set(predivider as u32);
    }

    fn set_predivider1(&self, predivider: usize) {
        // CAST: usize == u32 on RV32I
        self.registers.prediv_ch1.set(predivider as u32);
    }

    fn set_pattern0(&self, pattern: &[u32; 2]) {
        // PANIC: data_ch0.len() == 2
        self.registers.data_ch0[0].set(pattern[0]);
        self.registers.data_ch0[1].set(pattern[1]);
    }

    fn set_pattern1(&self, pattern: &[u32; 2]) {
        // PANIC: data_ch0.len() == 2
        self.registers.data_ch1[0].set(pattern[0]);
        self.registers.data_ch1[1].set(pattern[1]);
    }

    fn set_pattern0_length_and_repetition(
        &self,
        pattern_length: PatternLength,
        repetion_count: PatternRepetitionCount,
    ) {
        // CAST: usize == u32 on RV32I
        self.registers.size.modify(
            SIZE::LEN_CH0.val(pattern_length.as_u32() - 1)
                + SIZE::REPS_CH0.val(repetion_count.as_u32() - 1),
        );
    }

    fn set_pattern1_length_and_repetition(
        &self,
        pattern_length: PatternLength,
        repetion_count: PatternRepetitionCount,
    ) {
        // CAST: usize == u32 on RV32I
        self.registers.size.modify(
            SIZE::LEN_CH1.val(pattern_length.as_u32() - 1)
                + SIZE::REPS_CH1.val(repetion_count.as_u32() - 1),
        );
    }

    fn configure_channel0(
        &self,
        pattern: &[u32; 2],
        pattern_length: PatternLength,
        repetion_count: PatternRepetitionCount,
        predivider: usize,
    ) {
        self.set_predivider0(predivider);
        self.set_pattern0(pattern);
        self.set_pattern0_length_and_repetition(pattern_length, repetion_count);
    }

    fn configure_channel1(
        &self,
        pattern: &[u32; 2],
        pattern_length: PatternLength,
        repetion_count: PatternRepetitionCount,
        predivider: usize,
    ) {
        self.set_predivider1(predivider);
        self.set_pattern1(pattern);
        self.set_pattern1_length_and_repetition(pattern_length, repetion_count);
    }
}

pub enum Channel {
    Channel0,
    Channel1,
}

impl TryFrom<usize> for Channel {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Channel::Channel0),
            1 => Ok(Channel::Channel1),
            _ => Err(()),
        }
    }
}

pub struct PatternLength(usize);

impl PatternLength {
    const fn as_u32(self) -> u32 {
        // CAST: usize == u32 on RV32I
        self.0 as u32
    }
}

impl TryFrom<NonZeroUsize> for PatternLength {
    type Error = ();

    fn try_from(value: NonZeroUsize) -> Result<Self, Self::Error> {
        let inner_value = value.get();
        if inner_value > 64 {
            Err(())
        } else {
            Ok(Self(inner_value))
        }
    }
}

pub struct PatternRepetitionCount(usize);

impl PatternRepetitionCount {
    const fn as_u32(self) -> u32 {
        // CAST: usize == u32 on RV32I
        self.0 as u32
    }
}

impl TryFrom<NonZeroUsize> for PatternRepetitionCount {
    type Error = ();

    fn try_from(value: NonZeroUsize) -> Result<Self, Self::Error> {
        let inner_value = value.get();
        if inner_value > 1024 {
            Err(())
        } else {
            Ok(Self(inner_value))
        }
    }
}

impl PattGenHIL for PattGen {
    type Channel = Channel;
    type PatternLength = PatternLength;
    type PatternRepetitionCount = PatternRepetitionCount;

    fn start(
        &self,
        pattern: &[u32; 2],
        pattern_length: PatternLength,
        pattern_repetition_count: PatternRepetitionCount,
        predivider: usize,
        channel: Channel,
    ) -> Result<(), ErrorCode> {
        // Configure the channel accordingly
        let enable_field_value = match channel {
            Channel::Channel0 => {
                self.configure_channel0(
                    pattern,
                    pattern_length,
                    pattern_repetition_count,
                    predivider,
                );
                CTRL::ENABLE_CH0::Set
            },
            Channel::Channel1 => {
                self.configure_channel1(
                    pattern,
                    pattern_length,
                    pattern_repetition_count,
                    predivider,
                );
                CTRL::ENABLE_CH1::Set
            },
        };

        // Start pattern generation
        self.registers.ctrl.modify(enable_field_value);

        Ok(())
    }

    fn stop(&self, channel: Channel) -> Result<(), ErrorCode> {
        let disable_field_value = match channel {
            Channel::Channel0 => CTRL::ENABLE_CH0::Clear,
            Channel::Channel1 => CTRL::ENABLE_CH1::Clear,
        };

        // Disable pattern generation
        self.registers.ctrl.modify(disable_field_value);

        Ok(())
    }
}
