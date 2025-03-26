// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Interfaces for peripherals that deal with MCU reset functionality

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetReason {
    /// Cold boot power on reset
    PowerOnReset,

    /// Warm boot after MCU was put to sleep with CPU off
    LowPowerExit,

    /// Software requested that the MCU should be reset
    SoftwareRequest,

    /// Software failed and made the MCU reset
    SoftwareFault,

    /// Debugger requested that the MCU reset
    Debug,

    /// External REST pin asserted
    HardwareLine,

    /// Watchdog triggered a reset
    Watchdog,

    /// Voltage is out of range
    VoltageFault,

    /// One of the peripherls failed and triggered a reset
    PeripheralFault(u16),

    /// One of the peripherals requested a reset
    PeripheralRequest(u16),

    /// Unknown reason for reset, maybe multiple reasons were detected
    Unknown(u16),
}

/// Simple interface for reading an ADC sample on any channel.
pub trait ResetManagment {
    type ResetInfo: AsRef<[u32]> + Copy;

    /// determine the reason the MCU was last reset
    fn reset_reason(&self) -> Option<ResetReason>;

    /// supplementary reset reason information
    /// return a reference to a slice of u32s as:
    ///  - the info is ofthen unstructured (different from MCU to MCU)
    ///  - is not meant to be interpreted by the MCU during runtime but rather stored/displayed
    fn get_reset_info_dump(&self) -> Option<Self::ResetInfo>;

    /// trigger a software system reset.
    /// This function should not return as the MCU should reset (and not continue code execution) if it successful, if code continues to execute after this then the reset was not possible
    fn trigger_system_reset(&self);
}
