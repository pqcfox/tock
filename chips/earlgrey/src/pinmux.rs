// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Mux'ing between physical pads and GPIO or other peripherals.

use kernel::hil::gpio;
use kernel::hil::gpio::{Configuration, Configure, FloatingState};
use kernel::utilities::registers::interfaces::{Readable, Writeable};
use kernel::utilities::registers::{FieldValue, LocalRegisterCopy};
use kernel::utilities::StaticRef;

use crate::registers::pinmux_regs::{
    PinmuxRegisters, DIO_PAD_ATTR, DIO_PAD_ATTR_REGWEN, MIO_OUTSEL_REGWEN, MIO_PAD_ATTR,
    MIO_PAD_ATTR_REGWEN, MIO_PERIPH_INSEL_REGWEN,
};
use crate::registers::top_earlgrey::{
    DirectPads, MuxedPads, PinmuxInsel, PinmuxOutsel, PinmuxPeripheralIn, PINMUX_AON_BASE_ADDR,
    PINMUX_MIO_PERIPH_INSEL_IDX_OFFSET,
};

pub const PINMUX_BASE: StaticRef<PinmuxRegisters> =
    unsafe { StaticRef::new(PINMUX_AON_BASE_ADDR as *const PinmuxRegisters) };

enum PadAttribute {
    Mio(LocalRegisterCopy<u32, MIO_PAD_ATTR::Register>),
    Dio(LocalRegisterCopy<u32, DIO_PAD_ATTR::Register>),
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Pad {
    Mio(MuxedPads),
    Dio(DirectPads),
}

impl Pad {
    /// Extract value of attributes using common layout
    fn pad_attr(&self) -> PadAttribute {
        match *self {
            Self::Mio(mio) => PadAttribute::Mio(mio_pad_attr(&mio)),
            Self::Dio(dio) => PadAttribute::Dio(dio_pad_attr(&dio)),
        }
    }

    pub fn set_floating_state(&self, mode: gpio::FloatingState) {
        match *self {
            Self::Mio(mio) => {
                modify_mio_pad_attr(
                    &mio,
                    match mode {
                        gpio::FloatingState::PullUp => {
                            MIO_PAD_ATTR::PULL_EN_0::SET + MIO_PAD_ATTR::PULL_SELECT_0::PULL_UP
                        }
                        gpio::FloatingState::PullDown => {
                            MIO_PAD_ATTR::PULL_EN_0::SET + MIO_PAD_ATTR::PULL_SELECT_0::PULL_DOWN
                        }
                        gpio::FloatingState::PullNone => {
                            MIO_PAD_ATTR::PULL_EN_0::CLEAR + MIO_PAD_ATTR::PULL_SELECT_0::PULL_UP
                        }
                    },
                );
            }
            Self::Dio(dio) => {
                modify_dio_pad_attr(
                    &dio,
                    match mode {
                        gpio::FloatingState::PullUp => {
                            DIO_PAD_ATTR::PULL_EN_0::SET + DIO_PAD_ATTR::PULL_SELECT_0::PULL_UP
                        }
                        gpio::FloatingState::PullDown => {
                            DIO_PAD_ATTR::PULL_EN_0::SET + DIO_PAD_ATTR::PULL_SELECT_0::PULL_DOWN
                        }
                        gpio::FloatingState::PullNone => {
                            DIO_PAD_ATTR::PULL_EN_0::CLEAR + DIO_PAD_ATTR::PULL_SELECT_0::PULL_UP
                        }
                    },
                );
            }
        }
    }

    pub fn set_output_open_drain(&self) {
        match *self {
            Self::Mio(mio) => modify_mio_pad_attr(&mio, MIO_PAD_ATTR::OD_EN_0::SET),
            Self::Dio(dio) => modify_dio_pad_attr(&dio, DIO_PAD_ATTR::OD_EN_0::SET),
        }
    }

    pub fn set_output_push_pull(&self) {
        match *self {
            Self::Mio(mio) => modify_mio_pad_attr(&mio, MIO_PAD_ATTR::OD_EN_0::CLEAR),
            Self::Dio(dio) => modify_dio_pad_attr(&dio, DIO_PAD_ATTR::OD_EN_0::CLEAR),
        }
    }

    pub fn set_invert_sense(&self, invert: bool) {
        match *self {
            Self::Mio(mio) => {
                if invert {
                    modify_mio_pad_attr(&mio, MIO_PAD_ATTR::INVERT_0::SET)
                } else {
                    modify_mio_pad_attr(&mio, MIO_PAD_ATTR::INVERT_0::CLEAR)
                }
            }
            Self::Dio(dio) => {
                if invert {
                    modify_dio_pad_attr(&dio, DIO_PAD_ATTR::INVERT_0::SET)
                } else {
                    modify_dio_pad_attr(&dio, DIO_PAD_ATTR::INVERT_0::CLEAR)
                }
            }
        }
    }

    pub fn floating_state(&self) -> gpio::FloatingState {
        let pad_attr: PadAttribute = self.pad_attr();
        if match pad_attr {
            PadAttribute::Mio(attr) => attr
                .matches_all(MIO_PAD_ATTR::PULL_SELECT_0::PULL_UP + MIO_PAD_ATTR::PULL_EN_0::SET),
            PadAttribute::Dio(attr) => attr
                .matches_all(DIO_PAD_ATTR::PULL_SELECT_0::PULL_UP + DIO_PAD_ATTR::PULL_EN_0::SET),
        } {
            gpio::FloatingState::PullUp
        } else if match pad_attr {
            PadAttribute::Mio(attr) => attr
                .matches_all(MIO_PAD_ATTR::PULL_SELECT_0::PULL_DOWN + MIO_PAD_ATTR::PULL_EN_0::SET),
            PadAttribute::Dio(attr) => attr
                .matches_all(DIO_PAD_ATTR::PULL_SELECT_0::PULL_DOWN + DIO_PAD_ATTR::PULL_EN_0::SET),
        } {
            gpio::FloatingState::PullDown
        } else {
            gpio::FloatingState::PullNone
        }
    }

    /// Prohibits any further changes to input/output/open-drain or pullup configuration.
    pub fn lock_pad_attributes(&self) {
        match *self {
            Self::Mio(mio) => PINMUX_BASE.mio_pad_attr_regwen[(mio as u32) as usize]
                .write(MIO_PAD_ATTR_REGWEN::EN_0::CLEAR),
            Self::Dio(dio) => PINMUX_BASE.dio_pad_attr_regwen[(dio as u32) as usize]
                .write(DIO_PAD_ATTR_REGWEN::EN_0::CLEAR),
        };
    }
}

/// Extract value of MIO pad attributes.
fn mio_pad_attr(mio: &MuxedPads) -> LocalRegisterCopy<u32, MIO_PAD_ATTR::Register> {
    LocalRegisterCopy::new(PINMUX_BASE.mio_pad_attr[*mio as usize].get())
}

/// Extract value of DIO pad attributes.
fn dio_pad_attr(dio: &DirectPads) -> LocalRegisterCopy<u32, DIO_PAD_ATTR::Register> {
    LocalRegisterCopy::new(PINMUX_BASE.dio_pad_attr[*dio as usize].get())
}

/// Modify value of MIO pad attribute.
fn modify_mio_pad_attr(mio: &MuxedPads, flags: FieldValue<u32, MIO_PAD_ATTR::Register>) {
    let mut attr = mio_pad_attr(mio);
    attr.modify(flags);
    PINMUX_BASE.mio_pad_attr[*mio as usize].set(attr.get())
}

/// Modify value of DIO pad attribute.
fn modify_dio_pad_attr(dio: &DirectPads, flags: FieldValue<u32, DIO_PAD_ATTR::Register>) {
    let mut attr = dio_pad_attr(dio);
    attr.modify(flags);
    PINMUX_BASE.dio_pad_attr[*dio as usize].set(attr.get())
}

// Configuration of PINMUX multiplexers for I/O
// OpenTitan Documentation reference:
// https://opentitan.org/book/hw/ip/pinmux/doc/programmers_guide.html#pinmux-configuration

pub trait SelectOutput {
    /// Connect particular pad to internal peripheral
    fn connect_output(self, output: PinmuxOutsel);

    /// Connect particular pad output to always low
    fn connect_low(self);

    /// Connect particular pad output to always high
    fn connect_high(self);

    /// This function disconnect pad from peripheral
    /// and set it to High-Impedance state
    fn connect_high_z(self);

    /// Lock selection of output for particular pad
    fn lock(self);

    /// Get value of current output selection
    fn get_selector(self) -> PinmuxOutsel;
}

// We make a implicit conversion between PinmuxMioOut and MuxedPad
impl SelectOutput for MuxedPads {
    fn connect_output(self, output: PinmuxOutsel) {
        PINMUX_BASE.mio_outsel[self as usize].set(output as u32)
    }

    fn connect_low(self) {
        PINMUX_BASE.mio_outsel[self as usize].set(PinmuxOutsel::ConstantZero as u32)
    }

    fn connect_high(self) {
        PINMUX_BASE.mio_outsel[self as usize].set(PinmuxOutsel::ConstantOne as u32)
    }

    fn connect_high_z(self) {
        PINMUX_BASE.mio_outsel[self as usize].set(PinmuxOutsel::ConstantHighZ as u32)
    }

    fn lock(self) {
        PINMUX_BASE.mio_outsel_regwen[self as usize].write(MIO_OUTSEL_REGWEN::EN_0::CLEAR);
    }

    fn get_selector(self) -> PinmuxOutsel {
        match PinmuxOutsel::try_from(PINMUX_BASE.mio_outsel[self as usize].get()) {
            Ok(sel) => sel,
            // When this panic happend it mean we have some glitch in registers
            // or a incorect version definition of registers.
            Err(val) => panic!("PINMUX: Invalid register value: {}", val),
        }
    }
}

pub trait SelectInput {
    /// Connect internal peripheral input to particular pad
    fn connect_input(self, input: PinmuxInsel);

    /// Connect internal peripherals input to always low
    fn connect_low(self);

    /// Connect internal peripherals input to always high
    fn connect_high(self);

    /// Lock input configurations
    fn lock(self);

    /// Get value of current input selection
    fn get_selector(self) -> PinmuxInsel;
}

/// MuxedPads names and values overlap with PinmuxInsel,
/// function below is used to convert it to valid PinmuxInsel.
/// OpenTitan documentation reference:
/// <https://opentitan.org/book/hw/ip/pinmux/doc/programmers_guide.html#pinmux-configuration>
impl From<MuxedPads> for PinmuxInsel {
    fn from(pad: MuxedPads) -> Self {
        // Add 2 to skip constant ConstantZero and ConstantOne.
        match PinmuxInsel::try_from(pad as u32 + PINMUX_MIO_PERIPH_INSEL_IDX_OFFSET as u32) {
            Ok(select) => select,
            Err(_) => PinmuxInsel::ConstantZero,
        }
    }
}

impl SelectInput for PinmuxPeripheralIn {
    fn connect_input(self, input: PinmuxInsel) {
        PINMUX_BASE.mio_periph_insel[self as usize].set(input as u32)
    }

    fn connect_low(self) {
        PINMUX_BASE.mio_periph_insel[self as usize].set(PinmuxInsel::ConstantZero as u32)
    }

    fn connect_high(self) {
        PINMUX_BASE.mio_periph_insel[self as usize].set(PinmuxInsel::ConstantOne as u32)
    }

    fn lock(self) {
        PINMUX_BASE.mio_periph_insel_regwen[self as usize]
            .write(MIO_PERIPH_INSEL_REGWEN::EN_0::CLEAR);
    }

    fn get_selector(self) -> PinmuxInsel {
        match PinmuxInsel::try_from(PINMUX_BASE.mio_periph_insel[self as usize].get()) {
            Ok(sel) => sel,
            //
            Err(val) => panic!("PINMUX: Invalid insel register value {}", val),
        }
    }
}

// Enum below represent connection betwen pad and peripherals
// Diagram bellow help with interpreting meaning of input/output in enum bellow
// <https://opentitan.org/book/hw/ip/pinmux/doc/theory_of_operation.html#muxing-matrix>
// According to OpenTitan documentations uninitialized pinmux I/O selector are set to default
// values. With are respectively
// output selector - PinmuxOutsel::ConstantHighZ
// input selector - PinmuxInsel::ConstantZero
// <https://opentitan.org/book/hw/ip/pinmux/doc/registers.html#mio_outsel>
// <https://opentitan.org/book/hw/ip/pinmux/doc/registers.html#mio_periph_insel>
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PadConfig {
    // Internal Output and input not conected to any pad
    Unconnected,
    // Allow to pass signal from pad to peripheral
    // [PAD]------>[PeripherapInput]
    Input(MuxedPads, PinmuxPeripheralIn),
    // Allow to pass signal form peripheral to pad
    // [PAD]<------[PeripheralOut]
    Output(MuxedPads, PinmuxOutsel),
    // Allow to pass signal form pad to peripheral in bouth directions
    // [PAD]------>[PeripherapInput]
    // [PAD]<------[PeripheralOut]
    InOut(MuxedPads, PinmuxPeripheralIn, PinmuxOutsel),
}

impl PadConfig {
    /// Connect Pad to internal peripheral I/O using pinmux multiplexers
    pub fn connect(&self) {
        match *self {
            PadConfig::Unconnected => {}
            PadConfig::Input(pad, peripheral_in) => {
                peripheral_in.connect_input(PinmuxInsel::from(pad));
            }
            PadConfig::Output(pad, peripheral_out) => {
                pad.connect_output(peripheral_out);
            }
            PadConfig::InOut(pad, peripheral_in, peripheral_out) => {
                peripheral_in.connect_input(PinmuxInsel::from(pad));
                pad.connect_output(peripheral_out);
            }
        }
    }

    /// Disconnect pad from internal input and connect to always Low signal
    pub fn disconnect_input(&self) {
        match *self {
            PadConfig::Unconnected => {}
            PadConfig::Input(_pad, peripheral_in) => peripheral_in.connect_low(),
            PadConfig::Output(_pad, _peripheral_out) => {}
            PadConfig::InOut(_pad, peripheral_in, _peripheral_out) => {
                peripheral_in.connect_low();
            }
        };
    }

    // Disconnect pad from internal output and connect to Hi-Z
    pub fn disconnect_output(&self) {
        match *self {
            PadConfig::Unconnected => {}
            PadConfig::Input(_pad, _peripheral_in) => {}
            PadConfig::Output(pad, _peripheral_out) => pad.connect_high_z(),
            PadConfig::InOut(pad, _peripheral_in, _peripheral_out) => {
                pad.connect_high_z();
            }
        };
    }

    /// Disconnect input and output from peripheral/pad
    /// and connect to internal Hi-Z/Low signal
    pub fn disconnect(&self) {
        match *self {
            PadConfig::Unconnected => {}
            PadConfig::Input(_pad, peripheral_in) => {
                peripheral_in.connect_low();
            }
            PadConfig::Output(pad, _peripheral_out) => {
                pad.connect_high_z();
            }
            PadConfig::InOut(pad, peripheral_in, _peripheral_out) => {
                peripheral_in.connect_low();
                pad.connect_high_z();
            }
        }
    }

    /// Return copy of `enum` representing MIO pad
    /// associated with this connection
    pub fn get_pad(&self) -> Option<Pad> {
        match *self {
            PadConfig::Unconnected => None,
            PadConfig::Input(pad, _) => Some(Pad::Mio(pad)),
            PadConfig::Output(pad, _) => Some(Pad::Mio(pad)),
            PadConfig::InOut(pad, _, _) => Some(Pad::Mio(pad)),
        }
    }
}

impl From<PadConfig> for Configuration {
    fn from(pad: PadConfig) -> Configuration {
        match pad {
            PadConfig::Unconnected => Configuration::Other,
            PadConfig::Input(_pad, peripheral_in) => match peripheral_in.get_selector() {
                PinmuxInsel::ConstantZero => Configuration::LowPower,
                PinmuxInsel::ConstantOne => Configuration::Function,
                _ => Configuration::Input,
            },
            PadConfig::Output(pad, _peripheral_out) => match pad.get_selector() {
                PinmuxOutsel::ConstantZero => Configuration::Function,
                PinmuxOutsel::ConstantOne => Configuration::Function,
                PinmuxOutsel::ConstantHighZ => Configuration::LowPower,
                _ => Configuration::Output,
            },
            PadConfig::InOut(pad, peripheral_in, _peripheral_out) => {
                let input_selector = peripheral_in.get_selector();
                let output_selector = pad.get_selector();
                match (input_selector, output_selector) {
                    (PinmuxInsel::ConstantZero, PinmuxOutsel::ConstantHighZ) => {
                        Configuration::LowPower
                    }
                    (
                        PinmuxInsel::ConstantOne | PinmuxInsel::ConstantZero,
                        PinmuxOutsel::ConstantZero | PinmuxOutsel::ConstantOne,
                    ) => Configuration::Function,
                    (_, _) => Configuration::InputOutput,
                }
            }
        }
    }
}

impl Configure for PadConfig {
    fn configuration(&self) -> Configuration {
        Configuration::from(*self)
    }

    fn make_output(&self) -> Configuration {
        match self.configuration() {
            Configuration::LowPower => self.connect(),
            _ => {}
        };
        self.configuration()
    }

    fn disable_output(&self) -> Configuration {
        self.disconnect_output();
        self.configuration()
    }

    fn make_input(&self) -> Configuration {
        match self.configuration() {
            Configuration::LowPower => self.connect(),
            _ => {}
        };
        self.configuration()
    }

    fn disable_input(&self) -> Configuration {
        self.disconnect_input();
        self.configuration()
    }

    fn deactivate_to_low_power(&self) {
        self.disconnect();
    }

    fn set_floating_state(&self, state: FloatingState) {
        if let Some(pad) = self.get_pad() {
            pad.set_floating_state(state);
        }
    }

    fn floating_state(&self) -> FloatingState {
        if let Some(pad) = self.get_pad() {
            pad.floating_state()
        } else {
            FloatingState::PullNone
        }
    }
}
