// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! USB Requests

use kernel::utilities::cells::VolatileCell;

use core::num::NonZeroU16;

/// Standard USB device request that requires sending data to host
pub(super) enum StandardDeviceRequestToHost {
    GetStatus = 0,
    GetDescriptor = 6,
    GetConfiguration = 8,
    GetInterface = 10,
    SynchFrame = 12,
}

/// Standard USB device request that requires receiving data from host
pub(super) enum StandardDeviceRequestFromHost {
    ClearFeature = 1,
    SetFeature = 3,
    SetAddress = 5,
    SetDescriptor = 7,
    SetConfiguration = 9,
    SetInterface = 11,
}

/// Standard USB device request
pub(super) enum StandardDeviceRequest {
    ToHost(StandardDeviceRequestToHost),
    FromHost(StandardDeviceRequestFromHost),
}

/// Standard USB request
pub(super) enum StandardRequest {
    Device(StandardDeviceRequest),
}

/// Class USB request
pub(super) enum ClassRequest {
    /// Class request that needs to transfer data from device to host
    ToHost(Option<NonZeroU16>),
    /// Class request that needs to transfer data from host to device
    FromHost(Option<NonZeroU16>),
}

/// USB request
pub(super) enum Request {
    Standard(StandardRequest),
    Class(ClassRequest),
}

/// Errors while decoding a SETUP packet
#[derive(Debug)]
pub(super) enum RequestDecodeError {
    /// The packet is too short
    PacketTooShort,
    /// Unknown request type
    UnknownType,
    /// Unknown request recipient
    UnknownRecipient,
    /// Unknown request
    UnknownRequest,
}

/// Indicates request direction
enum RequestDirection {
    /// Data stage transfers are made from device to host
    FromHost,
    /// Data stage trnasfers are made from host to device
    ToHost,
}

impl Request {
    /// Determines the direction of the request
    ///
    /// # Parameters
    ///
    /// `request_type_byte`: the byte representing the request type
    ///
    /// # Return value
    ///
    /// Request direction
    const fn get_direction(request_type_byte: u8) -> RequestDirection {
        const REQUEST_DIRECTION_MASK: u8 = 0b1000_0000;
        match (request_type_byte & REQUEST_DIRECTION_MASK) != 0 {
            false => RequestDirection::FromHost,
            true => RequestDirection::ToHost,
        }
    }

    /// Constructor for a standard device USB request that needs to transfer data from device to
    /// host
    ///
    /// # Parameters
    ///
    /// + `request_type_byte`: byte representing the request type
    /// + `request_byte`: byte identifying the request
    ///
    /// # Return value
    ///
    /// + Ok: request_type_byte is valid
    /// + Err: request_type_byte is invalid
    const fn try_standard_device_to_host(request_byte: u8) -> Result<Self, RequestDecodeError> {
        match request_byte {
            0 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::ToHost(StandardDeviceRequestToHost::GetStatus),
            ))),
            6 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::ToHost(StandardDeviceRequestToHost::GetDescriptor),
            ))),
            8 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::ToHost(StandardDeviceRequestToHost::GetConfiguration),
            ))),
            10 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::ToHost(StandardDeviceRequestToHost::GetInterface),
            ))),
            12 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::ToHost(StandardDeviceRequestToHost::SynchFrame),
            ))),
            _ => Err(RequestDecodeError::UnknownRequest),
        }
    }

    /// Constructor for a standard device USB request that needs to transfer data from host to
    /// device
    ///
    /// # Parameters
    ///
    /// + `request_type_byte`: byte representing the request type
    /// + `request_byte`: byte identifying the request
    ///
    /// # Return value
    ///
    /// + Ok: request_type_byte is valid
    /// + Err: request_type_byte is invalid
    const fn try_standard_device_from_host(request_byte: u8) -> Result<Self, RequestDecodeError> {
        match request_byte {
            1 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::FromHost(StandardDeviceRequestFromHost::ClearFeature),
            ))),
            3 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::FromHost(StandardDeviceRequestFromHost::SetFeature),
            ))),
            5 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::FromHost(StandardDeviceRequestFromHost::SetAddress),
            ))),
            7 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::FromHost(StandardDeviceRequestFromHost::SetDescriptor),
            ))),
            9 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::FromHost(StandardDeviceRequestFromHost::SetConfiguration),
            ))),
            11 => Ok(Request::Standard(StandardRequest::Device(
                StandardDeviceRequest::FromHost(StandardDeviceRequestFromHost::SetInterface),
            ))),
            _ => Err(RequestDecodeError::UnknownRequest),
        }
    }

    /// Constructor for a standard device USB request
    ///
    /// # Parameters
    ///
    /// + `request_type_byte`: byte representing the request type
    /// + `request_byte`: byte identifying the request
    ///
    /// # Return value
    ///
    /// + Ok: request_type_byte is valid
    /// + Err: request_type_byte is invalid
    const fn try_standard_device(
        request_type_byte: u8,
        request_byte: u8,
    ) -> Result<Self, RequestDecodeError> {
        match Self::get_direction(request_type_byte) {
            RequestDirection::FromHost => Self::try_standard_device_from_host(request_byte),
            RequestDirection::ToHost => Self::try_standard_device_to_host(request_byte),
        }
    }

    /// Constructor for a standard USB request
    ///
    /// # Parameters
    ///
    /// + `request_type_byte`: byte representing the request type
    /// + `request_byte`: byte identifying the request
    ///
    /// # Return value
    ///
    /// + Ok: `request_type_byte` is valid
    /// + Err: `request_type_byte` is invalid
    const fn try_standard(
        request_type_byte: u8,
        request_byte: u8,
    ) -> Result<Self, RequestDecodeError> {
        const REQUEST_RECIPIENT_MASK: u8 = 0b0001_1111;
        match request_type_byte & REQUEST_RECIPIENT_MASK {
            0 => Self::try_standard_device(request_type_byte, request_byte),
            _ => Err(RequestDecodeError::UnknownRecipient),
        }
    }

    /// Constructs a class request
    ///
    /// # Parameters
    ///
    /// + ̀`request_type_byte`: byte representing the request type
    ///
    /// # Return value
    ///
    /// + Ok: `request_type_byte` is valid
    /// + Err: `request_type_byte` is invalid
    const fn new_class(request_type_byte: u8, length: Option<NonZeroU16>) -> Self {
        match Self::get_direction(request_type_byte) {
            RequestDirection::FromHost => Request::Class(ClassRequest::FromHost(length)),
            RequestDirection::ToHost => Request::Class(ClassRequest::ToHost(length)),
        }
    }

    /// Tries to create a USB request from a packet
    ///
    /// # Parameters
    ///
    /// + `packet`: the packet supposed to represent a packet
    ///
    /// # Return value
    ///
    /// + Ok: the packet represents a valid USB request
    /// + Err: the packet does not represent a valid USB request
    pub(super) fn try_from_packet(packet: &[VolatileCell<u8>]) -> Result<Self, RequestDecodeError> {
        let request_type_byte = match packet.first() {
            Some(volatile_byte) => volatile_byte.get(),
            None => return Err(RequestDecodeError::PacketTooShort),
        };

        let request_byte = match packet.get(1) {
            Some(volatile_byte) => volatile_byte.get(),
            None => return Err(RequestDecodeError::PacketTooShort),
        };

        let length_byte0 = match packet.get(6) {
            Some(volatile_byte) => volatile_byte.get(),
            None => return Err(RequestDecodeError::PacketTooShort),
        };

        let length_byte1 = match packet.get(7) {
            Some(volatile_byte) => volatile_byte.get(),
            None => return Err(RequestDecodeError::PacketTooShort),
        };

        let length = NonZeroU16::new(u16::from_ne_bytes([length_byte0, length_byte1]));

        const REQUEST_TYPE_MASK: u8 = 0b0110_0000;
        match request_type_byte & REQUEST_TYPE_MASK {
            0 => Self::try_standard(request_type_byte, request_byte),
            32 => Ok(Self::new_class(request_type_byte, length)),
            _ => Err(RequestDecodeError::UnknownType),
        }
    }
}
