// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! USB driver.

use super::available_buffer_list::AvailableBufferList;
use super::buffer_index::BufferIndex;
use super::chunk_index::ChunkIndex;
use super::chunk_index_iterator::ChunkIndexIterator;
use super::endpoint::Endpoint;
use super::endpoint_index::EndpointIndex;
use super::endpoint_index_iterator::EndpointIndexIterator;
use super::endpoint_state::{
    CtrlEndpointState, EndpointState, ReceiveCtrlEndpointState, TransmitCtrlEndpointState,
};
use super::interrupt::UsbInterrupt;
use super::packet_received::{OutPacket, PacketReceived, SetupPacket};
use super::packet_size::{PacketSize, EMPTY_PACKET_SIZE};
use super::request::{
    ClassRequest, Request, StandardDeviceRequest, StandardDeviceRequestFromHost, StandardRequest,
};
use super::usb_address::UsbAddress;
use super::utils;

use crate::registers::usbdev_regs::{
    UsbdevRegisters, AVOUTBUFFER, AVSETUPBUFFER, CONFIGIN, INTR, IN_SENT, PHY_CONFIG, USBCTRL,
    USBSTAT,
};

use kernel::hil::usb::{
    self, Client, CtrlInResult, CtrlOutResult, CtrlSetupResult, DeviceSpeed, InResult, OutResult,
    TransferType, UsbController,
};
use kernel::utilities::cells::{OptionalCell, VolatileCell};
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::StaticRef;

use core::cell::Cell;
use core::num::{NonZeroU16, NonZeroUsize};

/// Default endpoint index.
const DEFAULT_ENDPOINT_INDEX: EndpointIndex = EndpointIndex::Endpoint0;
/// Number of endpoints.
pub(super) const NUMBER_ENDPOINTS: NonZeroUsize = utils::create_non_zero_usize(12);

const WORD_SIZE: NonZeroUsize = utils::create_non_zero_usize(core::mem::size_of::<usize>());

/// USB driver
pub struct Usb<'a> {
    registers: StaticRef<UsbdevRegisters>,
    client: OptionalCell<&'a dyn Client<'a>>,
    endpoints: [Endpoint<'a>; NUMBER_ENDPOINTS.get()],
    available_buffer_list: AvailableBufferList,
    address: Cell<UsbAddress>,
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
            address: Cell::new(UsbAddress::default()),
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

    /// Fills the available buffer FIFOs for OUT and SETUP transactions, alternating between them
    /// to prevent deadlock.
    ///
    /// The USB controller has a FIFO for available buffers. Available buffers are used to store
    /// data received in OUT/SETUP transactions. This method fills the FIFO with available buffers.
    fn fill_available_buffer_fifo(&self) {
        let out_fifo_full = || self.registers.usbstat.is_set(USBSTAT::AV_OUT_FULL);
        let setup_fifo_full = || self.registers.usbstat.is_set(USBSTAT::AV_SETUP_FULL);
        let mut either = true;
        while either {
            either = false;
            if !out_fifo_full() {
                let buffer_index = self.available_buffer_list.next_and_occupy();
                // CAST: u32 == usize
                self.registers
                    .avoutbuffer
                    .modify(AVOUTBUFFER::BUFFER.val(buffer_index.to_usize() as u32));
                either = true;
            }
            if !setup_fifo_full() {
                let buffer_index = self.available_buffer_list.next_and_occupy();
                // CAST: u32 == usize
                self.registers
                    .avsetupbuffer
                    .modify(AVSETUPBUFFER::BUFFER.val(buffer_index.to_usize() as u32));
                either = true;
            }
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
                + INTR::AV_OUT_EMPTY::SET
                + INTR::AV_SETUP_EMPTY::SET
                + INTR::RX_FULL::SET
                + INTR::AV_OVERFLOW::SET
                + INTR::LINK_IN_ERR::SET
                + INTR::RX_CRC_ERR::SET
                + INTR::RX_PID_ERR::SET
                + INTR::RX_BITSTUFF_ERR::SET
                + INTR::FRAME::SET
                + INTR::POWERED::SET, //+ INTR::LINK_OUT_ERR::SET,
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
                + INTR::AV_OUT_EMPTY::CLEAR
                + INTR::AV_SETUP_EMPTY::CLEAR
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

    /// Converts a transfer type to an endpoint state.
    ///
    /// For control transfers, the returned endpoint state is the initial state: waiting to receive
    /// a SETUP packet.
    ///
    /// # Parameters
    ///
    /// + ̀`transfer_type`: the transfer type to be converted
    ///
    /// # Return value
    ///
    /// The endpoint state representation of the transfer type
    fn convert_transfer_type_to_endpoint_state(transfer_type: TransferType) -> EndpointState {
        match transfer_type {
            TransferType::Control => {
                EndpointState::Ctrl(CtrlEndpointState::Receive(ReceiveCtrlEndpointState::Setup))
            }
            TransferType::Bulk => EndpointState::Bulk,
            TransferType::Isochronous => EndpointState::Isochronous,
            TransferType::Interrupt => EndpointState::Interrupt,
        }
    }

    fn convert_endpoint_state_to_transfer_type(endpoint_state: EndpointState) -> TransferType {
        match endpoint_state {
            EndpointState::Ctrl(_) => TransferType::Control,
            EndpointState::Bulk => TransferType::Bulk,
            EndpointState::Interrupt => TransferType::Interrupt,
            EndpointState::Isochronous => TransferType::Isochronous,
        }
    }

    /// Initializes endpoint state for the given transfer type
    ///
    /// # Parameters
    ///
    /// + ̀`transfer_type`: transfer type used by the endpoint
    /// + ̀`endpoint_index`: the index of the endpoint to be initialized
    fn initialize_endpoint_state(
        &self,
        transfer_type: TransferType,
        endpoint_index: EndpointIndex,
    ) {
        let endpoint = self.get_endpoint(endpoint_index);
        let endpoint_state = Self::convert_transfer_type_to_endpoint_state(transfer_type);
        endpoint.set_state(endpoint_state);
    }

    /// Enable IN endpoint
    ///
    /// # Parameters:
    ///
    /// + `transfer_type`: the type of IN transfers performed on the given endpoint
    /// + `endpoint_index`: the index of the IN endpoint interface to be enabled.
    fn internal_endpoint_in_enable(
        &self,
        transfer_type: TransferType,
        endpoint_index: EndpointIndex,
    ) {
        self.initialize_endpoint_state(transfer_type, endpoint_index);
        self.registers.ep_in_enable[0].modify(endpoint_index.to_set_ep_in_enable_field_value());
    }

    /// Enable OUT endpoint
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface to be enabled.
    fn internal_endpoint_out_enable(
        &self,
        transfer_type: TransferType,
        endpoint_index: EndpointIndex,
    ) {
        self.initialize_endpoint_state(transfer_type, endpoint_index);
        self.registers.ep_out_enable[0].modify(endpoint_index.to_set_ep_out_enable_field_value());
    }

    /// Enable OUT packet reception
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface that must receive OUT packets.
    fn internal_endpoint_rxenable_out(&self, endpoint_index: EndpointIndex) {
        self.registers.rxenable_out[0].modify(endpoint_index.to_set_rxenable_out_field_value());
    }

    /// Disable OUT packet reception
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface that must be disabled.
    fn internal_endpoint_rxdisable_out(&self, endpoint_index: EndpointIndex) {
        self.registers.rxenable_out[0].modify(endpoint_index.to_clear_rxenable_out_field_value());
    }

    /// Enable SETUP packet reception
    ///
    /// # Parameters:
    ///
    /// + `endpoint_index`: the index of the OUT endpoint interface that must receive SETUP packets.
    fn internal_endpoint_rxenable_setup(&self, endpoint_index: EndpointIndex) {
        self.registers.rxenable_setup[0].modify(endpoint_index.to_set_rxenable_setup_field_value());
    }

    fn internal_enable_in_isochronous(&self, endpoint_index: EndpointIndex) {
        self.registers.in_iso[0].modify(endpoint_index.to_set_in_iso_field_value());
    }

    fn internal_enable_out_isochronous(&self, endpoint_index: EndpointIndex) {
        self.registers.out_iso[0].modify(endpoint_index.to_set_out_iso_field_value());
    }

    /// Get a chunk from the controller's buffer
    ///
    /// # Parameters
    ///
    /// + `chunk_index`: the index of the chunk to be returned
    ///
    /// # Return value
    ///
    /// The desired chunk
    fn get_buffer_chunk(&self, chunk_index: ChunkIndex) -> &ReadWrite<u32> {
        // PANIC: ChunkIndex guarantees safe access to `buffer`
        self.registers.buffer.get(chunk_index.to_usize()).unwrap()
    }

    /// Reads a chunk from the controller's buffer
    ///
    /// # Parameters
    ///
    /// + `chunk_index`: the index of the chunk to be read
    ///
    /// # Return value
    ///
    /// Chunk's stored value
    fn read_chunk(&self, chunk_index: ChunkIndex) -> usize {
        // CAST: u32 == usize on RV32I
        self.get_buffer_chunk(chunk_index).get() as usize
    }

    /// Writes a chunk to the controller's buffer
    ///
    /// # Parameters
    ///
    /// + `word`: the value to be written
    /// + `chunk_index`: the index of the chunk to be overwritten
    fn write_chunk(&self, word: usize, chunk_index: ChunkIndex) {
        // CAST: u32 == usize on RV32I
        self.get_buffer_chunk(chunk_index).set(word as u32)
    }

    /// Copies a buffer from the controller to the endpoint's out buffer, if any
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the hardware buffer that acts as a source
    /// + `endpoint_out_buffer`: endpoint's out buffer
    fn copy_from_hardware_to_user(
        &self,
        buffer_index: BufferIndex,
        endpoint_out_buffer: &'a [VolatileCell<u8>],
    ) {
        let chunk_index_iterator = ChunkIndexIterator::new(buffer_index);

        for (offset, chunk_index) in chunk_index_iterator.enumerate() {
            let word = self.read_chunk(chunk_index);
            for (byte_index, byte) in word.to_ne_bytes().iter().enumerate() {
                if let Some(destination_byte) =
                    endpoint_out_buffer.get((offset * WORD_SIZE.get()) + byte_index)
                {
                    destination_byte.set(*byte);
                }
            }
        }
    }

    /// Copies a buffer from the endpoint's in buffer, if any, to the controller
    ///
    /// # Parameters
    ///
    /// + `buffer_index`: the index of the hardware buffer that acts as a destination
    /// + `endpoint_in_buffer`: endpoint's in buffer
    fn copy_from_user_to_hardware(
        &self,
        buffer_index: BufferIndex,
        endpoint_in_buffer: &'a [VolatileCell<u8>],
    ) {
        let chunk_index_iterator = ChunkIndexIterator::new(buffer_index);

        for (offset, chunk_index) in chunk_index_iterator.enumerate() {
            let mut bytes = [0u8; WORD_SIZE.get()];

            for (byte_index, byte) in bytes.iter_mut().enumerate() {
                if let Some(source_byte) =
                    endpoint_in_buffer.get((offset * WORD_SIZE.get()) + byte_index)
                {
                    *byte = source_byte.get();
                }
            }

            let word = usize::from_ne_bytes(bytes);

            self.write_chunk(word, chunk_index);
        }
    }

    /// Configures `configin` buffer for transmit
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN interface will be used
    /// + `buffer_index`: the index of the buffer that stores the packet to be transmitted
    /// + `size`: the size of the packet to be transmitted
    fn configure_in_buffer(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        size: PacketSize,
    ) {
        let configin_register = self.get_configin_register(endpoint_index);

        configin_register.modify(
            CONFIGIN::BUFFER_0.val(buffer_index.to_usize() as u32)
                + CONFIGIN::SIZE_0.val(size.to_usize() as u32)
                + CONFIGIN::RDY_0::SET,
        );
    }

    /// Sends a packet
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN interface will be used
    /// + `buffer_index`: the index of the buffer that will store the packet to be transmitted
    /// + `size`: the size of the packet to be transmitted
    /// + `endpoint_in_buffer`: endpoint's in buffer that stores the packet to be transmitted
    fn send_packet(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        size: PacketSize,
        endpoint_in_buffer: &'a [VolatileCell<u8>],
    ) {
        self.copy_from_user_to_hardware(buffer_index, endpoint_in_buffer);
        self.configure_in_buffer(endpoint_index, buffer_index, size);
    }

    /// Sends an empty packet
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN interface will be used
    /// + `buffer_index`: the index of the buffer that will store the packet to be transmitted
    fn send_empty_packet(&self, endpoint_index: EndpointIndex, buffer_index: BufferIndex) {
        self.configure_in_buffer(endpoint_index, buffer_index, EMPTY_PACKET_SIZE);
    }

    /// Handler for a standard device request to host
    ///
    /// # Parameters
    ///
    /// + `setup_packet`: the setup packet representing the request
    /// + `client`: USB client
    fn handle_standard_device_to_host_request(
        &self,
        setup_packet: SetupPacket,
        client: &'a dyn Client<'a>,
    ) {
        let endpoint_index = setup_packet.get_endpoint_index();
        let buffer_index = setup_packet.get_buffer_index();
        let endpoint = self.get_endpoint(endpoint_index);
        match client.ctrl_in(endpoint_index.to_usize()) {
            CtrlInResult::Packet(size, last) => {
                // PANIC: This panics only if the upper layer is buggy
                let packet_size = PacketSize::try_from_usize(size).unwrap();
                let endpoint_in_buffer = endpoint.get_buffer_in();
                endpoint.set_last(last);
                endpoint_in_buffer.map(|buffer_in| {
                    self.send_packet(endpoint_index, buffer_index, packet_size, buffer_in);
                    endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Transmit(
                        TransmitCtrlEndpointState::Data,
                    )));
                });
            }
            CtrlInResult::Delay => unimplemented!(),
            // Currently, there is no upper layer that sends CtrlInResult::Error, as a
            // consequence this is not implemented. A future patch may add support for proper
            // error handling.
            CtrlInResult::Error => unimplemented!(),
        }
    }

    /// Handler for a standard device request from host
    ///
    /// # Parameters
    ///
    /// + `standard_device_request_from_host`: the standard device request from host
    /// + `setup_packet`: the setup packet containing the request
    /// + `client`: USB client
    fn handle_standard_device_from_host_request(
        &self,
        standard_device_request_from_host: StandardDeviceRequestFromHost,
        setup_packet: SetupPacket,
        client: &'a dyn Client<'a>,
    ) {
        let endpoint_index = setup_packet.get_endpoint_index();
        let buffer_index = setup_packet.get_buffer_index();
        let endpoint = self.get_endpoint(endpoint_index);

        match standard_device_request_from_host {
            StandardDeviceRequestFromHost::ClearFeature
            | StandardDeviceRequestFromHost::SetAddress
            | StandardDeviceRequestFromHost::SetConfiguration
            | StandardDeviceRequestFromHost::SetFeature
            | StandardDeviceRequestFromHost::SetInterface => {
                // All these requests don't have a data stage, so the endpoint passes directly in
                // status stage
                client.ctrl_status(endpoint_index.to_usize());
                endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Transmit(
                    TransmitCtrlEndpointState::Status,
                )));
                self.send_empty_packet(endpoint_index, buffer_index);
            }
            StandardDeviceRequestFromHost::SetDescriptor => unimplemented!(),
        }
    }

    /// Handler for standard device request
    ///
    /// + `standard_device_request`: the standard device request
    /// + `setup_packet`: the setup packet containing the request
    /// + `client`: USB client
    fn handle_standard_device_request(
        &self,
        standard_device_request: StandardDeviceRequest,
        setup_packet: SetupPacket,
        client: &'a dyn Client<'a>,
    ) {
        match standard_device_request {
            StandardDeviceRequest::ToHost(_standard_device_request_from_host) => {
                self.handle_standard_device_to_host_request(setup_packet, client)
            }
            StandardDeviceRequest::FromHost(standard_device_request_from_host) => self
                .handle_standard_device_from_host_request(
                    standard_device_request_from_host,
                    setup_packet,
                    client,
                ),
        }
    }

    /// Handler for standard request
    ///
    /// + `standard_request`: the standard request
    /// + `setup_packet`: the setup packet containing the request
    /// + `client`: USB client
    fn handle_standard_request(
        &self,
        standard_request: StandardRequest,
        setup_packet: SetupPacket,
        client: &'a dyn Client<'a>,
    ) {
        match standard_request {
            StandardRequest::Device(standard_device_request) => {
                self.handle_standard_device_request(standard_device_request, setup_packet, client)
            }
        }
    }

    /// Handler for class request with direction device to host
    ///
    /// + `setup_packet`: the setup packet containing the request
    /// + `client`: USB client
    /// + ̀`length`: the length of the data stage if any
    fn handle_class_to_host_request(
        &self,
        _setup_packet: SetupPacket,
        _client: &'a dyn Client<'a>,
        _length: Option<NonZeroU16>,
    ) {
        unimplemented!()
    }

    /// Handler for class request with direction host to device
    ///
    /// + `setup_packet`: the setup packet containing the request
    /// + `client`: USB client
    /// + ̀`length`: the length of the data stage if any
    fn handle_class_from_host_request(
        &self,
        setup_packet: SetupPacket,
        length: Option<NonZeroU16>,
    ) {
        let endpoint_index = setup_packet.get_endpoint_index();
        let buffer_index = setup_packet.get_buffer_index();
        let endpoint = self.get_endpoint(endpoint_index);

        match length {
            None => {
                self.send_empty_packet(endpoint_index, buffer_index);
                endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Transmit(
                    TransmitCtrlEndpointState::Status,
                )));
            }
            Some(_) => {
                self.free_buffer(buffer_index);
                self.fill_available_buffer_fifo();
                endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Receive(
                    ReceiveCtrlEndpointState::Data,
                )));
            }
        }
    }

    /// Handler for class request
    ///
    /// + `class_request`: the class request
    /// + `setup_packet`: the setup packet containing the request
    /// + `client`: USB client
    fn handle_class_request(
        &self,
        class_request: ClassRequest,
        setup_packet: SetupPacket,
        client: &'a dyn Client<'a>,
    ) {
        match class_request {
            ClassRequest::ToHost(length) => {
                self.handle_class_to_host_request(setup_packet, client, length)
            }
            ClassRequest::FromHost(length) => {
                self.handle_class_from_host_request(setup_packet, length)
            }
        }
    }

    /// Handler for a SETUP packet successfully received
    ///
    /// This method copies the received data to the endpoint's out buffer, informs the client about
    /// the receive of a SETUP packet and performs the indicated action.
    ///
    /// # Parameters
    ///
    /// + `setup_packet`: the SETUP packet that has been received
    /// + `endpoint`: the endpoint that received the SETUP packet
    fn handle_valid_setup_packet(&self, setup_packet: SetupPacket, endpoint: &Endpoint<'a>) {
        let buffer_index = setup_packet.get_buffer_index();
        let endpoint_out_buffer = endpoint.get_buffer_out();
        endpoint_out_buffer.map(|buffer_out| {
            self.copy_from_hardware_to_user(buffer_index, buffer_out);
            let request = match Request::try_from_packet(buffer_out) {
                Ok(request) => request,
                Err(error) => panic!(
                    "Error while decoding the USB request: {:?} {:?}",
                    error,
                    buffer_out.get(1).unwrap().get()
                ),
            };

            self.client.map(|client| {
                let endpoint_index = setup_packet.get_endpoint_index();
                match client.ctrl_setup(endpoint_index.to_usize()) {
                    CtrlSetupResult::Ok => match request {
                        Request::Standard(standard_request) => {
                            self.handle_standard_request(standard_request, setup_packet, client)
                        }
                        Request::Class(class_request) => {
                            self.handle_class_request(class_request, setup_packet, client)
                        }
                    },
                    CtrlSetupResult::OkSetAddress => {
                        // There is no data stage, so the endpoint passes in status stage. Also,
                        // when the data stage is missing, the status stage is from device to host
                        // as specified by section 9.4.6 in USB2.0 specification.
                        client.ctrl_status(endpoint_index.to_usize());
                        endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Transmit(
                            TransmitCtrlEndpointState::Status,
                        )));
                        self.send_empty_packet(endpoint_index, buffer_index);
                    }
                    // In case of an error, don't do anything and wait for another SETUP packet.
                    _ => {}
                }
            })
        });
    }

    /// Handler for a control SETUP packet that was expected to be received.
    ///
    /// If the controller expected the SETUP packet on the given endpoint, it handles it, otherwise
    /// tries receiving another packet.
    ///
    /// # Parameters
    ///
    /// + ̀`setup_packet`: the SETUP packet that has been received
    /// + ̀`receive_ctrl_state`: endpoint's state indicating the expected packet to be received
    /// + `endpoint`: the endpoint that received the SETUP packet
    fn handle_control_receive_setup_packet(
        &self,
        setup_packet: SetupPacket,
        receive_ctrl_state: ReceiveCtrlEndpointState,
        endpoint: &Endpoint<'a>,
    ) {
        match receive_ctrl_state {
            ReceiveCtrlEndpointState::Setup => {
                self.handle_valid_setup_packet(setup_packet, endpoint)
            }
            ReceiveCtrlEndpointState::Data => todo!("Retry receiving a packet"),
            ReceiveCtrlEndpointState::Status => todo!("Retry receiving a packet"),
        }
    }

    /// Handler for a control SETUP packet.
    ///
    /// If the controller expected to receive a packet, it tries handling the packet, otherwise, it
    /// waits for another packet.
    ///
    /// # Parameters
    ///
    /// + `setup_packet`: the SETUP packet that has been received
    /// + `ctrl_state`: endpoint's state indicating whether it was waiting for a packet or not
    /// + ̀`endpoint`: the endpoint that received the SETUP packet
    fn handle_control_setup_packet(
        &self,
        setup_packet: SetupPacket,
        ctrl_state: CtrlEndpointState,
        endpoint: &Endpoint<'a>,
    ) {
        match ctrl_state {
            CtrlEndpointState::Receive(receive_ctrl_state) => {
                self.handle_control_receive_setup_packet(setup_packet, receive_ctrl_state, endpoint)
            }
            CtrlEndpointState::Transmit(_transmit_ctrl_endpoint_state) => {
                todo!("Retry receiving a packet")
            }
        }
    }

    /// Handler for SETUP packet
    ///
    /// # Parameters
    ///
    /// + `setup_packet`: the SETUP packet that has been received
    fn handle_setup_packet(&self, setup_packet: SetupPacket) {
        let endpoint_index = setup_packet.get_endpoint_index();
        let endpoint = self.get_endpoint(endpoint_index);
        let endpoint_state = endpoint.get_state();

        match endpoint_state {
            EndpointState::Ctrl(ctrl_state) => {
                self.handle_control_setup_packet(setup_packet, ctrl_state, endpoint)
            }
            state @ EndpointState::Bulk | state @ EndpointState::Interrupt | state @ EndpointState::Isochronous => unreachable!("SETUP packet received on {:?} endpoint. This is an implementation bug, due to incorrect configuration in endpoint_out_enable()", state),
        }
    }

    /// Handler for a valid OUT packet received during status stage
    ///
    /// It informs the client that the status stage completed and puts the endpoint back in a state
    /// waiting for another SETUP packet.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint that received the packet
    /// + ̀`endpoint`: the endpoint that received the packet
    fn handle_valid_status_receive_control_out_packet(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        endpoint: &Endpoint<'a>,
    ) {
        self.client.map(|client| {
            client.ctrl_status_complete(endpoint_index.to_usize());
        });

        self.free_buffer(buffer_index);
        self.fill_available_buffer_fifo();

        endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Receive(
            ReceiveCtrlEndpointState::Setup,
        )));
    }

    /// Handler for an OUT packet received during status stage
    ///
    /// If the size of the packet is 0, the driver handles, otherwise it waits for another packet.
    ///
    /// # Parameters
    ///
    /// + `out_packet`: the OUT packet that has been received
    /// + `endpoint`: the endpoint that received the packet
    fn handle_status_receive_control_out_packet(
        &self,
        out_packet: OutPacket,
        endpoint: &Endpoint<'a>,
    ) {
        let packet_size = out_packet.get_size();
        let endpoint_index = out_packet.get_endpoint_index();
        let buffer_index = out_packet.get_buffer_index();

        match packet_size.to_usize() {
            0 => self.handle_valid_status_receive_control_out_packet(
                endpoint_index,
                buffer_index,
                endpoint,
            ),
            _ => todo!("Retry receiving packet"),
        }
    }

    fn handle_ok_data_receive_control_out_packet(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        endpoint: &Endpoint<'a>,
    ) {
        endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Transmit(
            TransmitCtrlEndpointState::Status,
        )));
        self.send_empty_packet(endpoint_index, buffer_index);
    }

    fn handle_data_receive_control_out_packet(
        &self,
        out_packet: OutPacket,
        endpoint: &Endpoint<'a>,
    ) {
        let endpoint_index = out_packet.get_endpoint_index();
        let packet_size = out_packet.get_size();
        let buffer_index = out_packet.get_buffer_index();
        let endpoint_buffer_out = endpoint.get_buffer_out();

        endpoint_buffer_out.map(|buffer_out| {
            self.copy_from_hardware_to_user(buffer_index, buffer_out);
        });

        self.client.map(|client| {
            // CAST: u32 == usize on RV32I
            match client.ctrl_out(endpoint_index.to_usize(), packet_size.to_usize() as u32) {
                CtrlOutResult::Ok => self.handle_ok_data_receive_control_out_packet(
                    endpoint_index,
                    buffer_index,
                    endpoint,
                ),
                CtrlOutResult::Delay => unimplemented!(),
                CtrlOutResult::Halted => unimplemented!(),
            }
        });
    }

    /// Handler for a control OUT packet that the endpoint expected to receive.
    ///
    /// If the endpoint expected a SETUP packet, the driver discards the packet and waits for
    /// another one, ohterwise it handles the packet.
    ///
    /// # Parameters
    ///
    /// + `out_packet`: the OUT packet that has been received
    /// + `receive_ctrl_state`: the endpoint's state indicating what type of packet was expected to
    /// be received
    /// + ̀`endpoint`: the endpoint that received the packet
    fn handle_receive_control_out_packet(
        &self,
        out_packet: OutPacket,
        receive_ctrl_state: ReceiveCtrlEndpointState,
        endpoint: &Endpoint<'a>,
    ) {
        match receive_ctrl_state {
            ReceiveCtrlEndpointState::Setup => todo!("Retry receiving the packet"),
            ReceiveCtrlEndpointState::Data => {
                self.handle_data_receive_control_out_packet(out_packet, endpoint)
            }
            ReceiveCtrlEndpointState::Status => {
                self.handle_status_receive_control_out_packet(out_packet, endpoint)
            }
        }
    }

    /// Handler for a control OUT packet.
    ///
    /// If the endpoint didn't expected to receive a packet, it is discarded and the driver waits
    /// for another packet. Otherwise, it handles the packet.
    ///
    /// # Parameters
    ///
    /// + `out_packet`: the OUT packet that has been received
    /// + `ctrl_state`: the state of the control endpoint indicating whether it is waiting to
    /// receive a packet or waiting to transmit a packet
    /// + `endpoint`: the endpoint that received the packet
    fn handle_control_out_packet(
        &self,
        out_packet: OutPacket,
        ctrl_state: CtrlEndpointState,
        endpoint: &Endpoint<'a>,
    ) {
        match ctrl_state {
            CtrlEndpointState::Receive(receive_ctrl_state) => {
                self.handle_receive_control_out_packet(out_packet, receive_ctrl_state, endpoint)
            }
            CtrlEndpointState::Transmit(_) => todo!("Retry receiving the packet"),
        }
    }

    fn handle_bulk_out_packet(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        packet_size: PacketSize,
    ) {
        self.internal_endpoint_rxdisable_out(endpoint_index);
        self.client.map(|client| {
            match client.packet_out(
                TransferType::Bulk,
                endpoint_index.to_usize(),
                packet_size.to_usize() as u32,
            ) {
                OutResult::Ok => {
                    self.free_buffer(buffer_index);
                    self.fill_available_buffer_fifo();
                    self.internal_endpoint_rxenable_out(endpoint_index);
                }
                OutResult::Delay => {
                    self.free_buffer(buffer_index);
                    self.fill_available_buffer_fifo();
                }
                OutResult::Error => unimplemented!(),
            }
        });
    }

    fn handle_interrupt_out_packet(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        packet_size: PacketSize,
    ) {
        self.client.map(|client| {
            match client.packet_out(
                TransferType::Interrupt,
                endpoint_index.to_usize(),
                packet_size.to_usize() as u32,
            ) {
                OutResult::Ok => {
                    self.free_buffer(buffer_index);
                    self.fill_available_buffer_fifo();
                }
                OutResult::Delay => unimplemented!(),
                // Normally, this should delay the endpoint. However, the upper layer responds with
                // OutResult::Error only when the host misbehaves. Reproducing and testing this is
                // hard. A future patch may implement proper error handling.
                OutResult::Error => unimplemented!(),
            }
        });
    }

    fn handle_isochronous_out_packet(
        &self,
        endpoint_index: EndpointIndex,
        buffer_index: BufferIndex,
        packet_size: PacketSize,
    ) {
        self.client.map(|client| {
            match client.packet_out(
                TransferType::Isochronous,
                endpoint_index.to_usize(),
                packet_size.to_usize() as u32,
            ) {
                OutResult::Ok => {
                    self.free_buffer(buffer_index);
                    self.fill_available_buffer_fifo();
                }
                OutResult::Delay => unimplemented!(),
                // Normally, this should delay the endpoint. However, the upper layer responds with
                // OutResult::Error only when the host misbehaves. Reproducing and testing this is
                // hard. A future patch may implement proper error handling.
                OutResult::Error => unimplemented!(),
            }
        });
    }

    /// Handler for an OUT packet.
    ///
    /// # Parameters
    ///
    /// + `out_packet`: the OUT packet that has been received
    fn handle_out_packet(&self, out_packet: OutPacket) {
        let endpoint_index = out_packet.get_endpoint_index();
        let endpoint = self.get_endpoint(endpoint_index);
        let endpoint_state = endpoint.get_state();
        let packet_size = out_packet.get_size();
        let buffer_index = out_packet.get_buffer_index();
        let endpoint_out_buffer = endpoint.get_buffer_out();

        endpoint_out_buffer.map(|buffer_out| {
            self.copy_from_hardware_to_user(buffer_index, buffer_out);
        });

        match endpoint_state {
            EndpointState::Ctrl(ctrl_state) => {
                self.handle_control_out_packet(out_packet, ctrl_state, endpoint)
            }
            EndpointState::Bulk => {
                self.handle_bulk_out_packet(endpoint_index, buffer_index, packet_size)
            }
            EndpointState::Interrupt => {
                self.handle_interrupt_out_packet(endpoint_index, buffer_index, packet_size)
            }
            EndpointState::Isochronous => {
                self.handle_isochronous_out_packet(endpoint_index, buffer_index, packet_size)
            }
        }
    }

    /// Reads one entry from receive FIFO
    ///
    /// # Return value
    ///
    /// The packet received
    fn read_rx_fifo(&self) -> PacketReceived {
        let rx_fifo_content = self.registers.rxfifo.extract();
        PacketReceived::new(rx_fifo_content)
    }

    /// Checks the `in_sent` register.
    ///
    /// Gets the index of the endpoint that successfully transmitted a buffer and clears the
    /// corresponding bit in `in_sent` register.
    ///
    /// # Return value
    ///
    /// + Some: the index of the endpoint that received a packet
    /// + None: no index received a packet
    fn get_and_clear_endpoint_index_packet_sent(&self) -> Option<EndpointIndex> {
        if self.registers.in_sent[0].is_set(IN_SENT::SENT_0) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_0::SET);
            Some(EndpointIndex::Endpoint0)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_1) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_1::SET);
            Some(EndpointIndex::Endpoint1)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_2) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_2::SET);
            Some(EndpointIndex::Endpoint2)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_3) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_3::SET);
            Some(EndpointIndex::Endpoint3)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_4) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_4::SET);
            Some(EndpointIndex::Endpoint4)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_5) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_5::SET);
            Some(EndpointIndex::Endpoint5)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_6) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_6::SET);
            Some(EndpointIndex::Endpoint6)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_7) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_7::SET);
            Some(EndpointIndex::Endpoint7)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_8) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_8::SET);
            Some(EndpointIndex::Endpoint8)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_9) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_9::SET);
            Some(EndpointIndex::Endpoint9)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_10) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_10::SET);
            Some(EndpointIndex::Endpoint10)
        } else if self.registers.in_sent[0].is_set(IN_SENT::SENT_11) {
            self.registers.in_sent[0].modify(IN_SENT::SENT_11::SET);
            Some(EndpointIndex::Endpoint11)
        } else {
            None
        }
    }

    /// Returns a reference to a `configin` register
    ///
    /// # Parameters
    ///
    /// + `endpoint`: the index of the endpoint whose `configin` register must be returned
    ///
    /// # Return value
    ///
    /// A reference to the `configin` register
    fn get_configin_register(
        &self,
        endpoint_index: EndpointIndex,
    ) -> &ReadWrite<u32, CONFIGIN::Register> {
        // PANIC: EndpointIndex guarantees safe access to `configin`
        self.registers
            .configin
            .get(endpoint_index.to_usize())
            .unwrap()
    }

    /// Frees a buffer
    ///
    /// # Parameters
    ///
    /// + ̀`buffer_index`: the buffer to be freed
    fn free_buffer(&self, buffer_index: BufferIndex) {
        self.available_buffer_list.free_buffer(buffer_index);
    }

    /// Returns the transmit buffer index used by this endpoint
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose transmit buffer index must be returned
    fn get_transmit_buffer(&self, endpoint_index: EndpointIndex) -> BufferIndex {
        let configin_register = self.get_configin_register(endpoint_index);

        // PANIC: `try_from_usize()` may never panic because BUFFER_0 bitfield is 5-bit wide
        // CAST: u32 == usize on RV32I
        BufferIndex::try_from_usize(configin_register.read(CONFIGIN::BUFFER_0) as usize).unwrap()
    }

    /// Frees a buffer used for transmit
    ///
    /// # Parameters
    ///
    /// + ̀`endpoint_index`: the index of the endpoint whose IN buffer must be freed
    fn free_transmit_buffer(&self, endpoint_index: EndpointIndex) {
        let buffer_index = self.get_transmit_buffer(endpoint_index);

        self.free_buffer(buffer_index);
    }

    /// Checks if transmit is pending on the given endpoint
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the endpoint to be checked
    ///
    /// # Return value
    ///
    /// + `false`: there is no pending transmit on the given endpoint
    /// + `true`: there is a pending transmit on the given endpoint
    fn is_transmit_pending(&self, endpoint_index: EndpointIndex) -> bool {
        let configin_register = self.get_configin_register(endpoint_index);
        configin_register.is_set(CONFIGIN::RDY_0)
    }

    /// Handler for the last data control IN packet successfully transmitted.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_last_data_transmit_ctrl_in_packet(
        &self,
        endpoint_index: EndpointIndex,
        endpoint: &Endpoint<'a>,
    ) {
        self.client.map(|client| {
            client.ctrl_status(endpoint_index.to_usize());
        });

        endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Receive(
            ReceiveCtrlEndpointState::Status,
        )));

        self.free_transmit_buffer(endpoint_index);
    }

    /// Handler for non-last data control IN packet successfully transmitted.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_non_last_data_transmit_ctrl_in_packet(
        &self,
        endpoint_index: EndpointIndex,
        endpoint: &Endpoint<'a>,
    ) {
        self.client.map(|client| {
            match client.ctrl_in(endpoint_index.to_usize()) {
                CtrlInResult::Packet(size, last) => {
                    // PANIC: This panics only if the upper layer is buggy
                    let packet_size = PacketSize::try_from_usize(size).unwrap();
                    let endpoint_in_buffer = endpoint.get_buffer_in();
                    let buffer_index = self.get_transmit_buffer(endpoint_index);
                    endpoint.set_last(last);
                    endpoint_in_buffer.map(|buffer_in| {
                        self.send_packet(endpoint_index, buffer_index, packet_size, buffer_in);
                    });
                }
                CtrlInResult::Delay => unimplemented!(),
                // Currently, there is no upper layer that sends CtrlInResult::Error, as a
                // consequence this is not implemented. A future patch may add support for proper
                // error handling.
                CtrlInResult::Error => unimplemented!(),
            }
        });
    }

    /// Handler for a data control IN packet successfully transmitted.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_data_transmit_ctrl_in_packet(
        &self,
        endpoint_index: EndpointIndex,
        endpoint: &Endpoint<'a>,
    ) {
        if endpoint.get_last() {
            self.handle_last_data_transmit_ctrl_in_packet(endpoint_index, endpoint);
        } else {
            self.handle_non_last_data_transmit_ctrl_in_packet(endpoint_index, endpoint);
        }
    }

    /// Handler for a status control IN packet successfully transmitted.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_status_transmit_ctrl_in_packet(
        &self,
        endpoint_index: EndpointIndex,
        endpoint: &Endpoint<'a>,
    ) {
        self.client.map(|client| {
            client.ctrl_status_complete(endpoint_index.to_usize());
        });

        endpoint.set_state(EndpointState::Ctrl(CtrlEndpointState::Receive(
            ReceiveCtrlEndpointState::Setup,
        )));

        self.free_transmit_buffer(endpoint_index);
    }

    /// Handler for a control IN packet successfully transmitted.
    ///
    /// # Parameters
    ///
    /// + ̀̀`transmit_ctrl_endpoint_state`: the endpoint's state indicating which type of control
    /// packet has been transmitted
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_transmit_ctrl_in_packet(
        &self,
        transmit_ctrl_endpoint_state: TransmitCtrlEndpointState,
        endpoint_index: EndpointIndex,
        endpoint: &Endpoint<'a>,
    ) {
        match transmit_ctrl_endpoint_state {
            TransmitCtrlEndpointState::Data => {
                self.handle_data_transmit_ctrl_in_packet(endpoint_index, endpoint)
            }
            TransmitCtrlEndpointState::Status => {
                self.handle_status_transmit_ctrl_in_packet(endpoint_index, endpoint)
            }
        }
    }

    /// Handler for a control IN packet.
    ///
    /// If the endpoint was waiting for a packet to be transmitted, the event is handled.
    /// Otherwise, it ignores the event (this probably means a bug in upper layers).
    ///
    /// # Parameters
    ///
    /// + ̀`ctrl_endpoint_state`: the endpoint's state indicating whether the endpoint is waiting
    /// for a receive packet or waiting for a packet to be transmitted.
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_ctrl_in_packet(
        &self,
        ctrl_endpoint_state: CtrlEndpointState,
        endpoint_index: EndpointIndex,
        endpoint: &Endpoint<'a>,
    ) {
        match ctrl_endpoint_state {
            CtrlEndpointState::Receive(_) => todo!("Retry packet transmission"),
            CtrlEndpointState::Transmit(transmit_ctrl_endpoint_state) => self
                .handle_transmit_ctrl_in_packet(
                    transmit_ctrl_endpoint_state,
                    endpoint_index,
                    endpoint,
                ),
        }
    }

    /// Handler for a bulk IN packet.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    /// + `endpoint`: the endpoint whose IN buffer has been transmitted
    fn handle_bulk_in_packet(&self, endpoint_index: EndpointIndex, endpoint: &Endpoint<'a>) {
        let endpoint_state = endpoint.get_state();
        let transfer_type = Self::convert_endpoint_state_to_transfer_type(endpoint_state);
        let buffer_index = self.get_transmit_buffer(endpoint_index);

        self.client.map(|client| {
            client.packet_transmitted(endpoint_index.to_usize(), Ok(()));

            match client.packet_in(transfer_type, endpoint_index.to_usize()) {
                InResult::Packet(raw_packet_size) => {
                    let packet_size = match PacketSize::try_from_usize(raw_packet_size) {
                        Ok(packet_size) => packet_size,
                        Err(()) => todo!("Return error on invalid packet size"),
                    };
                    let endpoint_buffer_in = endpoint.get_buffer_in();

                    endpoint_buffer_in.map(|buffer_in| {
                        self.send_packet(endpoint_index, buffer_index, packet_size, buffer_in);
                    });
                }
                InResult::Delay => {
                    self.free_transmit_buffer(endpoint_index);
                }
                // Normally, this should delay the endpoint. However, the upper layer responds with
                // InResult::Error only when the host misbehaves. Reproducing and testing this is
                // hard. A future patch may implement proper error handling.
                InResult::Error => unimplemented!(),
            }
        });
    }

    fn handle_interrupt_in_packet(&self, endpoint_index: EndpointIndex) {
        self.free_transmit_buffer(endpoint_index);

        self.client.map(|client| {
            client.packet_transmitted(endpoint_index.to_usize(), Ok(()));
        });
    }

    fn handle_isochronous_in_packet(&self, endpoint_index: EndpointIndex) {
        self.free_transmit_buffer(endpoint_index);

        self.client.map(|client| {
            client.packet_transmitted(endpoint_index.to_usize(), Ok(()));
        });
    }

    /// Handler for an IN packet.
    ///
    /// # Parameters
    ///
    /// + `endpoint_index`: the index of the endpoint whose IN buffer has been transmitted
    fn handle_in_packet(&self, endpoint_index: EndpointIndex) {
        let endpoint = self.get_endpoint(endpoint_index);
        let endpoint_state = endpoint.get_state();

        match endpoint_state {
            EndpointState::Ctrl(ctrl_endpoint_state) => {
                self.handle_ctrl_in_packet(ctrl_endpoint_state, endpoint_index, endpoint)
            }
            EndpointState::Bulk => self.handle_bulk_in_packet(endpoint_index, endpoint),
            EndpointState::Interrupt => self.handle_interrupt_in_packet(endpoint_index),
            EndpointState::Isochronous => self.handle_isochronous_in_packet(endpoint_index),
        }
    }

    fn internal_endpoint_resume_in(
        &self,
        endpoint_index: EndpointIndex,
        packet_size: PacketSize,
        endpoint: &Endpoint<'a>,
    ) {
        let buffer_index = if self.is_transmit_pending(endpoint_index) {
            self.get_transmit_buffer(endpoint_index)
        } else {
            self.available_buffer_list.next_and_occupy()
        };
        let endpoint_buffer_in = endpoint.get_buffer_in();

        endpoint_buffer_in.map(|buffer_in| {
            self.send_packet(endpoint_index, buffer_index, packet_size, buffer_in);
        });
    }

    /// Clears packet received interrupt
    fn clear_packet_received_interrupt(&self) {
        self.registers.intr_state.modify(INTR::PKT_RECEIVED::SET);
    }

    /// Handler for packet received interrupt
    fn handle_packet_received_interrupt(&self) {
        while !self.registers.usbstat.is_set(USBSTAT::RX_EMPTY) {
            let packet_received = self.read_rx_fifo();

            match packet_received {
                PacketReceived::Setup(setup_packet) => self.handle_setup_packet(setup_packet),
                PacketReceived::Out(out_packet) => self.handle_out_packet(out_packet),
            }
        }

        // The interrupt must be cleared only after the receive FIFO is emptied
        self.clear_packet_received_interrupt();
    }

    /// Clears packet sent interrupt
    fn clear_packet_sent_interrupt(&self) {
        self.registers.intr_state.modify(INTR::PKT_SENT::SET);
    }

    /// Handler for packet sent interrupt
    fn handle_packet_sent_interrupt(&self) {
        while let Some(endpoint_index) = self.get_and_clear_endpoint_index_packet_sent() {
            self.handle_in_packet(endpoint_index);
        }

        // The interrupt must be cleared only after all bits in `in_sent` are cleared.
        self.clear_packet_sent_interrupt();
    }

    /// Clears disconnected interrupt
    fn clear_disconnected_interrupt(&self) {
        self.registers.intr_state.modify(INTR::DISCONNECTED::SET);
    }

    /// Handler for disconnected interrupt
    fn handle_disconnected_interrupt(&self) {
        self.clear_disconnected_interrupt();
        self.client.map(|client| client.disconnected());
    }

    /// Clears host lost interrupt
    fn clear_host_lost_interrupt(&self) {
        self.registers.intr_state.modify(INTR::HOST_LOST::SET);
    }

    /// Handler for host lost interrupt
    fn handle_host_lost_interrupt(&self) {
        self.clear_host_lost_interrupt();
        self.client.map(|client| client.host_lost());
    }

    /// Clears link reset interrupt
    fn clear_link_reset_interrupt(&self) {
        self.registers.intr_state.modify(INTR::LINK_RESET::SET);
    }

    /// Handler for link reset interrupt
    fn handle_link_reset_interrupt(&self) {
        self.clear_link_reset_interrupt();
        self.client.map(|client| client.bus_reset());
    }

    /// Clears link suspended interrupt
    fn clear_link_suspended_interrupt(&self) {
        self.registers.intr_state.modify(INTR::LINK_SUSPEND::SET);
    }

    /// Handler for link suspended interrupt
    fn handle_link_suspended_interrupt(&self) {
        self.clear_link_suspended_interrupt();
        self.client.map(|client| client.link_suspended());
    }

    /// Clears link resume interrupt
    fn clear_link_resume_interrupt(&self) {
        self.registers.intr_state.modify(INTR::LINK_RESUME::SET);
    }

    /// Handler for link resume interrupt
    fn handle_link_resume_interrupt(&self) {
        self.clear_link_resume_interrupt();
        self.client.map(|client| client.link_resume());
    }

    /// Clears link in err interrupt
    fn clear_link_in_err_interrupt(&self) {
        self.registers.intr_state.modify(INTR::LINK_IN_ERR::SET);
    }

    /// Handle for link in err interrupt
    fn handle_link_in_err_interrupt(&self) {
        self.clear_link_in_err_interrupt();
        kernel::debug!("Link in error");
    }

    /// Clears frame interrupt
    fn clear_frame_interrupt(&self) {
        self.registers.intr_state.modify(INTR::FRAME::SET);
    }

    /// Handler for frame interrupt
    fn handle_frame_interrupt(&self) {
        self.clear_frame_interrupt();
        for endpoint_index in EndpointIndexIterator::new() {
            let endpoint = self.get_endpoint(endpoint_index);
            let endpoint_state = endpoint.get_state();

            if endpoint_state == EndpointState::Interrupt
                || endpoint_state == EndpointState::Isochronous
            {
                let endpoint_buffer_in = endpoint.get_buffer_in();

                endpoint_buffer_in.map(|buffer_in| {
                    self.client.map(|client| {
                        let transfer_type = if endpoint_state == EndpointState::Interrupt {
                            TransferType::Interrupt
                        } else {
                            TransferType::Isochronous
                        };

                        match client.packet_in(transfer_type, endpoint_index.to_usize()) {
                            InResult::Packet(raw_packet_size) => {
                                let packet_size = match PacketSize::try_from_usize(raw_packet_size)
                                {
                                    Ok(packet_size) => packet_size,
                                    Err(()) => panic!("Invalid packet size {}", raw_packet_size),
                                };
                                let buffer_index = if self.is_transmit_pending(endpoint_index) {
                                    self.get_transmit_buffer(endpoint_index)
                                } else {
                                    self.available_buffer_list.next_and_occupy()
                                };

                                self.send_packet(
                                    endpoint_index,
                                    buffer_index,
                                    packet_size,
                                    buffer_in,
                                );
                            }
                            InResult::Delay => unimplemented!(),
                            InResult::Error => unimplemented!(),
                        }
                    });
                });
            }
        }
    }

    /// Clears powered interrupt
    fn clear_powered_interrupt(&self) {
        self.registers.intr_state.modify(INTR::POWERED::SET);
    }

    /// Handler for powered interrupt
    fn handle_powered_interrupt(&self) {
        self.clear_powered_interrupt();
        self.client.map(|client| client.bus_powered());
    }

    /// Clears link out error interrupt
    fn clear_link_out_err_interrupt(&self) {
        self.registers.intr_state.modify(INTR::LINK_OUT_ERR::SET);
    }

    /// Handler for link out error interrupt
    fn handle_link_out_err_interrupt(&self) {
        self.clear_link_out_err_interrupt();
        kernel::debug!("Link out error");
    }

    /// USB driver interrupt handler.
    ///
    /// # Parameters
    ///
    /// + `usb_interrupt`: the USB interrupt to be handled.
    pub fn handle_interrupt(&self, usb_interrupt: UsbInterrupt) {
        match usb_interrupt {
            UsbInterrupt::PacketReceived => self.handle_packet_received_interrupt(),
            UsbInterrupt::PacketSent => self.handle_packet_sent_interrupt(),
            UsbInterrupt::Disconnected => self.handle_disconnected_interrupt(),
            UsbInterrupt::HostLost => self.handle_host_lost_interrupt(),
            UsbInterrupt::LinkReset => self.handle_link_reset_interrupt(),
            UsbInterrupt::LinkSuspended => self.handle_link_suspended_interrupt(),
            UsbInterrupt::LinkResume => self.handle_link_resume_interrupt(),
            UsbInterrupt::AvOutEmpty => unimplemented!(),
            UsbInterrupt::RxFull => unimplemented!(),
            UsbInterrupt::AvOverflow => unimplemented!(),
            UsbInterrupt::LinkInErr => self.handle_link_in_err_interrupt(),
            UsbInterrupt::RxCrcErr => unimplemented!(),
            UsbInterrupt::RxPidErr => unimplemented!(),
            UsbInterrupt::RxBitstuffErr => unimplemented!(),
            UsbInterrupt::Frame => self.handle_frame_interrupt(),
            UsbInterrupt::Powered => self.handle_powered_interrupt(),
            UsbInterrupt::LinkOutErr => self.handle_link_out_err_interrupt(),
            UsbInterrupt::AvSetupEmpty => unimplemented!(),
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

    fn endpoint_set_in_buffer(
        &self,
        raw_endpoint_index: usize,
        buffer: &'a [VolatileCell<u8>],
    ) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return Err(usb::Error::InvalidEndpoint);
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.set_buffer_in(endpoint_index, buffer);

        Ok(())
    }

    fn endpoint_set_out_buffer(
        &self,
        raw_endpoint_index: usize,
        buffer: &'a [VolatileCell<u8>],
    ) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return Err(usb::Error::InvalidEndpoint);
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.set_buffer_out(endpoint_index, buffer);

        Ok(())
    }

    fn enable_as_device(&self, _speed: DeviceSpeed) {
        self.registers.phy_config.modify(
            PHY_CONFIG::USE_DIFF_RCVR::SET
                + PHY_CONFIG::TX_USE_D_SE0::CLEAR
                + PHY_CONFIG::EOP_SINGLE_BIT::CLEAR
                + PHY_CONFIG::PINFLIP::CLEAR
                + PHY_CONFIG::USB_REF_DISABLE::SET
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

    fn set_address(&self, address: u16) {
        // PANIC: ̀`try_from_u8()` can panic only if the upper layer attempts to set an invalid USB
        // address.
        // CAST: a USB address is 7-bit long, so the upper byte can be ignored
        let usb_address = UsbAddress::try_from_u8(address as u8).unwrap();
        self.address.set(usb_address);
    }

    fn enable_address(&self) {
        let usb_address = self.address.get();
        // CAST: size_of(u32) > size_of(u8)
        self.registers
            .usbctrl
            .modify(USBCTRL::DEVICE_ADDRESS.val(usb_address.to_u8() as u32));
    }

    fn endpoint_in_enable(
        &self,
        transfer_type: TransferType,
        raw_endpoint_index: usize,
    ) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return Err(usb::Error::InvalidEndpoint);
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_in_enable(transfer_type, endpoint_index);

        if transfer_type == TransferType::Isochronous {
            self.internal_enable_in_isochronous(endpoint_index);
        }

        Ok(())
    }

    fn endpoint_out_enable(
        &self,
        transfer_type: TransferType,
        raw_endpoint_index: usize,
    ) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return Err(usb::Error::InvalidEndpoint);
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_out_enable(transfer_type, endpoint_index);
        if transfer_type == TransferType::Control {
            self.internal_endpoint_rxenable_out(endpoint_index);
            self.internal_endpoint_rxenable_setup(endpoint_index);
        } else if transfer_type == TransferType::Isochronous {
            self.internal_enable_out_isochronous(endpoint_index);
        }

        Ok(())
    }

    fn endpoint_in_out_enable(
        &self,
        transfer_type: TransferType,
        raw_endpoint_index: usize,
    ) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => {
                return Err(usb::Error::InvalidEndpoint);
            }
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_in_enable(transfer_type, endpoint_index);
        self.internal_endpoint_out_enable(transfer_type, endpoint_index);
        self.internal_endpoint_rxenable_out(endpoint_index);
        if transfer_type == TransferType::Control {
            self.internal_endpoint_rxenable_setup(endpoint_index);
        }

        Ok(())
    }

    fn endpoint_resume_in(&self, raw_endpoint_index: usize) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Ok(endpoint_index) => endpoint_index,
            Err(()) => return Err(usb::Error::InvalidEndpoint),
        };
        let endpoint = self.get_endpoint(endpoint_index);
        let endpoint_state = endpoint.get_state();
        let transfer_type = Self::convert_endpoint_state_to_transfer_type(endpoint_state);

        self.client.map(|client| {
            match client.packet_in(transfer_type, endpoint_index.to_usize()) {
                InResult::Packet(raw_packet_size) => {
                    let packet_size = match PacketSize::try_from_usize(raw_packet_size) {
                        Ok(packet_size) => packet_size,
                        Err(()) => todo!("Return error on invalid packet size"),
                    };

                    self.internal_endpoint_resume_in(endpoint_index, packet_size, endpoint);
                }
                InResult::Delay => unimplemented!(),
                // Normally, this should delay the endpoint. However, the upper layer responds with
                // InResult::Error only when the host misbehaves. Reproducing and testing this is
                // hard. A future patch may implement proper error handling.
                InResult::Error => unimplemented!(),
            }
        });

        Ok(())
    }

    fn endpoint_resume_out(&self, raw_endpoint_index: usize) -> Result<(), usb::Error> {
        let endpoint_index = match EndpointIndex::try_from_usize(raw_endpoint_index) {
            Err(()) => return Err(usb::Error::InvalidEndpoint),
            Ok(endpoint_index) => endpoint_index,
        };

        self.internal_endpoint_rxenable_out(endpoint_index);

        Ok(())
    }
}
