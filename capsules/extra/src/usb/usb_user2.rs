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
use kernel::syscall::{CommandReturn, SyscallDriver};

pub const DRIVER_NUM: usize = capsules_core::driver::NUM::UsbUser2 as usize;

#[derive(Default)]
pub struct AppData {

}

const UPCALL_COUNT: u8 = 2;
const ALLOW_RO_COUNT: u8 = 1;
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
const BULK_OUT_ENDPOINT: usize = 1;

pub struct UsbClient<'a, Usb: usb::UsbController<'a>> {
    usb: &'a Usb,
    usb_ctrl: ClientCtrl<'a, 'static, Usb>,
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
                max_packet_size: 8,
                interval: 0,
            },
            EndpointDescriptor {
                endpoint_address: EndpointAddress::new_const(BULK_OUT_ENDPOINT, TransferDirection::HostToDevice),
                transfer_type: usb::TransferType::Bulk,
                max_packet_size: 8,
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
                    max_packet_size_ep0: 64,
                    ..DeviceDescriptor::default()
                },
                ConfigurationDescriptor::default(),
                interfaces,
                endpoints,
                None,
                None,
            );

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
        }
    }
}

pub struct UsbSyscallDriver<
    'a,
    Usb: usb::UsbController<'a>,
> {
    usb_client: &'a UsbClient<'a, Usb>,
    grant: UsbGrant,
}

impl<'a, Usb: usb::UsbController<'a>> UsbSyscallDriver<'a, Usb> {
    pub fn new(usb_client: &'a UsbClient<'a, Usb>, grant: UsbGrant) -> Self {
        Self {
            usb_client,
            grant,
        }
    }

    fn enable_command(&self) -> CommandReturn {
        self.usb_client.enable();
        CommandReturn::success()
    }

    fn attach_command(&self) -> CommandReturn {
        self.usb_client.attach();
        CommandReturn::success()
    }
}

enum Command {
    DriverExists = 0,
    Enable = 1,
    Attach = 2,
}

impl Command {
    const fn new(command_number: usize) -> Result<Self, ()> {
        const DRIVER_EXISTS_NUMBER: usize = Command::DriverExists as usize;
        const ENABLE_NUMBER: usize = Command::Enable as usize;
        const ATTACH_NUMBER: usize = Command::Attach as usize;
        match command_number {
            DRIVER_EXISTS_NUMBER => Ok(Command::DriverExists),
            ENABLE_NUMBER => Ok(Command::Enable),
            ATTACH_NUMBER => Ok(Command::Attach),
            _ => Err(()),
        }
    }
}

impl<'a, Usb: usb::UsbController<'a>> usb::Client<'a> for UsbClient<'a, Usb> {
    fn enable(&'a self) {
        self.usb_ctrl.enable();

        // IN endpoint
        self.usb.endpoint_in_enable(usb::TransferType::Bulk, BULK_IN_ENDPOINT).unwrap();

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

    fn packet_in(&'a self, _transfer_type: usb::TransferType, _endpoint: usize) -> usb::InResult {
        todo!();
    }

    fn packet_out(
        &'a self,
        _transfer_type: usb::TransferType,
        _endpoint: usize,
        _packet_bytes: u32,
    ) -> usb::OutResult {
        todo!();
    }

    fn packet_transmitted(&'a self, _endpoint: usize, _result: Result<(), ()>) {
        todo!();
    }
}

impl<'a, Usb: usb::UsbController<'a>> SyscallDriver for UsbSyscallDriver<'a, Usb> {
    fn command(
        &self,
        command_number: usize,
        _argument1: usize,
        _argument2: usize,
        _process_id: ProcessId
    ) -> CommandReturn {
        let command = match Command::new(command_number) {
            Ok(command) => command,
            Err(()) => return CommandReturn::failure(ErrorCode::NOSUPPORT),
        };

        match command {
            Command::DriverExists => CommandReturn::success(),
            Command::Enable => self.enable_command(),
            Command::Attach => self.attach_command(),
        }
    }

    fn allocate_grant(&self, process_id: ProcessId) -> Result<(), kernel::process::Error> {
        self.grant.enter(process_id, |_, _| {})
    }
}
