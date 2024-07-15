// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! System call interface for generic USB transport layer

use super::descriptors::{
    create_descriptor_buffers,
    ConfigurationDescriptor,
    DeviceDescriptor,
    EndpointAddress,
    EndpointDescriptor,
    InterfaceDescriptor,
    TransferDirection,
};
use super::usbc_client_ctrl::ClientCtrl;

use kernel::ErrorCode;
use kernel::ProcessId;
use kernel::grant::{Grant, AllowRoCount, AllowRwCount, UpcallCount};
use kernel::hil::usb::{self, Client};
use kernel::process;
use kernel::processbuffer::{ReadOnlyProcessBufferRef, ReadableProcessBuffer, WriteableProcessBuffer};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, VolatileCell};

use core::cell::Cell;

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::UsbUser2 as usize;

#[derive(Default)]
pub struct AppData {

}

#[repr(usize)]
enum UpcallId {
    Transmit = 0,
    Receive = 1,
}

impl UpcallId {
    const fn to_usize(self) -> usize {
        // CAST: UpcallId is marked repr(usize)
        self as usize
    }
}

const UPCALL_COUNT: u8 = 2;

#[repr(usize)]
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
    AppData,
    UpcallCount<UPCALL_COUNT>,
    AllowRoCount<ALLOW_RO_COUNT>,
    AllowRwCount<ALLOW_RW_COUNT>,
>;

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

pub struct UsbClient<'a, Usb: usb::UsbController<'a>> {
    usb: &'a Usb,
    usb_ctrl: ClientCtrl<'a, 'static, Usb>,
    usb_syscall_driver: OptionalCell<&'a UsbSyscallDriver<'a, Usb>>,
    // TODO: Remove hard constant
    transmit_chunk: [VolatileCell<u8>; 64],
    transmit_length: Cell<usize>,
    // TODO: Remove hard constant
    receive_chunk: [VolatileCell<u8>; 64],
}

impl<'a, Usb: usb::UsbController<'a>> UsbClient<'a, Usb> {
    pub fn new(usb: &'a Usb) -> Self {
        let interfaces: &mut [InterfaceDescriptor] =
            &mut [InterfaceDescriptor {
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
                endpoint_address: EndpointAddress::new_const(BULK_IN_ENDPOINT, TransferDirection::DeviceToHost),
                transfer_type: usb::TransferType::Bulk,
                // TODO: Remove hard constant
                max_packet_size: 64,
                interval: 0,
            },
            EndpointDescriptor {
                endpoint_address: EndpointAddress::new_const(BULK_OUT_ENDPOINT, TransferDirection::HostToDevice),
                transfer_type: usb::TransferType::Bulk,
                // TODO: Remove hard constant
                max_packet_size: 64,
                interval: 0,
            },
        ]];

        let (device_descriptor_buffer, other_descriptor_buffer) =
            create_descriptor_buffers(
                DeviceDescriptor {
                    vendor_id: VENDOR_ID,
                    product_id: PRODUCT_ID,
                    manufacturer_string: 1,
                    product_string: 2,
                    serial_number_string: 3,
                    // TODO: Remove hard constant
                    max_packet_size_ep0: 64,
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
            // TODO: Remove hard coded value
            transmit_chunk: [DEFAULT_VOLATILE_CELL; 64],
            transmit_length: Cell::new(0),
            receive_chunk: [DEFAULT_VOLATILE_CELL; 64],
        }
    }

    fn set_usb_syscall_driver(&self, usb_syscall_driver: &'a UsbSyscallDriver<'a, Usb>) {
        self.usb_syscall_driver.set(usb_syscall_driver);
    }

    pub fn transmit_chunk(
        &self,
        buffer: ReadOnlyProcessBufferRef,
        start: usize,
        end: usize,
    ) -> Result<(), process::Error> {
        match buffer.enter(|buffer| {
            for (index, byte) in buffer[start..end].iter().enumerate() {
                self.transmit_chunk[index].set(byte.get());
            }
            self.transmit_length.set(end - start);
            self.usb.endpoint_resume_in(BULK_IN_ENDPOINT).unwrap();
        }) {
            Ok(_) => Ok(()),
            Err(_) => Err(process::Error::KernelError)
        }
    }
}

pub struct UsbSyscallDriver<
    'a,
    Usb: usb::UsbController<'a>,
> {
    usb_client: &'a UsbClient<'a, Usb>,
    grant: UsbGrant,
    current_owner: Cell<Option<ProcessId>>,
    transmit_length: Cell<usize>,
    transmit_position: Cell<usize>,
    receive_length: Cell<usize>,
    receive_position: Cell<usize>,
}

impl<'a, Usb: usb::UsbController<'a>> UsbSyscallDriver<'a, Usb> {
    pub fn new(usb_client: &'a UsbClient<'a, Usb>, grant: UsbGrant) -> Self {
        Self {
            usb_client,
            grant,
            current_owner: Cell::new(None),
            transmit_length: Cell::new(0),
            transmit_position: Cell::new(0),
            receive_length: Cell::new(0),
            receive_position: Cell::new(0),
        }
    }

    pub fn init(&'a self) {
        self.usb_client.set_usb_syscall_driver(self);
    }

    fn enable_command(&self) -> CommandReturn {
        self.usb_client.enable();
        CommandReturn::success()
    }

    fn attach_command(&self) -> CommandReturn {
        self.usb_client.attach();
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
        self.current_owner.set(Some(owner));
    }

    fn clear_owner(&self) {
        self.current_owner.set(None);
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
            CommandReturn::failure(ErrorCode::BUSY)
        }
    }

    fn transmit_chunk(&self) -> Result<(), ()> {
        let owner = match self.get_owner() {
            None => return Err(()),
            Some(owner) => owner,
        };

        let transmit_length = self.transmit_length.get();

        if transmit_length == 0 {
            return Err(());
        }

        self.grant.enter(owner, |_, kernel_data| {
            kernel_data.get_readonly_processbuffer(ReadOnlyBufferId::Transmit.to_usize()).and_then(|buffer| {
                let start = self.transmit_position.get();
                if transmit_length == start && transmit_length != 0 {
                    // The capsule can't do anything if the upcall fails to be scheduled, so the
                    // result is ignored.
                    let _ = kernel_data.schedule_upcall(UpcallId::Transmit.to_usize(), (transmit_length, 0, 0));
                    self.transmit_length.set(0);

                    Ok(())
                } else {
                    // TODO: Remove hard coded value
                    let end = core::cmp::min(start + 64, transmit_length);
                    self.transmit_position.set(end);
                    self.usb_client.transmit_chunk(buffer, start, end)
                }
            })
        }).map(|result: Result<(), process::Error>| result.map_err(|_| ())).unwrap_or(Err(()))
    }

    fn transmit_failed(&self) {
        let owner = match self.get_owner() {
            None => unreachable!("Transmit command checks for owner"),
            Some(owner) => owner,
        };

        self.grant.enter(owner, |_, kernel_data| {
            let transmit_length = self.transmit_length.get();
            // The capsule can't do anything if the upcall fails to be scheduled, so the
            // result is ignored.
            let _ = kernel_data.schedule_upcall(UpcallId::Transmit.to_usize(), (transmit_length, 0, 0));
        }).unwrap();
    }

    fn transmit_command(&self, process_id: ProcessId) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::BUSY);
        }

        let length = self.grant.enter(process_id, |_, kernel_data| {
            match kernel_data.get_readonly_processbuffer(ReadOnlyBufferId::Transmit.to_usize()) {
                Ok(buffer) => buffer.len(),
                Err(_) => 0,
            }
        }).unwrap_or(0);

        if length == 0 {
            return CommandReturn::failure(ErrorCode::NOMEM);
        }

        self.transmit_position.set(0);
        self.transmit_length.set(length);

        if let Err(()) = self.transmit_chunk() {
            return CommandReturn::failure(ErrorCode::FAIL);
        }

        CommandReturn::success()
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
            return usb::OutResult::Ok;
        }

        let owner = match self.get_owner() {
            None => return usb::OutResult::Ok,
            Some(owner) => owner,
        };

        self.grant.enter(owner, |_, kernel_data| {
            kernel_data
                .get_readwrite_processbuffer(ReadWriteBufferId::Receive.to_usize())
                .and_then(|buffer| {
                    let mut receive_position = self.receive_position.get();
                    let receive_length = self.receive_length.get();
                    let packet_length = packet.len();
                    let copy_length = core::cmp::min(packet_length, receive_length - receive_position);
                    if let Err(_) = buffer.mut_enter(|buffer| {
                        for index in 0..copy_length {
                            let byte = packet[index].get();
                            buffer[receive_position + index].set(byte);
                        }
                    }) {
                        Ok(usb::OutResult::Error)
                    } else {
                        receive_position += copy_length;
                        if receive_position == receive_length {
                            self.receive_length.set(0);
                            // The capsule can't do anything if the upcall fails to be scheduled, so the
                            // result is ignored.
                            let _ = kernel_data.schedule_upcall(UpcallId::Receive.to_usize(), (receive_length, 0, 0));
                        } else {
                            self.receive_position.set(receive_position);
                        }
                        Ok(usb::OutResult::Ok)
                    }
                }).unwrap_or(usb::OutResult::Error)
        }).unwrap_or(usb::OutResult::Error)
    }

    fn packet_received(
        &self,
        transfer_type: usb::TransferType,
        endpoint: usize,
        packet: &[VolatileCell<u8>],
    ) -> usb::OutResult {
        match transfer_type {
            usb::TransferType::Control =>
                unreachable!("The peripheral never invokes packet_out() when a control packet is received"),
            // CAST: Tock is not supposed to run 16-bit platforms, so usize is always at least as
            // wide as a u32
            usb::TransferType::Bulk => self.handle_packet_received_bulk(endpoint, packet),
            usb::TransferType::Interrupt => unimplemented!(),
            usb::TransferType::Isochronous => unimplemented!(),
        }
    }

    fn receive_command(&self, process_id: ProcessId) -> CommandReturn {
        if !self.is_owner(process_id) {
            return CommandReturn::failure(ErrorCode::BUSY);
        }

        let length = self.grant.enter(process_id, |_, kernel_data| {
            match kernel_data.get_readwrite_processbuffer(ReadWriteBufferId::Receive.to_usize()) {
                Ok(buffer) => buffer.len(),
                Err(_) => 0,
            }
        }).unwrap_or(0);

        if length == 0 {
            return CommandReturn::failure(ErrorCode::NOMEM);
        }

        self.receive_position.set(0);
        self.receive_length.set(length);

        CommandReturn::success()
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

impl<'a, Usb: usb::UsbController<'a>> usb::Client<'a> for UsbClient<'a, Usb> {
    fn enable(&'a self) {
        self.usb_ctrl.enable();

        // IN endpoint
        self.usb.endpoint_in_enable(usb::TransferType::Bulk, BULK_IN_ENDPOINT).unwrap();
        self.usb.endpoint_set_in_buffer(BULK_IN_ENDPOINT, &self.transmit_chunk).unwrap();

        // OUT endpoint
        self.usb.endpoint_out_enable(usb::TransferType::Bulk, BULK_OUT_ENDPOINT).unwrap();
        self.usb.endpoint_set_out_buffer(BULK_OUT_ENDPOINT, &self.receive_chunk).unwrap();
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
        match endpoint {
            BULK_IN_ENDPOINT => {
                let transmit_length = self.transmit_length.get();
                self.transmit_length.set(0);
                usb::InResult::Packet(transmit_length)
            }
            _ => usb::InResult::Delay,
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
        self.usb_syscall_driver.map_or(usb::OutResult::Delay, |usb_syscall_driver|
            usb_syscall_driver.packet_received(transfer_type, endpoint, packet)
        )
    }

    fn packet_transmitted(&'a self, endpoint: usize, result: Result<(), ()>) {
        self.usb_syscall_driver.map(|usb_syscall_driver| {
            match result {
                Err(()) => usb_syscall_driver.transmit_failed(),
                Ok(()) => match endpoint {
                    BULK_IN_ENDPOINT => {
                        // The process may have terminated before the transmit ended. In this case,
                        // the capsule can't do anything about it, so the result is simply ignored.
                        let _ = usb_syscall_driver.transmit_chunk();
                    }
                    _ => unreachable!(),
                }
            }
        });
    }
}

impl<'a, Usb: usb::UsbController<'a>> SyscallDriver for UsbSyscallDriver<'a, Usb> {
    fn command(
        &self,
        command_number: usize,
        _argument1: usize,
        _argument2: usize,
        process_id: ProcessId
    ) -> CommandReturn {
        let command = match Command::new(command_number) {
            Ok(command) => command,
            Err(()) => return CommandReturn::failure(ErrorCode::NOSUPPORT),
        };

        match command {
            Command::DriverExists => CommandReturn::success(),
            Command::Enable => self.enable_command(),
            Command::Attach => self.attach_command(),
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
