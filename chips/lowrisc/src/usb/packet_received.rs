// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Packet received

use super::buffer_index::BufferIndex;
use super::endpoint_index::EndpointIndex;
use super::packet_size::PacketSize;

use crate::registers::usbdev_regs::RXFIFO;

use kernel::utilities::registers::LocalRegisterCopy;

/// SETUP packet
pub(super) struct SetupPacket {
    rx_fifo_content: LocalRegisterCopy<u32, RXFIFO::Register>,
}

impl SetupPacket {
    /// [SetupPacket] constructor
    ///
    /// # Parameters
    ///
    /// + ̀`rx_fifo_content`: the content of the receive FIFO representing the SETUP packet
    ///
    /// # Return value
    ///
    /// A new instance of [SetupPacket]
    const fn new(rx_fifo_content: LocalRegisterCopy<u32, RXFIFO::Register>) -> Self {
        Self { rx_fifo_content }
    }

    /// Returns the index of the endpoint on which the SETUP packet was received
    ///
    /// # Return value
    ///
    /// The index of the endpoint on which the SETUP packet was received
    pub(super) fn get_endpoint_index(&self) -> EndpointIndex {
        // PANIC: `try_from_usize()` may panic only if the hardware is not working properly
        // CAST: u32 == usize on RV32I
        EndpointIndex::try_from_usize(self.rx_fifo_content.read(RXFIFO::EP) as usize).unwrap()
    }

    /// Returns the index of the controller's buffer containing the SETUP packet
    ///
    /// # Return value
    ///
    /// The index of the buffer containing the SETUP packet
    pub(super) fn get_buffer_index(&self) -> BufferIndex {
        // PANIC: `try_from_usize()` never panics because the BUFFER field is 5-bit wide.
        // CAST: u32 == usize on RV32I
        BufferIndex::try_from_usize(self.rx_fifo_content.read(RXFIFO::BUFFER) as usize).unwrap()
    }
}

/// OUT packet
pub(super) struct OutPacket {
    rx_fifo_content: LocalRegisterCopy<u32, RXFIFO::Register>,
}

impl OutPacket {
    /// [OutPacket] constructor
    ///
    /// # Parameters
    ///
    /// + ̀`rx_fifo_content`: the content of the receive FIFO representing the OUT packet
    ///
    /// # Return value
    ///
    /// A new instance of [OutPacket]
    const fn new(rx_fifo_content: LocalRegisterCopy<u32, RXFIFO::Register>) -> Self {
        Self { rx_fifo_content }
    }

    /// Returns the index of the endpoint on which the OUT packet was received
    ///
    /// # Return value
    ///
    /// The index of the endpoint on which the OUT packet was received
    pub(super) fn get_endpoint_index(&self) -> EndpointIndex {
        // PANIC: `try_from_usize()` may panic only if the hardware is not working properly
        // CAST: u32 == usize on RV32I
        EndpointIndex::try_from_usize(self.rx_fifo_content.read(RXFIFO::EP) as usize).unwrap()
    }

    /// Returns the index of the endpoint on which the OUT packet was received
    ///
    /// # Return value
    ///
    /// The index of the buffer on which the OUT packet was received
    pub(super) fn get_buffer_index(&self) -> BufferIndex {
        // PANIC: `try_from_usize()` never panics because the BUFFER field is 5-bit wide.
        // CAST: u32 == usize on RV32I
        BufferIndex::try_from_usize(self.rx_fifo_content.read(RXFIFO::BUFFER) as usize).unwrap()
    }

    /// Returns the size of the packet
    ///
    /// # Return value
    ///
    /// Packet's size
    pub(super) fn get_size(&self) -> PacketSize {
        // PANIC: `try_form_usize()` may panic only if the hardware is not working properly
        // CAST: u32 == usize on RV32i
        PacketSize::try_from_usize(self.rx_fifo_content.read(RXFIFO::SIZE) as usize).unwrap()
    }
}

/// Receive packet
pub(super) enum PacketReceived {
    /// Setup packet received
    Setup(SetupPacket),
    /// Out packet received
    Out(OutPacket),
}

impl PacketReceived {
    /// [PacketReceived] constructor
    ///
    /// # Parameters
    ///
    /// + ̀`rx_fifo_content`: the content of the receive FIFO representing the receive packet
    ///
    /// # Return value
    ///
    /// A new instance of [PacketReceived]
    pub(super) fn new(rx_fifo_content: LocalRegisterCopy<u32, RXFIFO::Register>) -> Self {
        match rx_fifo_content.is_set(RXFIFO::SETUP) {
            false => PacketReceived::Out(OutPacket::new(rx_fifo_content)),
            true => PacketReceived::Setup(SetupPacket::new(rx_fifo_content)),
        }
    }
}
