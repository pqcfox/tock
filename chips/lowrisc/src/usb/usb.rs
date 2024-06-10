// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! USB driver.

use super::available_buffer_list::AvailableBufferList;
use super::endpoint::Endpoint;
use super::endpoint_index::EndpointIndex;
use super::interrupt::UsbInterrupt;

use crate::registers::usbdev_regs::{
    UsbdevRegisters, AVBUFFER, INTR, PHY_CONFIG, USBCTRL, USBSTAT,
};

use kernel::hil::usb::{Client, DeviceSpeed, TransferType, UsbController};
use kernel::utilities::cells::{OptionalCell, VolatileCell};
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable};
use kernel::utilities::StaticRef;

use core::num::NonZeroUsize;

/// Default endpoint index.
const DEFAULT_ENDPOINT_INDEX: EndpointIndex = EndpointIndex::Endpoint0;
/// Number of endpoints.
const NUMBER_ENDPOINTS: NonZeroUsize = match NonZeroUsize::new(12) {
    Some(non_zero_usize) => non_zero_usize,
    None => unreachable!(),
};

/// USB driver
pub struct Usb<'a> {
    registers: StaticRef<UsbdevRegisters>,
    client: OptionalCell<&'a dyn Client<'a>>,
    endpoints: [Endpoint<'a>; NUMBER_ENDPOINTS.get()],
    available_buffer_list: AvailableBufferList,
}

impl<'a> Usb<'a> {
    /// Constructs a new USB driver.
    ///
    /// The returned driver:
    ///
    /// + has all endpoints disabled
    /// + no buffer set for any endpoint
    /// + no client set
    ///
    /// # Parameters
    ///
    /// + `registers`: the base address of the USB registers.
    ///
    /// # Return value
    ///
    /// A new instance of [Usb]
    pub const fn new(registers: StaticRef<UsbdevRegisters>) -> Self {
        Self {
            registers,
            client: OptionalCell::empty(),
            endpoints: [
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
            ],
            available_buffer_list: AvailableBufferList::new(),
        }
    }

    /// Get an endpoint.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint to be returned.
    ///
    /// # Return value
    ///
    /// A reference to the endpoint pointed to by `endpoint_index`.
    fn get_endpoint(&self, endpoint_index: EndpointIndex) -> &Endpoint<'a> {
        // PANIC: EndpointIndex guarantees safe access to `endpoints`
        self.endpoints.get(endpoint_index.to_usize()).unwrap()
    }

    /// Sets the IN buffer for the given endpoint.
    ///
    /// The IN buffer is used for IN transactions.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer must be set
    /// + `buffer_in`: the IN buffer
    fn set_buffer_in(&self, endpoint_index: EndpointIndex, buffer_in: &'a [VolatileCell<u8>]) {
        self.get_endpoint(endpoint_index).set_buffer_in(buffer_in);
    }

    /// Sets the OUT buffer for the given endpoint.
    ///
    /// The OUT buffer is used for OUT transactions.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose OUT buffer must be set
    /// + `buffer_out`: the OUT buffer
    fn set_buffer_out(&self, endpoint_index: EndpointIndex, buffer_out: &'a [VolatileCell<u8>]) {
        self.get_endpoint(endpoint_index).set_buffer_out(buffer_out);
    }

    /// Fills the available buffer FIFO.
    ///
    /// The USB controller has a FIFO for available buffers. Available buffers are used to store
    /// data received in OUT/SETUP transactions. This method fills the FIFO with available buffers.
    fn fill_available_buffer_fifo(&self) {
        while !self.registers.usbstat.is_set(USBSTAT::AV_FULL) {
            let buffer_index = self.available_buffer_list.next_and_occupy();
            // CAST: u32 == usize
            self.registers
                .avbuffer
                .modify(AVBUFFER::BUFFER.val(buffer_index.to_usize() as u32));
        }
    }

    /// Enable interrupts.
    fn enable_interrupts(&self) {
        self.registers.intr_enable.modify(
            INTR::PKT_RECEIVED::SET
                + INTR::PKT_SENT::SET
                + INTR::DISCONNECTED::SET
                + INTR::HOST_LOST::SET
                + INTR::LINK_RESET::SET
                + INTR::LINK_SUSPEND::SET
                + INTR::LINK_RESUME::SET
                + INTR::AV_EMPTY::SET
                + INTR::RX_FULL::SET
                + INTR::AV_OVERFLOW::SET
                + INTR::LINK_IN_ERR::SET
                + INTR::RX_CRC_ERR::SET
                + INTR::RX_PID_ERR::SET
                + INTR::RX_BITSTUFF_ERR::SET
                + INTR::FRAME::SET
                + INTR::POWERED::SET
                + INTR::LINK_OUT_ERR::SET,
        );
    }

    /// Disable interrupts.
    fn disable_interrupts(&self) {
        self.registers.intr_enable.modify(
            INTR::PKT_RECEIVED::CLEAR
                + INTR::PKT_SENT::CLEAR
                + INTR::DISCONNECTED::CLEAR
                + INTR::HOST_LOST::CLEAR
                + INTR::LINK_RESET::CLEAR
                + INTR::LINK_SUSPEND::CLEAR
                + INTR::LINK_RESUME::CLEAR
                + INTR::AV_EMPTY::CLEAR
                + INTR::RX_FULL::CLEAR
                + INTR::AV_OVERFLOW::CLEAR
                + INTR::LINK_IN_ERR::CLEAR
                + INTR::RX_CRC_ERR::CLEAR
                + INTR::RX_PID_ERR::CLEAR
                + INTR::RX_BITSTUFF_ERR::CLEAR
                + INTR::FRAME::CLEAR
                + INTR::POWERED::CLEAR
                + INTR::LINK_OUT_ERR::CLEAR,
        );
    }

    /// Enable USB controller.
    fn enable(&self) {
        self.registers.usbctrl.modify(USBCTRL::ENABLE::SET);
    }

    /// Disable USB controller.
    fn disable(&self) {
        self.registers.usbctrl.modify(USBCTRL::ENABLE::CLEAR);
    }

    /// Enable IN endpoint
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the IN endpoint interface to be enabled.
    fn internal_endpoint_in_enable(&self, endpoint_index: EndpointIndex) {
        self.registers
            .ep_in_enable
            .modify(endpoint_index.to_set_ep_in_enable_field_value());
    }

    /// Enable OUT endpoint
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface to be enabled.
    fn internal_endpoint_out_enable(&self, endpoint_index: EndpointIndex) {
        self.registers
            .ep_out_enable
            .modify(endpoint_index.to_set_ep_out_enable_field_value());
    }

    /// Enable OUT packet reception
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface that must receive OUT packets.
    fn internal_endpoint_rxenable_out(&self, endpoint_index: EndpointIndex) {
        self.registers
            .rxenable_out
            .modify(endpoint_index.to_set_rxenable_out_field_value());
    }

    /// Enable SETUP packet reception
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface that must receive SETUP packets.
    fn internal_endpoint_rxenable_setup(&self, endpoint_index: EndpointIndex) {
        self.registers
            .rxenable_setup
            .modify(endpoint_index.to_set_rxenable_setup_field_value());
    }

    /// USB driver interrupt handler.
    ///
    /// # Parameters
    ///
    /// + `usb_interrupt`: the USB interrupt to be handled.
    pub fn handle_interrupt(&self, usb_interrupt: UsbInterrupt) {
        match usb_interrupt {
            UsbInterrupt::PacketReceived => unimplemented!(),
            UsbInterrupt::PacketSent => unimplemented!(),
            UsbInterrupt::Disconnected => unimplemented!(),
            UsbInterrupt::HostLost => unimplemented!(),
            UsbInterrupt::LinkReset => unimplemented!(),
            UsbInterrupt::LinkSuspended => unimplemented!(),
            UsbInterrupt::LinkResume => unimplemented!(),
            UsbInterrupt::AvEmpty => unimplemented!(),
            UsbInterrupt::RxFull => unimplemented!(),
            UsbInterrupt::AvOverflow => unimplemented!(),
            UsbInterrupt::LinkInErr => unimplemented!(),
            UsbInterrupt::RxCrcErr => unimplemented!(),
            UsbInterrupt::RxPidErr => unimplemented!(),
            UsbInterrupt::RxBitstuffErr => unimplemented!(),
            UsbInterrupt::Frame => unimplemented!(),
            UsbInterrupt::Powered => unimplemented!(),
            UsbInterrupt::LinkOutErr => unimplemented!(),
        }
    }
}

impl<'a> UsbController<'a> for Usb<'a> {
    fn set_client(&self, client: &'a dyn Client<'a>) {
        self.client.set(client);
    }

    fn endpoint_set_ctrl_buffer(&self, buffer: &'a [VolatileCell<u8>]) {
        self.set_buffer_in(DEFAULT_ENDPOINT_INDEX, buffer);
        self.set_buffer_out(DEFAULT_ENDPOINT_INDEX, buffer);
    }

    fn endpoint_set_in_buffer(&self, raw_endpoint_index: usize, buffer: &'a [VolatileCell<u8>]) {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return;
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.set_buffer_in(endpoint_index, buffer);
    }

    fn endpoint_set_out_buffer(&self, raw_endpoint_index: usize, buffer: &'a [VolatileCell<u8>]) {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return;
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.set_buffer_out(endpoint_index, buffer);
    }

    fn enable_as_device(&self, _speed: DeviceSpeed) {
        self.registers.phy_config.modify(
            PHY_CONFIG::USE_DIFF_RCVR::SET
                + PHY_CONFIG::TX_USE_D_SE0::CLEAR
                + PHY_CONFIG::EOP_SINGLE_BIT::SET
                + PHY_CONFIG::PINFLIP::CLEAR
                + PHY_CONFIG::USB_REF_DISABLE::CLEAR
                + PHY_CONFIG::TX_OSC_TEST_MODE::CLEAR,
        );
    }

    fn attach(&self) {
        self.fill_available_buffer_fifo();
        self.enable_interrupts();
        self.enable();
    }

    fn detach(&self) {
        self.disable();
        self.disable_interrupts();
    }

    fn set_address(&self, _address: u16) {
        unimplemented!()
    }

    fn enable_address(&self) {
        unimplemented!()
    }

    fn endpoint_in_enable(&self, _transfer_type: TransferType, raw_endpoint_index: usize) {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return;
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_in_enable(endpoint_index);
    }

    fn endpoint_out_enable(&self, _transfer_type: TransferType, raw_endpoint_index: usize) {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return;
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_out_enable(endpoint_index);
        self.internal_endpoint_rxenable_out(endpoint_index);
        self.internal_endpoint_rxenable_setup(endpoint_index);
    }

    fn endpoint_in_out_enable(&self, _transfer_type: TransferType, raw_endpoint_index: usize) {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return;
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_in_enable(endpoint_index);
        self.internal_endpoint_out_enable(endpoint_index);
        self.internal_endpoint_rxenable_out(endpoint_index);
        self.internal_endpoint_rxenable_setup(endpoint_index);
    }

    fn endpoint_resume_in(&self, _endpoint: usize) {
        unimplemented!()
    }

    fn endpoint_resume_out(&self, _endpoint: usize) {
        unimplemented!()
    }
}
