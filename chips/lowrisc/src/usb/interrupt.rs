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
    /// Raised if a packet was received using an OUT or SETUP transaction.  This
    /// interrupt is directly tied to whether the RX FIFO is empty, so it should
    /// be cleared only after handling the FIFO entry.
    PacketReceived,
    /// Raised if a packet was sent as part of an IN transaction.  This
    /// interrupt is directly tied to whether a sent packet has not been
    /// acknowledged in the `in_sent` register.  It should be cleared only after
    /// clearing all bits in the `in_sent` register.
    PacketSent,
    /// Raised if VBUS is lost, thus the link is disconnected.
    Disconnected,
    /// Raised if link is active but SOF was not received from host for 4.096
    /// ms. The SOF should be every 1 ms.
    HostLost,
    /// Raised if the link is at SE0 longer than 3 us indicating a link reset
    /// (host asserts for min 10 ms, device can react after 2.5 us).
    LinkReset,
    /// Raised if the line has signaled J for longer than 3ms and is therefore
    /// in suspend state.
    LinkSuspended,
    /// Raised when the link becomes active again after being suspended.
    LinkResume,
    /// Raised when the Available OUT Buffer FIFO is empty and the device
    /// interface is enabled.  This interrupt is directly tied to the FIFO
    /// status, so the Available OUT Buffer FIFO must be provided with a free
    /// buffer before the interrupt can be cleared.
    AvOutEmpty,
    /// Raised when the RX FIFO is full and the device interface is enabled.
    /// This interrupt is directly tied to the FIFO status, so the RX FIFO must
    /// have an entry removed before the interrupt is cleared. If the condition
    /// is not cleared, the interrupt can re-assert.
    RxFull,
    /// Raised if a write was done to either the Available OUT Buffer FIFO or
    /// the Available SETUP Buffer FIFO when the FIFO was full.
    AvOverflow,
    /// Raised if a packet to an IN endpoint started to be received but was then
    /// dropped due to an error. After transmitting the IN payload, the USB
    /// device expects a valid ACK handshake packet. This error is raised if
    /// either the packet or CRC is invalid, leading to a NAK instead, or if a
    /// different token was received.
    LinkInErr,
    /// Raised if a CRC error occurred on a received packet.
    RxCrcErr,
    /// Raised if an invalid Packet IDentifier (PID) was received.
    RxPidErr,
    /// Raised if an invalid bitstuffing was received.
    RxBitstuffErr,
    /// Raised when the USB frame number is updated with a valid SOF.
    Frame,
    /// Raised if VBUS is applied.
    Powered,
    /// Raised if a packet to an OUT endpoint started to be received but was
    /// then dropped due to an error.  This error is raised if the data toggle,
    /// token, packet and/or CRC are invalid, or if the appropriate Available
    /// OUT Buffer FIFO is empty and/or the Received Buffer FIFO is full when a
    /// packet should have been received.
    LinkOutErr,
    /// Raised when the Available SETUP Buffer FIFO is empty and the device
    /// interface is enabled.  This interrupt is directly tied to the FIFO
    /// status, so the Available SETUP Buffer FIFO must be provided with a free
    /// buffer before the interrupt can be cleared.
    AvSetupEmpty,
}
