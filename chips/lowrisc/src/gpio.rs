// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! General Purpose Input/Output driver.

use crate::registers::gpio_regs::{
    GpioRegisters, DATA_IN, DIRECT_OE, DIRECT_OUT, INTR, MASKED_OE_LOWER, MASKED_OE_UPPER,
    MASKED_OUT_LOWER, MASKED_OUT_UPPER,
};
use kernel::hil::gpio;
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::registers::{Field, ReadWrite};
use kernel::utilities::StaticRef;

pub type GpioBitfield = Field<u32, INTR::Register>;

#[derive(Copy, Clone)]
pub enum GpioInterrupt {
    /// raised if any of GPIO pin detects configured interrupt mode
    Gpio,
}

pub struct GpioPin<'a, PAD> {
    gpio_registers: StaticRef<GpioRegisters>,
    padctl: PAD,
    pin: u8,
    client: OptionalCell<&'a dyn gpio::Client>,
}

impl<'a, PAD> GpioPin<'a, PAD> {
    pub const fn new(
        gpio_base: StaticRef<GpioRegisters>,
        padctl: PAD,
        pin: u8,
    ) -> GpioPin<'a, PAD> {
        GpioPin {
            gpio_registers: gpio_base,
            padctl,
            pin,
            client: OptionalCell::empty(),
        }
    }

    #[inline(always)]
    fn oe_half_set(
        val: bool,
        field: Field<u32, INTR::Register>,
        lower: &ReadWrite<u32, MASKED_OE_LOWER::Register>,
        upper: &ReadWrite<u32, MASKED_OE_UPPER::Register>,
    ) {
        let shift = field.shift;
        let bit = u32::from(val);
        if shift < 16 {
            lower.write(
                MASKED_OE_LOWER::DATA.val(bit << shift) + MASKED_OE_LOWER::MASK.val(1u32 << shift),
            );
        } else {
            let upper_shift = shift - 16;
            upper.write(
                MASKED_OE_UPPER::DATA.val(bit << upper_shift)
                    + MASKED_OE_UPPER::MASK.val(1u32 << upper_shift),
            );
        }
    }

    #[inline(always)]
    fn out_half_set(
        val: bool,
        field: Field<u32, INTR::Register>,
        lower: &ReadWrite<u32, MASKED_OUT_LOWER::Register>,
        upper: &ReadWrite<u32, MASKED_OUT_UPPER::Register>,
    ) {
        let shift = field.shift;
        let bit = u32::from(val);
        if shift < 16 {
            lower.write(
                MASKED_OUT_LOWER::DATA.val(bit << shift)
                    + MASKED_OUT_LOWER::MASK.val(1u32 << shift),
            );
        } else {
            let upper_shift = shift - 16;
            upper.write(
                MASKED_OUT_UPPER::DATA.val(bit << upper_shift)
                    + MASKED_OUT_UPPER::MASK.val(1u32 << upper_shift),
            );
        }
    }

    pub fn handle_interrupt(&self, interrupt: GpioInterrupt) {
        match interrupt {
            GpioInterrupt::Gpio => {
                let pin = intr_pin(self.pin);

                if self.gpio_registers.intr_state.is_set(pin) {
                    self.gpio_registers.intr_state.modify(pin.val(1));
                    self.client.map(|client| {
                        client.fired();
                    });
                }
            }
        }
    }
}

impl<PAD: gpio::Configure> gpio::Configure for GpioPin<'_, PAD> {
    fn configuration(&self) -> gpio::Configuration {
        match (
            self.padctl.configuration(),
            self.gpio_registers
                .direct_oe
                .is_set(direct_oe_pin(self.pin)),
        ) {
            (gpio::Configuration::InputOutput, true) => gpio::Configuration::InputOutput,
            (gpio::Configuration::InputOutput, false) => gpio::Configuration::Input,
            (gpio::Configuration::Input, false) => gpio::Configuration::Input,
            // This is configuration error we can't enable ouput
            // for GPIO pin connect to input only pad.
            (gpio::Configuration::Input, true) => gpio::Configuration::Function,
            // We curently dont support output only GPIO
            // OT register have only output_enable flag.
            (gpio::Configuration::Output, _) => gpio::Configuration::Function,
            (conf, _) => conf,
        }
    }

    fn set_floating_state(&self, mode: gpio::FloatingState) {
        self.padctl.set_floating_state(mode);
    }

    fn floating_state(&self) -> gpio::FloatingState {
        self.padctl.floating_state()
    }

    fn deactivate_to_low_power(&self) {
        self.disable_input();
        self.disable_output();
        self.padctl.deactivate_to_low_power();
    }

    fn make_output(&self) -> gpio::Configuration {
        // Re-connect in case we make output after switching from LowPower state.
        if let gpio::Configuration::InputOutput = self.padctl.make_output() {
            Self::oe_half_set(
                true,
                intr_pin(self.pin),
                &self.gpio_registers.masked_oe_lower,
                &self.gpio_registers.masked_oe_upper,
            );
        }
        self.configuration()
    }

    fn disable_output(&self) -> gpio::Configuration {
        Self::oe_half_set(
            false,
            intr_pin(self.pin),
            &self.gpio_registers.masked_oe_lower,
            &self.gpio_registers.masked_oe_upper,
        );
        self.configuration()
    }

    fn make_input(&self) -> gpio::Configuration {
        // Re-connect in case we make input after switching from LowPower state.
        self.padctl.make_input();
        self.configuration()
    }

    fn disable_input(&self) -> gpio::Configuration {
        self.configuration()
    }
}

impl<PAD> gpio::Input for GpioPin<'_, PAD> {
    fn read(&self) -> bool {
        self.gpio_registers.data_in.read(DATA_IN::DATA_IN) & (1 << self.pin) != 0
    }
}

impl<PAD> gpio::Output for GpioPin<'_, PAD> {
    fn toggle(&self) -> bool {
        let new_state =
            self.gpio_registers.direct_out.read(DIRECT_OUT::DIRECT_OUT) & (1 << self.pin) == 0;

        Self::out_half_set(
            new_state,
            intr_pin(self.pin),
            &self.gpio_registers.masked_out_lower,
            &self.gpio_registers.masked_out_upper,
        );
        new_state
    }

    fn set(&self) {
        Self::out_half_set(
            true,
            intr_pin(self.pin),
            &self.gpio_registers.masked_out_lower,
            &self.gpio_registers.masked_out_upper,
        );
    }

    fn clear(&self) {
        Self::out_half_set(
            false,
            intr_pin(self.pin),
            &self.gpio_registers.masked_out_lower,
            &self.gpio_registers.masked_out_upper,
        );
    }
}

impl<'a, PAD> gpio::Interrupt<'a> for GpioPin<'a, PAD> {
    fn set_client(&self, client: &'a dyn gpio::Client) {
        self.client.set(client);
    }

    fn enable_interrupts(&self, mode: gpio::InterruptEdge) {
        let pin = intr_pin(self.pin);

        match mode {
            gpio::InterruptEdge::RisingEdge => {
                self.gpio_registers.intr_ctrl_en_rising.modify(pin.val(1));
                self.gpio_registers.intr_ctrl_en_falling.modify(pin.val(0));
            }
            gpio::InterruptEdge::FallingEdge => {
                self.gpio_registers.intr_ctrl_en_rising.modify(pin.val(0));
                self.gpio_registers.intr_ctrl_en_falling.modify(pin.val(1));
            }
            gpio::InterruptEdge::EitherEdge => {
                self.gpio_registers.intr_ctrl_en_rising.modify(pin.val(1));
                self.gpio_registers.intr_ctrl_en_falling.modify(pin.val(1));
            }
        }
        self.gpio_registers.intr_state.modify(pin.val(1));
        self.gpio_registers.intr_enable.modify(pin.val(1));
    }

    fn disable_interrupts(&self) {
        let pin = intr_pin(self.pin);

        self.gpio_registers.intr_enable.modify(pin.val(0));
        // Clear any pending interrupt
        self.gpio_registers.intr_state.modify(pin.val(1));
    }

    fn is_pending(&self) -> bool {
        self.gpio_registers.intr_state.is_set(intr_pin(self.pin))
    }
}

/// Returns the GPIO register for the given pin ID.
///
/// # Panics
///
/// If the pin ID is out of bounds.
const fn intr_pin(num: u8) -> Field<u32, INTR::Register> {
    match num {
        0 => INTR::GPIO_0,
        1 => INTR::GPIO_1,
        2 => INTR::GPIO_2,
        3 => INTR::GPIO_3,
        4 => INTR::GPIO_4,
        5 => INTR::GPIO_5,
        6 => INTR::GPIO_6,
        7 => INTR::GPIO_7,
        8 => INTR::GPIO_8,
        9 => INTR::GPIO_9,
        10 => INTR::GPIO_10,
        11 => INTR::GPIO_11,
        12 => INTR::GPIO_12,
        13 => INTR::GPIO_13,
        14 => INTR::GPIO_14,
        15 => INTR::GPIO_15,
        16 => INTR::GPIO_16,
        17 => INTR::GPIO_17,
        18 => INTR::GPIO_18,
        19 => INTR::GPIO_19,
        20 => INTR::GPIO_20,
        21 => INTR::GPIO_21,
        22 => INTR::GPIO_22,
        23 => INTR::GPIO_23,
        24 => INTR::GPIO_24,
        25 => INTR::GPIO_25,
        26 => INTR::GPIO_26,
        27 => INTR::GPIO_27,
        28 => INTR::GPIO_28,
        29 => INTR::GPIO_29,
        30 => INTR::GPIO_30,
        31 => INTR::GPIO_31,
        _ => panic!("GPIO pin ID out of bounds"),
    }
}

/// Returns the direct OE register for the given pin ID.
///
/// # Panics
///
/// If the pin ID is out of bounds.
const fn direct_oe_pin(num: u8) -> Field<u32, DIRECT_OE::Register> {
    match num {
        0 => DIRECT_OE::DIRECT_OE_0,
        1 => DIRECT_OE::DIRECT_OE_1,
        2 => DIRECT_OE::DIRECT_OE_2,
        3 => DIRECT_OE::DIRECT_OE_3,
        4 => DIRECT_OE::DIRECT_OE_4,
        5 => DIRECT_OE::DIRECT_OE_5,
        6 => DIRECT_OE::DIRECT_OE_6,
        7 => DIRECT_OE::DIRECT_OE_7,
        8 => DIRECT_OE::DIRECT_OE_8,
        9 => DIRECT_OE::DIRECT_OE_9,
        10 => DIRECT_OE::DIRECT_OE_10,
        11 => DIRECT_OE::DIRECT_OE_11,
        12 => DIRECT_OE::DIRECT_OE_12,
        13 => DIRECT_OE::DIRECT_OE_13,
        14 => DIRECT_OE::DIRECT_OE_14,
        15 => DIRECT_OE::DIRECT_OE_15,
        16 => DIRECT_OE::DIRECT_OE_16,
        17 => DIRECT_OE::DIRECT_OE_17,
        18 => DIRECT_OE::DIRECT_OE_18,
        19 => DIRECT_OE::DIRECT_OE_19,
        20 => DIRECT_OE::DIRECT_OE_20,
        21 => DIRECT_OE::DIRECT_OE_21,
        22 => DIRECT_OE::DIRECT_OE_22,
        23 => DIRECT_OE::DIRECT_OE_23,
        24 => DIRECT_OE::DIRECT_OE_24,
        25 => DIRECT_OE::DIRECT_OE_25,
        26 => DIRECT_OE::DIRECT_OE_26,
        27 => DIRECT_OE::DIRECT_OE_27,
        28 => DIRECT_OE::DIRECT_OE_28,
        29 => DIRECT_OE::DIRECT_OE_29,
        30 => DIRECT_OE::DIRECT_OE_30,
        31 => DIRECT_OE::DIRECT_OE_31,
        _ => panic!("GPIO pin ID out of bounds"),
    }
}
