// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! GPIO instantiation.

use core::ops::{Index, IndexMut};

use kernel::utilities::StaticRef;
pub use lowrisc::gpio::{GpioBitfield, GpioPin};
use lowrisc::registers::gpio_regs::{GpioRegisters, INTR};

use crate::pinmux::PadConfig;
use crate::pinmux_config::EarlGreyPinmuxConfig;
use crate::registers::top_earlgrey::GPIO_BASE_ADDR;
use crate::registers::top_earlgrey::{
    MuxedPads, PinmuxInsel, PinmuxOutsel, PinmuxPeripheralIn, PINMUX_MIO_PERIPH_INSEL_IDX_OFFSET,
    PINMUX_PERIPH_OUTSEL_IDX_OFFSET,
};

pub const GPIO_BASE: StaticRef<GpioRegisters> =
    unsafe { StaticRef::new(GPIO_BASE_ADDR as *const GpioRegisters) };

pub struct Port<'a> {
    pins: [GpioPin<'a, PadConfig>; 32],
}

/// Wrapper type to get around the orphan rule.
pub struct Bitfield(GpioBitfield);

impl From<Bitfield> for PinmuxPeripheralIn {
    fn from(pin: Bitfield) -> PinmuxPeripheralIn {
        // We used fact that first 0-31 values are directly maped to GPIO
        Self::try_from(pin.0.shift as u32).unwrap()
    }
}

impl From<Bitfield> for PinmuxOutsel {
    fn from(pin: Bitfield) -> Self {
        // We skip first 3 constans to convert value to output selector
        match Self::try_from(pin.0.shift as u32 + PINMUX_PERIPH_OUTSEL_IDX_OFFSET as u32) {
            Ok(outsel) => outsel,
            Err(_) => PinmuxOutsel::ConstantHighZ,
        }
    }
}

// This function use extract GPIO mapping from initial pinmux configurations
pub fn gpio_pad_config<Layout: EarlGreyPinmuxConfig>(pin: GpioBitfield) -> PadConfig {
    let inp: PinmuxPeripheralIn = PinmuxPeripheralIn::from(Bitfield(pin));
    match Layout::INPUT[inp as usize] {
        // Current implementation don't support Output only GPIO
        PinmuxInsel::ConstantZero | PinmuxInsel::ConstantOne => PadConfig::Unconnected,
        input_selector => {
            if let Ok(pad) = MuxedPads::try_from(
                input_selector as u32 - PINMUX_MIO_PERIPH_INSEL_IDX_OFFSET as u32,
            ) {
                let out: PinmuxOutsel = Layout::OUTPUT[pad as usize];
                // Checking for bi-directional I/O
                if out == PinmuxOutsel::from(Bitfield(pin)) {
                    PadConfig::InOut(pad, inp, out)
                } else {
                    PadConfig::Input(pad, inp)
                }
            } else {
                // Upper match checked for unconnected pad so in this
                // place we probably have some invalid value in INPUT array.
                PadConfig::Unconnected
            }
        }
    }
}

// Configuring first all GPIO based on board layout
impl<'a> Port<'a> {
    pub fn new<Layout: EarlGreyPinmuxConfig>() -> Self {
        Self {
            // Intentionally prevent splitting GpioPin to multiple line
            #[rustfmt::skip]
            pins: [
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_0), 0),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_1), 1),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_2), 2),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_3), 3),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_4), 4),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_5), 5),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_6), 6),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_7), 7),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_8), 8),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_9), 9),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_10), 10),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_11), 11),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_12), 12),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_13), 13),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_14), 14),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_15), 15),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_16), 16),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_17), 17),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_18), 18),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_19), 19),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_20), 20),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_21), 21),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_22), 22),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_23), 23),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_24), 24),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_25), 25),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_26), 26),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_27), 27),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_28), 28),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_29), 29),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_30), 30),
                GpioPin::new(GPIO_BASE, gpio_pad_config::<Layout>(INTR::GPIO_31), 31),
            ],
        }
    }
}

impl<'a> Index<usize> for Port<'a> {
    type Output = GpioPin<'a, PadConfig>;

    fn index(&self, index: usize) -> &GpioPin<'a, PadConfig> {
        &self.pins[index]
    }
}

impl<'a> IndexMut<usize> for Port<'a> {
    fn index_mut(&mut self, index: usize) -> &mut GpioPin<'a, PadConfig> {
        &mut self.pins[index]
    }
}
