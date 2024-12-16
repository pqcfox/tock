// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! USB interrupts

#[repr(usize)]
/// A list of all USB interrupts
pub enum UsbInterrupt {
    /// A packet has been received
    PacketReceived = 135,
    /// A packet has been sent
    PacketSent,
    /// VBUS disconnected
    Disconnected,
    /// No frame packet for more than 4.096ms
    HostLost,
    /// The host reset the link
    LinkReset,
    /// No link activity for more than 3ms
    LinkSuspended,
    /// Link active again after being suspended
    LinkResume,
    /// Empty available buffer FIFO
    AvEmpty,
    /// Receive FIFO full.
    RxFull,
    /// Attempt to write to available buffer FIFO when full
    AvOverflow,
    /// Error during IN transaction
    LinkInErr,
    /// Packet received with invalid CRC
    RxCrcErr,
    /// Packet received with invalid PID
    RxPidErr,
    /// Invalid bitstuffing received
    RxBitstuffErr,
    /// Frame packet received
    Frame,
    /// VBUS applied
    Powered,
    /// Error during OUT/SETUP transaction
    LinkOutErr,
}

/// Returned when an invalid USB interrupt ID is provided.
#[derive(Debug)]
pub struct UsbInvalidInterruptError;

impl UsbInterrupt {
    /// Converts a usize to a USB interrupt
    ///
    /// # Parameters
    ///
    /// + `value`: the usize to be converted
    ///
    /// # Return value
    ///
    /// + Ok: if value >= 135 && value <= 151
    /// + Err: if value < 135 || value > 151
    pub fn try_from_usize(value: usize) -> Result<Self, UsbInvalidInterruptError> {
        match value {
            135 => Ok(UsbInterrupt::PacketReceived),
            136 => Ok(UsbInterrupt::PacketSent),
            137 => Ok(UsbInterrupt::Disconnected),
            138 => Ok(UsbInterrupt::HostLost),
            139 => Ok(UsbInterrupt::LinkReset),
            140 => Ok(UsbInterrupt::LinkSuspended),
            141 => Ok(UsbInterrupt::LinkResume),
            142 => Ok(UsbInterrupt::AvEmpty),
            143 => Ok(UsbInterrupt::RxFull),
            144 => Ok(UsbInterrupt::AvOverflow),
            145 => Ok(UsbInterrupt::LinkInErr),
            146 => Ok(UsbInterrupt::RxCrcErr),
            147 => Ok(UsbInterrupt::RxPidErr),
            148 => Ok(UsbInterrupt::RxBitstuffErr),
            149 => Ok(UsbInterrupt::Frame),
            150 => Ok(UsbInterrupt::Powered),
            151 => Ok(UsbInterrupt::LinkOutErr),
            _ => Err(UsbInvalidInterruptError),
        }
    }
}
