// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use crate::registers::pattgen_regs::{PattgenRegisters, CTRL, INTR, SIZE};

use kernel::hil::pattgen::{PattGen as PattGenHIL, PattGenClient};
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{ReadWriteable, Writeable};
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

use core::num::NonZeroUsize;

/// Pattern generator driver
pub struct PattGen<'a> {
    registers: StaticRef<PattgenRegisters>,
    client: OptionalCell<&'a dyn PattGenClient<Channel>>,
}

impl PattGen<'_> {
    /// Pattern generator constructor
    ///
    /// # Parameters
    ///
    /// + `registers`: pattern generator registers
    ///
    /// # Return value
    ///
    /// A new instance of [PattGen]
    pub fn new(registers: StaticRef<PattgenRegisters>) -> Self {
        let pattern_generator = Self {
            registers,
            client: OptionalCell::empty(),
        };

        pattern_generator.init();

        pattern_generator
    }

    /// Initializes the driver
    fn init(&self) {
        self.enable_interrupts();
    }

    /// Enables interrupts
    fn enable_interrupts(&self) {
        self.registers
            .intr_enable
            .modify(INTR::DONE_CH0::SET + INTR::DONE_CH1::SET);
    }

    /// Sets channel 0 predivider
    ///
    /// # Parameters
    ///
    /// + `predivider`: channel 0 input clock predivider to be set
    fn set_predivider0(&self, predivider: usize) {
        // CAST: usize == u32 on RV32I
        self.registers.prediv_ch0.set(predivider as u32);
    }

    /// Sets channel 1 predivider
    ///
    /// # Parameters
    ///
    /// + `predivider`: channel 1 predivider to be set
    fn set_predivider1(&self, predivider: usize) {
        // CAST: usize == u32 on RV32I
        self.registers.prediv_ch1.set(predivider as u32);
    }

    /// Sets channel 0 pattern
    ///
    /// # Parameters
    ///
    /// + `pattern`: channel 0 pattern to be set
    fn set_pattern0(&self, pattern: &[u32; 2]) {
        // PANIC: data_ch0.len() == 2
        self.registers.data_ch0[0].set(pattern[0]);
        self.registers.data_ch0[1].set(pattern[1]);
    }

    /// Sets channel 1 pattern
    ///
    /// # Parameters
    ///
    /// + `pattern`: channel 1 pattern to be set
    fn set_pattern1(&self, pattern: &[u32; 2]) {
        // PANIC: data_ch0.len() == 2
        self.registers.data_ch1[0].set(pattern[0]);
        self.registers.data_ch1[1].set(pattern[1]);
    }

    /// Sets channel 0 pattern length and patern repetition count
    ///
    /// # Parameters
    ///
    /// + `pattern_length`: pattern length in bits
    /// + `repetion_count`: pattern repetition count
    fn set_pattern0_length_and_repetition(
        &self,
        pattern_length: PatternLength,
        repetion_count: PatternRepetitionCount,
    ) {
        // CAST: usize == u32 on RV32I
        self.registers.size.modify(
            SIZE::LEN_CH0.val(pattern_length.into_u32() - 1)
                + SIZE::REPS_CH0.val(repetion_count.into_u32() - 1),
        );
    }

    /// Sets channel 1 pattern length and patern repetition count
    ///
    /// # Parameters
    ///
    /// + `pattern_length`: pattern length in bits
    /// + `repetion_count`: pattern repetition count
    fn set_pattern1_length_and_repetition(
        &self,
        pattern_length: PatternLength,
        repetion_count: PatternRepetitionCount,
    ) {
        // CAST: usize == u32 on RV32I
        self.registers.size.modify(
            SIZE::LEN_CH1.val(pattern_length.into_u32() - 1)
                + SIZE::REPS_CH1.val(repetion_count.into_u32() - 1),
        );
    }

    /// Configures channel 0 with the given parameters
    ///
    ///
    /// # Parameters
    ///
    /// + `pattern`: pattern to be used
    /// + `pattern_length`: pattern length in bits
    /// + `repetition_count`: pattern repetition count
    /// + `predivider`: predivider for input clock
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

    /// Configures channel 1 with the given parameters
    ///
    ///
    /// # Parameters
    ///
    /// + `pattern`: pattern to be used
    /// + `pattern_length`: pattern length in bits
    /// + `repetition_count`: pattern repetition count
    /// + `predivider`: predivider for input clock
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

    /// Clears channel 0 done interrupt
    fn clear_channel0_interrupt(&self) {
        self.registers.intr_state.modify(INTR::DONE_CH0::SET);
    }

    /// Clears channel 1 done interrupt
    fn clear_channel1_interrupt(&self) {
        self.registers.intr_state.modify(INTR::DONE_CH1::SET);
    }

    /// Channel 0 done interrupt handler
    fn handle_channel0_interrupt(&self) {
        self.clear_channel0_interrupt();
        self.client
            .map(|client| client.pattgen_done(Channel::Channel0));
    }

    /// Channel 1 done interrupt handler
    fn handle_channel1_interrupt(&self) {
        self.clear_channel1_interrupt();
        self.client
            .map(|client| client.pattgen_done(Channel::Channel1));
    }

    /// Pattgen interrupt handler
    pub fn handle_interrupt(&self, pattgen_interrupt: PattgenInterrupt) {
        match pattgen_interrupt {
            PattgenInterrupt::Channel0Done => self.handle_channel0_interrupt(),
            PattgenInterrupt::Channel1Done => self.handle_channel1_interrupt(),
        }
    }
}

/// List of all pattgen interrupts
pub enum PattgenInterrupt {
    Channel0Done = 122,
    Channel1Done,
}

impl TryFrom<usize> for PattgenInterrupt {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            122 => Ok(PattgenInterrupt::Channel0Done),
            123 => Ok(PattgenInterrupt::Channel1Done),
            _ => Err(()),
        }
    }
}

/// List of all possible pattern generator channels.
#[repr(usize)]
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

impl From<Channel> for usize {
    fn from(value: Channel) -> Self {
        // CAST: Channel is marked repr(usize)
        value as usize
    }
}

/// Pattern length in bits
pub struct PatternLength(NonZeroUsize);

impl PatternLength {
    /// [PatternLength] constructor
    ///
    /// # Parameters
    ///
    /// + `value`: the value to be used as a pattern length
    ///
    /// # Return value
    ///
    /// + Ok(Self): `value` is valid (<= 64)
    /// + Err(()): ̀`value` is invalid (> 64)
    const fn new(value: NonZeroUsize) -> Result<Self, ()> {
        let inner_value = value.get();
        if inner_value > 64 {
            Err(())
        } else {
            Ok(Self(value))
        }
    }

    /// Cast [PatternLength] to u32
    ///
    /// # Return value
    ///
    /// The inner value of [PatternLength] casted to u32
    const fn into_u32(self) -> u32 {
        // CAST: usize == u32 on RV32I
        self.0.get() as u32
    }
}

impl TryFrom<NonZeroUsize> for PatternLength {
    type Error = ();

    fn try_from(value: NonZeroUsize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Pattern repetition count
pub struct PatternRepetitionCount(NonZeroUsize);

impl PatternRepetitionCount {
    /// [PatternRepetitionCount] constructor
    ///
    /// # Parameters
    ///
    /// + `value`: the value to be used as a pattern repetition count
    ///
    /// # Return value
    ///
    /// + Ok(Self): `value` is valid (<= 1024)
    /// + Err(()): ̀`value` is invalid (> 1024)
    const fn new(value: NonZeroUsize) -> Result<Self, ()> {
        let inner_value = value.get();
        if inner_value > 1024 {
            Err(())
        } else {
            Ok(Self(value))
        }
    }

    /// Cast [PatternRepetitionCount] to u32
    ///
    /// # Return value
    ///
    /// The inner value of [PatternRepetitionCount] casted to u32
    const fn into_u32(self) -> u32 {
        // CAST: usize == u32 on RV32I
        self.0.get() as u32
    }
}

impl TryFrom<NonZeroUsize> for PatternRepetitionCount {
    type Error = ();

    fn try_from(value: NonZeroUsize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl<'a> PattGenHIL<'a> for PattGen<'a> {
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
        match channel {
            Channel::Channel0 => {
                // The enable bit must be cleared after it's set
                self.registers.ctrl.modify(CTRL::ENABLE_CH0::CLEAR);
                self.configure_channel0(
                    pattern,
                    pattern_length,
                    pattern_repetition_count,
                    predivider,
                );
                self.registers.ctrl.modify(CTRL::ENABLE_CH0::SET);
            }
            Channel::Channel1 => {
                // The enable bit must be cleared after it's set
                self.registers.ctrl.modify(CTRL::ENABLE_CH1::CLEAR);
                self.configure_channel1(
                    pattern,
                    pattern_length,
                    pattern_repetition_count,
                    predivider,
                );
                self.registers.ctrl.modify(CTRL::ENABLE_CH1::SET);
            }
        };

        Ok(())
    }

    fn stop(&self, channel: Channel) -> Result<(), ErrorCode> {
        let disable_field_value = match channel {
            Channel::Channel0 => CTRL::ENABLE_CH0::CLEAR,
            Channel::Channel1 => CTRL::ENABLE_CH1::CLEAR,
        };

        // Disable pattern generation
        self.registers.ctrl.modify(disable_field_value);

        Ok(())
    }

    fn set_client(&self, client: &'a dyn PattGenClient<Channel>) {
        self.client.set(client);
    }
}

/// Tests for pattern generator
///
/// Usage
/// -----
///
/// First, enable "tests" feature for `lowrisc` dependency. Then, inside the main function of the
/// board crate, add the following lines:
///
/// ```rust,ignore
/// let pattgen_test = static_init!(
///     lowrisc::pattgen::tests::PattGenTest,
///     lowrisc::pattgen::tests::PattGenTest::new(&peripherals.pattgen),
/// );
///
/// lowrisc::pattgen::tests::run_all(pattgen_test);
/// ```
#[cfg(feature = "test_pattgen")]
pub mod tests {
    use super::*;
    use core::cell::Cell;

    /// Pattern length
    const PATTERN_LENGTH: PatternLength = match PatternLength::new(match NonZeroUsize::new(64) {
        Some(pattern_length) => pattern_length,
        None => unreachable!(),
    }) {
        Ok(pattern_length) => pattern_length,
        Err(()) => unreachable!(),
    };

    /// Pattern repetition count
    const PATTERN_REPETITION_COUNT: PatternRepetitionCount =
        match PatternRepetitionCount::new(match NonZeroUsize::new(1024) {
            Some(pattern_repetition_count) => pattern_repetition_count,
            None => unreachable!(),
        }) {
            Ok(pattern_repetition_count) => pattern_repetition_count,
            Err(()) => unreachable!(),
        };

    /// Predivider used for both channels
    const PREDIVIDER: usize = 4;

    /// Pattern generator test
    pub struct PattGenTest<'a> {
        pattgen: &'a PattGen<'a>,
        pattern_channel0: Cell<u64>,
        channel0_bit_index: Cell<u64>,
        pattern_channel1: Cell<u64>,
        channel1_bit_index: Cell<u64>,
    }

    impl<'a> PattGenTest<'a> {
        /// PattGenTest constructor
        ///
        /// # Parameters
        ///
        /// + `pattgen`: a reference to the pattern generator peripheral
        ///
        /// # Return value
        ///
        /// A new instance of [PattGenTest].
        pub fn new(pattgen: &'a PattGen<'a>) -> Self {
            Self {
                pattgen,
                pattern_channel0: Cell::new(0),
                channel0_bit_index: Cell::new(1),
                pattern_channel1: Cell::new(u64::MAX),
                channel1_bit_index: Cell::new(1u64 << 63),
            }
        }

        /// Returns the currently configured pattern for channel 0
        fn get_pattern_channel0(&self) -> [u32; 2] {
            // SAFETY: a u64 can be viewed as an array of two u32.
            unsafe { core::mem::transmute(self.pattern_channel0.get().to_le_bytes()) }
        }

        /// Returns the currently configured pattern for channel 1
        fn get_pattern_channel1(&self) -> [u32; 2] {
            // SAFETY: a u64 can be viewed as an array of two u32.
            unsafe { core::mem::transmute(self.pattern_channel1.get().to_le_bytes()) }
        }

        /// Start pattern on channel 0
        fn start_channel0(&self) {
            self.pattgen
                .start(
                    &self.get_pattern_channel0(),
                    PATTERN_LENGTH,
                    PATTERN_REPETITION_COUNT,
                    PREDIVIDER,
                    Channel::Channel0,
                )
                .expect("Failed to start pattern generator for channel 0");
        }

        /// Start pattern on channel 1
        fn start_channel1(&self) {
            self.pattgen
                .start(
                    &self.get_pattern_channel1(),
                    PATTERN_LENGTH,
                    PATTERN_REPETITION_COUNT,
                    PREDIVIDER,
                    Channel::Channel1,
                )
                .expect("Failed to start pattern generator for channel 1")
        }

        /// Generate the next pattern for channel 0
        fn next_pattern_channel0(&self) {
            let old_pattern_channel0 = match self.pattern_channel0.get() {
                u64::MAX => 0,
                old_pattern_channel0 => old_pattern_channel0,
            };

            let channel0_bit_index = self.channel0_bit_index.get();
            let new_pattern_channel0 = old_pattern_channel0 | channel0_bit_index;

            self.pattern_channel0.set(new_pattern_channel0);

            if channel0_bit_index == (1u64 << 63) {
                self.channel0_bit_index.set(1);
            } else {
                self.channel0_bit_index.set(channel0_bit_index << 1);
            }
        }

        /// Generate the next pattern for channel 1
        fn next_pattern_channel1(&self) {
            let old_pattern_channel1 = match self.pattern_channel1.get() {
                0 => u64::MAX,
                old_pattern_channel1 => old_pattern_channel1,
            };

            let channel1_bit_index = self.channel1_bit_index.get();
            let new_pattern_channel1 = old_pattern_channel1 & !channel1_bit_index;

            self.pattern_channel1.set(new_pattern_channel1);

            if channel1_bit_index == 0 {
                self.channel1_bit_index.set(1u64 << 63);
            } else {
                self.channel1_bit_index.set(channel1_bit_index >> 1);
            }
        }
    }

    impl<'a> PattGenClient<Channel> for PattGenTest<'a> {
        fn pattgen_done(&self, channel: Channel) {
            match channel {
                Channel::Channel0 => {
                    self.next_pattern_channel0();
                    self.start_channel0();
                }
                Channel::Channel1 => {
                    self.next_pattern_channel1();
                    self.start_channel1();
                }
            }
        }
    }

    /// Channel 0 makes a LED brighten, while channel 1 makes a LED fade.
    pub fn run_all<'a>(pattgen_test: &'a PattGenTest<'a>) {
        kernel::debug!("Starting Pattgen tests...");
        pattgen_test.pattgen.set_client(pattgen_test);
        pattgen_test.start_channel0();
        pattgen_test.start_channel1();
        kernel::debug!("Finished Pattgen tests... PASSED");
    }
}
