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
use kernel::processbuffer::{ReadOnlyProcessBufferRef, ReadableProcessBuffer};
use kernel::syscall::{CommandReturn, SyscallDriver};
use kernel::utilities::cells::{OptionalCell, VolatileCell};

use core::cell::Cell;

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::UsbUser2 as usize;

#[derive(Default)]
pub struct AppData {

}

const UPCALL_COUNT: u8 = 1;

#[repr(usize)]
enum UpcallId {
    Transmit = 0,
}

impl UpcallId {
    const fn to_usize(self) -> usize {
        // CAST: UpcallId is marked repr(usize)
        self as usize
    }
}

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

const ALLOW_RW_COUNT: u8 = 0;

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
}

impl<'a, Usb: usb::UsbController<'a>> UsbSyscallDriver<'a, Usb> {
    pub fn new(usb_client: &'a UsbClient<'a, Usb>, grant: UsbGrant) -> Self {
        Self {
            usb_client,
            grant,
            current_owner: Cell::new(None),
            transmit_length: Cell::new(0),
            transmit_position: Cell::new(0),
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

    fn is_busy(&self) -> bool {
        match self.current_owner.get() {
            None => false,
            Some(process_id) => Err(process::Error::NoSuchApp) == self.grant.enter(process_id, |_, _| {}),
        }
    }

    fn update_owner(&self, owner: ProcessId) {
        self.current_owner.set(Some(owner));
    }

    fn get_owner_panic(&self) -> ProcessId {
        self.current_owner.get().unwrap()
    }

    fn transmit_chunk(&self) -> Result<(), ()> {
        let owner = self.get_owner_panic();

        self.grant.enter(owner, |_, kernel_data| {
            kernel_data.get_readonly_processbuffer(ReadOnlyBufferId::Transmit.to_usize()).and_then(|buffer| {
                let start = self.transmit_position.get();
                let transmit_length = self.transmit_length.get();
                if transmit_length == start {
                    // The capsule can't do anything if the upcall fails to be scheduled, so the
                    // result is ignored.
                    let _ = kernel_data.schedule_upcall(UpcallId::Transmit.to_usize(), (transmit_length, 0, 0));

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
        let owner = self.get_owner_panic();

        self.grant.enter(owner, |_, kernel_data| {
            let transmit_length = self.transmit_length.get();
            // The capsule can't do anything if the upcall fails to be scheduled, so the
            // result is ignored.
            let _ = kernel_data.schedule_upcall(UpcallId::Transmit.to_usize(), (transmit_length, 0, 0));
        }).unwrap();
    }

    fn transmit_command(&self, process_id: ProcessId) -> CommandReturn {
        if self.is_busy() {
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

        self.update_owner(process_id);
        self.transmit_position.set(0);
        self.transmit_length.set(length);

        if let Err(()) = self.transmit_chunk() {
            return CommandReturn::failure(ErrorCode::FAIL);
        }

        CommandReturn::success()
    }
}

enum Command {
    DriverExists = 0,
    Enable = 1,
    Attach = 2,
    Transmit = 3,
}

impl Command {
    const fn new(command_number: usize) -> Result<Self, ()> {
        const DRIVER_EXISTS_NUMBER: usize = Command::DriverExists as usize;
        const ENABLE_NUMBER: usize = Command::Enable as usize;
        const ATTACH_NUMBER: usize = Command::Attach as usize;
        const TRANSMIT_NUMBER: usize = Command::Transmit as usize;
        match command_number {
            DRIVER_EXISTS_NUMBER => Ok(Command::DriverExists),
            ENABLE_NUMBER => Ok(Command::Enable),
            ATTACH_NUMBER => Ok(Command::Attach),
            TRANSMIT_NUMBER => Ok(Command::Transmit),
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
        _transfer_type: usb::TransferType,
        _endpoint: usize,
        _packet_bytes: u32,
    ) -> usb::OutResult {
        todo!();
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
            Command::Transmit => self.transmit_command(process_id),
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(process_id, |_, _| {})
    }
}
