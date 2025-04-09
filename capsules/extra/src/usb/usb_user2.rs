// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! System call interface for generic USB transport layer

use super::descriptors::{
    create_descriptor_buffers, ConfigurationDescriptor, DeviceDescriptor, EndpointAddress,
    EndpointDescriptor, InterfaceDescriptor, TransferDirection,
};
use super::usbc_client_ctrl::ClientCtrl;

use kernel::grant::{AllowRoCount, AllowRwCount, Grant, UpcallCount};
use kernel::hil::usb::{self, Client};
use kernel::process;
use kernel::processbuffer::{ReadableProcessBuffer, WriteableProcessBuffer};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, VolatileCell};
use kernel::ErrorCode;
use kernel::ProcessId;

use core::cell::Cell;

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::UsbUser2 as usize;

#[repr(usize)]
#[derive(Clone, Copy)]
enum UpcallId {
    Transmit = 0,
    Receive = 1,
    Attached = 2,
}

impl UpcallId {
    const fn to_usize(self) -> usize {
        // CAST: UpcallId is marked repr(usize)
        self as usize
    }
}

const UPCALL_COUNT: u8 = 3;

#[repr(usize)]
#[derive(Clone, Copy)]
enum ReadOnlyBufferId {
    Transmit = 0,
}

impl ReadOnlyBufferId {
    const fn to_usize(self) -> usize {
        // CAST: ReadOnlyBufferId is marked repr(usize)
        self as usize
    }
}

const ALLOW_RO_COUNT: u8 = 1;

#[repr(usize)]
#[derive(Clone, Copy)]
enum ReadWriteBufferId {
    Receive = 0,
}

impl ReadWriteBufferId {
    const fn to_usize(self) -> usize {
        // CAST: ReadWriteBufferId is marked repr(usize)
        self as usize
    }
}

const ALLOW_RW_COUNT: u8 = 1;

type UsbGrant = Grant<
    (),
    UpcallCount<UPCALL_COUNT>,
    AllowRoCount<ALLOW_RO_COUNT>,
    AllowRwCount<ALLOW_RW_COUNT>,
>;

// Google product that is recognized as a CDC by a Linux host machine.
// TODO: this is used for testing, but needs to be changed.
const VENDOR_ID: u16 = 0x18d1;
const PRODUCT_ID: u16 = 0x503a;

static LANGUAGES: &[u16; 1] = &[
    0x0409, // English (United States)
];

static STRINGS: &[&str] = &[
    "XYZ Corp.",      // Manufacturer
    "The Zorpinator", // Product
    "Serial No. 5",   // Serial number
];

const BULK_IN_ENDPOINT: usize = 1;
const BULK_OUT_ENDPOINT: usize = 2;

pub struct UsbClient<'a, Usb: usb::UsbController<'a>, const MAX_PACKET_SIZE: usize> {
    usb: &'a Usb,
    usb_ctrl: ClientCtrl<'a, 'static, Usb>,
    usb_syscall_driver: OptionalCell<&'a UsbSyscallDriver<'a, Usb, MAX_PACKET_SIZE>>,
    transmit_chunk: [VolatileCell<u8>; MAX_PACKET_SIZE],
    transmit_response: Cell<usb::InResult>,
    receive_chunk: [VolatileCell<u8>; MAX_PACKET_SIZE],
}

impl<'a, Usb: usb::UsbController<'a>, const MAX_PACKET_SIZE: usize>
    UsbClient<'a, Usb, MAX_PACKET_SIZE>
{
    pub fn new(usb: &'a Usb) -> Self {
        let interfaces: &mut [InterfaceDescriptor] = &mut [InterfaceDescriptor {
            interface_number: 0,
            alternate_setting: 0,
            num_endpoints: 2,      // (excluding default control endpoint)
            interface_class: 0xff, // vendor_specific
            interface_subclass: 0x50,
            interface_protocol: 1,
            string_index: 0,
        }];
        let endpoints: &[&[EndpointDescriptor]] = &mut [&[
            EndpointDescriptor {
                endpoint_address: EndpointAddress::new_const(
                    BULK_IN_ENDPOINT,
                    TransferDirection::DeviceToHost,
                ),
                transfer_type: usb::TransferType::Bulk,
                max_packet_size: MAX_PACKET_SIZE as u16,
                interval: 0,
            },
            EndpointDescriptor {
                endpoint_address: EndpointAddress::new_const(
                    BULK_OUT_ENDPOINT,
                    TransferDirection::HostToDevice,
                ),
                transfer_type: usb::TransferType::Bulk,
                max_packet_size: MAX_PACKET_SIZE as u16,
                interval: 0,
            },
        ]];

        let (device_descriptor_buffer, other_descriptor_buffer) = create_descriptor_buffers(
            DeviceDescriptor {
                vendor_id: VENDOR_ID,
                product_id: PRODUCT_ID,
                manufacturer_string: 1,
                product_string: 2,
                serial_number_string: 3,
                max_packet_size_ep0: MAX_PACKET_SIZE as u8,
                ..DeviceDescriptor::default()
            },
            ConfigurationDescriptor::default(),
            interfaces,
            endpoints,
            None,
            None,
        );

        const DEFAULT_VOLATILE_CELL: VolatileCell<u8> = VolatileCell::new(0);

        Self {
            usb,
            usb_ctrl: ClientCtrl::new(
                usb,
                device_descriptor_buffer,
                other_descriptor_buffer,
                None,
                None,
                LANGUAGES,
                STRINGS,
            ),
            usb_syscall_driver: OptionalCell::empty(),
            transmit_chunk: [DEFAULT_VOLATILE_CELL; MAX_PACKET_SIZE],
            transmit_response: Cell::new(usb::InResult::Delay),
            receive_chunk: [DEFAULT_VOLATILE_CELL; MAX_PACKET_SIZE],
        }
    }

    fn set_usb_syscall_driver(
        &self,
        usb_syscall_driver: &'a UsbSyscallDriver<'a, Usb, MAX_PACKET_SIZE>,
    ) {
        self.usb_syscall_driver.set(usb_syscall_driver);
    }

    fn start_transmission(&self) {
        self.usb_syscall_driver.map(|usb_syscall_driver| {
            let packet = &self.transmit_chunk[..];
            let in_result = usb_syscall_driver.fill_packet(BULK_IN_ENDPOINT, packet);
            self.transmit_response.set(in_result);
            self.usb.endpoint_resume_in(BULK_IN_ENDPOINT).unwrap();
        });
    }

    fn start_reception(&self) {
        self.usb.endpoint_resume_out(BULK_OUT_ENDPOINT).unwrap();
    }
}

pub struct UsbSyscallDriver<'a, Usb: usb::UsbController<'a>, const MAX_PACKET_SIZE: usize> {
    usb_client: &'a UsbClient<'a, Usb, MAX_PACKET_SIZE>,
    grant: UsbGrant,
    current_owner: OptionalCell<ProcessId>,
    transmit_length: Cell<usize>,
    transmit_position: Cell<usize>,
    receive_length: Cell<usize>,
    receive_position: Cell<usize>,
    usb_attached: Cell<bool>,
}

impl<'a, Usb: usb::UsbController<'a>, const MAX_PACKET_SIZE: usize>
    UsbSyscallDriver<'a, Usb, MAX_PACKET_SIZE>
{
    pub fn new(usb_client: &'a UsbClient<'a, Usb, MAX_PACKET_SIZE>, grant: UsbGrant) -> Self {
        Self {
            usb_client,
            grant,
            current_owner: OptionalCell::empty(),
            transmit_length: Cell::new(0),
            transmit_position: Cell::new(0),
            receive_length: Cell::new(0),
            receive_position: Cell::new(0),
            usb_attached: Cell::new(false),
        }
    }

    pub fn init(&'a self) {
        self.usb_client.set_usb_syscall_driver(self);
    }

    fn enable_command(&self, process_id: ProcessId) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::RESERVE);
        }

        self.usb_client.enable();
        CommandReturn::success()
    }

    fn attach_command(&self, process_id: ProcessId) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::RESERVE);
        }

        if self.usb_attached.get() {
            self.notify_attach(process_id);
        } else {
            self.usb_client.attach();
        }

        CommandReturn::success()
    }

    fn is_owner(&self, process_id: ProcessId) -> bool {
        match self.get_owner() {
            None => false,
            Some(other_process_id) => {
                if let Err(process::Error::NoSuchApp) = self.grant.enter(process_id, |_, _| {}) {
                    return false;
                }

                other_process_id == process_id
            }
        }
    }

    fn update_owner(&self, owner: ProcessId) {
        self.current_owner.replace(owner);
    }

    fn clear_owner(&self) {
        self.current_owner.clear();
    }

    fn get_owner(&self) -> Option<ProcessId> {
        self.current_owner.get()
    }

    fn lock_command(&self, process_id: ProcessId) -> CommandReturn {
        match self.get_owner() {
            None => {
                self.update_owner(process_id);
                CommandReturn::success()
            }
            Some(other_process_id) => {
                if other_process_id == process_id {
                    CommandReturn::failure(ErrorCode::ALREADY)
                } else {
                    CommandReturn::failure(ErrorCode::BUSY)
                }
            }
        }
    }

    fn unlock_command(&self, process_id: ProcessId) -> CommandReturn {
        if self.is_owner(process_id) {
            self.clear_owner();
            CommandReturn::success()
        } else {
            CommandReturn::failure(ErrorCode::RESERVE)
        }
    }

    fn transmit_failed(&self) {
        let owner = match self.get_owner() {
            None => unreachable!("Transmit command checks for owner"),
            Some(owner) => owner,
        };

        self.grant
            .enter(owner, |_, kernel_data| {
                let transmit_length = self.transmit_length.get();
                // The capsule can't do anything if the upcall fails to be scheduled, so the
                // result is ignored.
                let _ = kernel_data
                    .schedule_upcall(UpcallId::Transmit.to_usize(), (transmit_length, 0, 0));
            })
            .unwrap();
    }

    fn transmit_command(&self, process_id: ProcessId) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::RESERVE);
        }

        let length = self
            .grant
            .enter(process_id, |_, kernel_data| {
                match kernel_data.get_readonly_processbuffer(ReadOnlyBufferId::Transmit.to_usize())
                {
                    Ok(buffer) => buffer.len(),
                    Err(_) => 0,
                }
            })
            .unwrap_or(0);

        if length == 0 {
            return CommandReturn::failure(ErrorCode::NOMEM);
        }

        self.transmit_position.set(0);
        self.transmit_length.set(length);

        self.usb_client.start_transmission();

        CommandReturn::success()
    }

    fn internal_fill_packet(&self, packet: &[VolatileCell<u8>]) -> usb::InResult {
        if self.transmit_length.get() == 0 {
            return usb::InResult::Delay;
        }

        let owner = match self.get_owner() {
            None => return usb::InResult::Delay,
            Some(owner) => owner,
        };

        self.grant
            .enter(owner, |_, kernel_data| {
                kernel_data
                    .get_readonly_processbuffer(ReadOnlyBufferId::Transmit.to_usize())
                    .map(|buffer| {
                        let mut transmit_position = self.transmit_position.get();
                        let transmit_length = self.transmit_length.get();
                        let copy_length =
                            core::cmp::min(MAX_PACKET_SIZE, transmit_length - transmit_position);
                        if copy_length == 0 {
                            // No more data left to be transmitted
                            // The capsule can't do anything if the upcall fails to be scheduled, so the
                            // result is ignored.
                            let _ = kernel_data.schedule_upcall(
                                UpcallId::Transmit.to_usize(),
                                (transmit_length, 0, 0),
                            );
                            return usb::InResult::Delay;
                        }

                        if let Err(_) = buffer.enter(|buffer| {
                            for index in 0..copy_length {
                                let byte = buffer[transmit_position + index].get();
                                packet[index].set(byte);
                            }
                        }) {
                            usb::InResult::Error
                        } else {
                            transmit_position += copy_length;
                            self.transmit_position.set(transmit_position);
                            usb::InResult::Packet(copy_length)
                        }
                    })
                    .unwrap_or(usb::InResult::Error)
            })
            .unwrap_or(usb::InResult::Error)
    }

    fn fill_packet(&self, endpoint: usize, packet: &[VolatileCell<u8>]) -> usb::InResult {
        if endpoint != BULK_IN_ENDPOINT {
            return usb::InResult::Delay;
        }

        self.internal_fill_packet(packet)
    }

    fn handle_packet_received_bulk(
        &self,
        endpoint: usize,
        packet: &[VolatileCell<u8>],
    ) -> usb::OutResult {
        if endpoint != BULK_OUT_ENDPOINT {
            return usb::OutResult::Delay;
        }

        if self.receive_length.get() == 0 {
            return usb::OutResult::Delay;
        }

        let owner = match self.get_owner() {
            None => return usb::OutResult::Delay,
            Some(owner) => owner,
        };

        self.grant
            .enter(owner, |_, kernel_data| {
                kernel_data
                    .get_readwrite_processbuffer(ReadWriteBufferId::Receive.to_usize())
                    .map(|buffer| {
                        let mut receive_position = self.receive_position.get();
                        let receive_length = self.receive_length.get();
                        let packet_length = packet.len();
                        let copy_length =
                            core::cmp::min(packet_length, receive_length - receive_position);
                        if let Err(_) = buffer.mut_enter(|buffer| {
                            for index in 0..copy_length {
                                let byte = packet[index].get();
                                buffer[receive_position + index].set(byte);
                            }
                        }) {
                            usb::OutResult::Error
                        } else {
                            receive_position += copy_length;
                            if receive_position == receive_length {
                                self.receive_length.set(0);
                                // The capsule can't do anything if the upcall fails to be scheduled, so the
                                // result is ignored.
                                let _ = kernel_data.schedule_upcall(
                                    UpcallId::Receive.to_usize(),
                                    (receive_length, 0, 0),
                                );
                                usb::OutResult::Delay
                            } else {
                                self.receive_position.set(receive_position);
                                usb::OutResult::Ok
                            }
                        }
                    })
                    .unwrap_or(usb::OutResult::Error)
            })
            .unwrap_or(usb::OutResult::Error)
    }

    fn packet_received(
        &self,
        transfer_type: usb::TransferType,
        endpoint: usize,
        packet: &[VolatileCell<u8>],
    ) -> usb::OutResult {
        match transfer_type {
            usb::TransferType::Control => unreachable!(
                "The peripheral never invokes packet_out() when a control packet is received"
            ),
            usb::TransferType::Bulk => self.handle_packet_received_bulk(endpoint, packet),
            usb::TransferType::Interrupt => unimplemented!(),
            usb::TransferType::Isochronous => unimplemented!(),
        }
    }

    fn receive_command(&self, process_id: ProcessId) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::RESERVE);
        }

        let length = self
            .grant
            .enter(process_id, |_, kernel_data| {
                match kernel_data.get_readwrite_processbuffer(ReadWriteBufferId::Receive.to_usize())
                {
                    Ok(buffer) => buffer.len(),
                    Err(_) => 0,
                }
            })
            .unwrap_or(0);

        if length == 0 {
            return CommandReturn::failure(ErrorCode::NOMEM);
        }

        self.receive_position.set(0);
        self.receive_length.set(length);

        self.usb_client.start_reception();

        CommandReturn::success()
    }

    fn bus_powered(&self) {
        let owner = match self.get_owner() {
            None => return,
            Some(owner) => owner,
        };

        self.usb_attached.set(true);

        self.notify_attach(owner);
    }

    fn notify_attach(&self, process_id: ProcessId) {
        let _ = self.grant.enter(process_id, |_, kernel_data| {
            // The capsule can't do anything if the upcall fails to be scheduled, so the
            // result is ignored.
            let _ = kernel_data.schedule_upcall(UpcallId::Attached.to_usize(), (0, 0, 0));
        });
    }
}

enum Command {
    DriverExists = 0,
    Enable = 1,
    Attach = 2,
    Lock = 3,
    Unlock = 4,
    Transmit = 5,
    Receive = 6,
}

impl Command {
    const fn new(command_number: usize) -> Result<Self, ()> {
        const DRIVER_EXISTS_NUMBER: usize = Command::DriverExists as usize;
        const ENABLE_NUMBER: usize = Command::Enable as usize;
        const ATTACH_NUMBER: usize = Command::Attach as usize;
        const LOCK_NUMBER: usize = Command::Lock as usize;
        const UNLOCK_NUMBER: usize = Command::Unlock as usize;
        const TRANSMIT_NUMBER: usize = Command::Transmit as usize;
        const RECEIVE_NUMBER: usize = Command::Receive as usize;
        match command_number {
            DRIVER_EXISTS_NUMBER => Ok(Command::DriverExists),
            ENABLE_NUMBER => Ok(Command::Enable),
            ATTACH_NUMBER => Ok(Command::Attach),
            LOCK_NUMBER => Ok(Command::Lock),
            UNLOCK_NUMBER => Ok(Command::Unlock),
            TRANSMIT_NUMBER => Ok(Command::Transmit),
            RECEIVE_NUMBER => Ok(Command::Receive),
            _ => Err(()),
        }
    }
}

impl<'a, Usb: usb::UsbController<'a>, const MAX_PACKET_SIZE: usize> usb::Client<'a>
    for UsbClient<'a, Usb, MAX_PACKET_SIZE>
{
    fn enable(&'a self) {
        self.usb_ctrl.enable();

        // IN endpoint
        self.usb
            .endpoint_in_enable(usb::TransferType::Bulk, BULK_IN_ENDPOINT)
            .unwrap();
        self.usb
            .endpoint_set_in_buffer(BULK_IN_ENDPOINT, &self.transmit_chunk)
            .unwrap();

        // OUT endpoint
        self.usb
            .endpoint_out_enable(usb::TransferType::Bulk, BULK_OUT_ENDPOINT)
            .unwrap();
        self.usb
            .endpoint_set_out_buffer(BULK_OUT_ENDPOINT, &self.receive_chunk)
            .unwrap();
    }

    fn attach(&'a self) {
        self.usb_ctrl.attach();
    }

    fn bus_reset(&'a self) {
        kernel::debug!("Bus reset");
    }

    fn link_suspended(&'a self) {
        kernel::debug!("Link suspended");
    }

    fn link_resume(&'a self) {
        kernel::debug!("Link resumed");
    }

    fn disconnected(&'a self) {
        kernel::debug!("Disconnected");
    }

    fn host_lost(&'a self) {
        kernel::debug!("Host lost");
    }

    fn bus_powered(&'a self) {
        kernel::debug!("Bus powered");
        self.usb_syscall_driver
            .map(|usb_syscall_driver| usb_syscall_driver.bus_powered());
    }

    fn ctrl_setup(&'a self, endpoint: usize) -> usb::CtrlSetupResult {
        self.usb_ctrl.ctrl_setup(endpoint)
    }

    fn ctrl_in(&'a self, endpoint: usize) -> usb::CtrlInResult {
        self.usb_ctrl.ctrl_in(endpoint)
    }

    fn ctrl_out(&'a self, endpoint: usize, packet_bytes: u32) -> usb::CtrlOutResult {
        self.usb_ctrl.ctrl_out(endpoint, packet_bytes)
    }

    fn ctrl_status(&'a self, endpoint: usize) {
        self.usb_ctrl.ctrl_status(endpoint)
    }

    fn ctrl_status_complete(&'a self, endpoint: usize) {
        self.usb_ctrl.ctrl_status_complete(endpoint)
    }

    fn packet_in(&'a self, _transfer_type: usb::TransferType, endpoint: usize) -> usb::InResult {
        if endpoint != BULK_IN_ENDPOINT {
            usb::InResult::Delay
        } else {
            self.transmit_response.get()
        }
    }

    fn packet_out(
        &'a self,
        transfer_type: usb::TransferType,
        endpoint: usize,
        packet_bytes: u32,
    ) -> usb::OutResult {
        let length = packet_bytes as usize;
        let packet = &self.receive_chunk[..length];
        self.usb_syscall_driver
            .map_or(usb::OutResult::Delay, |usb_syscall_driver| {
                usb_syscall_driver.packet_received(transfer_type, endpoint, packet)
            })
    }

    fn packet_transmitted(&'a self, endpoint: usize, result: Result<(), ()>) {
        self.usb_syscall_driver
            .map(|usb_syscall_driver| match result {
                Err(()) => usb_syscall_driver.transmit_failed(),
                Ok(()) => {
                    let packet = &self.transmit_chunk[..];
                    let in_result = usb_syscall_driver.fill_packet(endpoint, packet);
                    self.transmit_response.set(in_result);
                }
            });
    }
}

impl<'a, Usb: usb::UsbController<'a>, const MAX_PACKET_SIZE: usize> SyscallDriver
    for UsbSyscallDriver<'a, Usb, MAX_PACKET_SIZE>
{
    fn command(
        &self,
        command_number: usize,
        _argument1: usize,
        _argument2: usize,
        process_id: ProcessId,
    ) -> CommandReturn {
        let command = match Command::new(command_number) {
            Ok(command) => command,
            Err(()) => return CommandReturn::failure(ErrorCode::NOSUPPORT),
        };

        match command {
            Command::DriverExists => CommandReturn::success(),
            Command::Enable => self.enable_command(process_id),
            Command::Attach => self.attach_command(process_id),
            Command::Lock => self.lock_command(process_id),
            Command::Unlock => self.unlock_command(process_id),
            Command::Transmit => self.transmit_command(process_id),
            Command::Receive => self.receive_command(process_id),
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(process_id, |_, _| {})
    }
}
