// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

//! Endpoint index

use crate::registers::usbdev_regs::{
    EP_IN_ENABLE, EP_OUT_ENABLE, IN_ISO, OUT_ISO, RXENABLE_OUT, RXENABLE_SETUP,
};

use kernel::utilities::registers::FieldValue;

#[repr(usize)]
#[derive(Clone, Copy, Debug)]
/// Endpoint index.
pub(super) enum EndpointIndex {
    Endpoint0,
    Endpoint1,
    Endpoint2,
    Endpoint3,
    Endpoint4,
    Endpoint5,
    Endpoint6,
    Endpoint7,
    Endpoint8,
    Endpoint9,
    Endpoint10,
    Endpoint11,
}

impl EndpointIndex {
    /// Converts the endpoint index to usize
    ///
    /// # Return value
    ///
    /// The usize representation of the endpoint index.
    pub(super) const fn to_usize(self) -> usize {
        // CAST: u32 == usize on RV32I
        self as usize
    }

    /// Try creating an endpoint index.
    ///
    /// Attempts to construct an endpoint index from the given value.
    ///
    /// # Parameters
    ///
    /// + `value`: the usize representation of the endpoint index
    ///
    /// # Return value
    ///
    /// + Ok: `value` is valid (< 12)
    /// + Err: `value` is invalid (>= 12)
    pub(super) const fn try_from_usize(value: usize) -> Result<Self, ()> {
        match value {
            0 => Ok(EndpointIndex::Endpoint0),
            1 => Ok(EndpointIndex::Endpoint1),
            2 => Ok(EndpointIndex::Endpoint2),
            3 => Ok(EndpointIndex::Endpoint3),
            4 => Ok(EndpointIndex::Endpoint4),
            5 => Ok(EndpointIndex::Endpoint5),
            6 => Ok(EndpointIndex::Endpoint6),
            7 => Ok(EndpointIndex::Endpoint7),
            8 => Ok(EndpointIndex::Endpoint8),
            9 => Ok(EndpointIndex::Endpoint9),
            10 => Ok(EndpointIndex::Endpoint10),
            11 => Ok(EndpointIndex::Endpoint11),
            _ => Err(()),
        }
    }

    /// Converts the endpoint index to `ep_in_enable` bitfield set to 1.
    ///
    /// # Return value
    ///
    /// The corresponding `ep_in_enable` bitfield set to 1.
    pub(super) const fn to_set_ep_in_enable_field_value(
        self,
    ) -> FieldValue<u32, EP_IN_ENABLE::Register> {
        match self {
            EndpointIndex::Endpoint0 => EP_IN_ENABLE::ENABLE_0::SET,
            EndpointIndex::Endpoint1 => EP_IN_ENABLE::ENABLE_1::SET,
            EndpointIndex::Endpoint2 => EP_IN_ENABLE::ENABLE_2::SET,
            EndpointIndex::Endpoint3 => EP_IN_ENABLE::ENABLE_3::SET,
            EndpointIndex::Endpoint4 => EP_IN_ENABLE::ENABLE_4::SET,
            EndpointIndex::Endpoint5 => EP_IN_ENABLE::ENABLE_5::SET,
            EndpointIndex::Endpoint6 => EP_IN_ENABLE::ENABLE_6::SET,
            EndpointIndex::Endpoint7 => EP_IN_ENABLE::ENABLE_7::SET,
            EndpointIndex::Endpoint8 => EP_IN_ENABLE::ENABLE_8::SET,
            EndpointIndex::Endpoint9 => EP_IN_ENABLE::ENABLE_9::SET,
            EndpointIndex::Endpoint10 => EP_IN_ENABLE::ENABLE_10::SET,
            EndpointIndex::Endpoint11 => EP_IN_ENABLE::ENABLE_11::SET,
        }
    }

    /// Converts the endpoint index to `ep_out_enable` bitfield set to 1.
    ///
    /// # Return value
    ///
    /// The corresponding `ep_out_enable` bitfield set to 1.
    pub(super) const fn to_set_ep_out_enable_field_value(
        self,
    ) -> FieldValue<u32, EP_OUT_ENABLE::Register> {
        match self {
            EndpointIndex::Endpoint0 => EP_OUT_ENABLE::ENABLE_0::SET,
            EndpointIndex::Endpoint1 => EP_OUT_ENABLE::ENABLE_1::SET,
            EndpointIndex::Endpoint2 => EP_OUT_ENABLE::ENABLE_2::SET,
            EndpointIndex::Endpoint3 => EP_OUT_ENABLE::ENABLE_3::SET,
            EndpointIndex::Endpoint4 => EP_OUT_ENABLE::ENABLE_4::SET,
            EndpointIndex::Endpoint5 => EP_OUT_ENABLE::ENABLE_5::SET,
            EndpointIndex::Endpoint6 => EP_OUT_ENABLE::ENABLE_6::SET,
            EndpointIndex::Endpoint7 => EP_OUT_ENABLE::ENABLE_7::SET,
            EndpointIndex::Endpoint8 => EP_OUT_ENABLE::ENABLE_8::SET,
            EndpointIndex::Endpoint9 => EP_OUT_ENABLE::ENABLE_9::SET,
            EndpointIndex::Endpoint10 => EP_OUT_ENABLE::ENABLE_10::SET,
            EndpointIndex::Endpoint11 => EP_OUT_ENABLE::ENABLE_11::SET,
        }
    }

    /// Converts the endpoint index to `rxenable_out` bitfield set to 1.
    ///
    /// # Return value
    ///
    /// The corresponding `rxenable_out` bitfield set to 1.
    pub(super) const fn to_set_rxenable_out_field_value(
        self,
    ) -> FieldValue<u32, RXENABLE_OUT::Register> {
        match self {
            EndpointIndex::Endpoint0 => RXENABLE_OUT::OUT_0::SET,
            EndpointIndex::Endpoint1 => RXENABLE_OUT::OUT_1::SET,
            EndpointIndex::Endpoint2 => RXENABLE_OUT::OUT_2::SET,
            EndpointIndex::Endpoint3 => RXENABLE_OUT::OUT_3::SET,
            EndpointIndex::Endpoint4 => RXENABLE_OUT::OUT_4::SET,
            EndpointIndex::Endpoint5 => RXENABLE_OUT::OUT_5::SET,
            EndpointIndex::Endpoint6 => RXENABLE_OUT::OUT_6::SET,
            EndpointIndex::Endpoint7 => RXENABLE_OUT::OUT_7::SET,
            EndpointIndex::Endpoint8 => RXENABLE_OUT::OUT_8::SET,
            EndpointIndex::Endpoint9 => RXENABLE_OUT::OUT_9::SET,
            EndpointIndex::Endpoint10 => RXENABLE_OUT::OUT_10::SET,
            EndpointIndex::Endpoint11 => RXENABLE_OUT::OUT_11::SET,
        }
    }

    /// Converts the endpoint index to `rxenable_out` bitfield set to 0.
    ///
    /// # Return value
    ///
    /// The corresponding `rxenable_out` bitfield set to 0.
    pub(super) const fn to_clear_rxenable_out_field_value(
        self,
    ) -> FieldValue<u32, RXENABLE_OUT::Register> {
        match self {
            EndpointIndex::Endpoint0 => RXENABLE_OUT::OUT_0::CLEAR,
            EndpointIndex::Endpoint1 => RXENABLE_OUT::OUT_1::CLEAR,
            EndpointIndex::Endpoint2 => RXENABLE_OUT::OUT_2::CLEAR,
            EndpointIndex::Endpoint3 => RXENABLE_OUT::OUT_3::CLEAR,
            EndpointIndex::Endpoint4 => RXENABLE_OUT::OUT_4::CLEAR,
            EndpointIndex::Endpoint5 => RXENABLE_OUT::OUT_5::CLEAR,
            EndpointIndex::Endpoint6 => RXENABLE_OUT::OUT_6::CLEAR,
            EndpointIndex::Endpoint7 => RXENABLE_OUT::OUT_7::CLEAR,
            EndpointIndex::Endpoint8 => RXENABLE_OUT::OUT_8::CLEAR,
            EndpointIndex::Endpoint9 => RXENABLE_OUT::OUT_9::CLEAR,
            EndpointIndex::Endpoint10 => RXENABLE_OUT::OUT_10::CLEAR,
            EndpointIndex::Endpoint11 => RXENABLE_OUT::OUT_11::CLEAR,
        }
    }

    /// Converts the endpoint index to `rxenable_setup` bitfield set to 1.
    ///
    /// # Return value
    ///
    /// The corresponding `rxenable_setup` bitfield set to 1.
    pub(super) const fn to_set_rxenable_setup_field_value(
        self,
    ) -> FieldValue<u32, RXENABLE_SETUP::Register> {
        match self {
            EndpointIndex::Endpoint0 => RXENABLE_SETUP::SETUP_0::SET,
            EndpointIndex::Endpoint1 => RXENABLE_SETUP::SETUP_1::SET,
            EndpointIndex::Endpoint2 => RXENABLE_SETUP::SETUP_2::SET,
            EndpointIndex::Endpoint3 => RXENABLE_SETUP::SETUP_3::SET,
            EndpointIndex::Endpoint4 => RXENABLE_SETUP::SETUP_4::SET,
            EndpointIndex::Endpoint5 => RXENABLE_SETUP::SETUP_5::SET,
            EndpointIndex::Endpoint6 => RXENABLE_SETUP::SETUP_6::SET,
            EndpointIndex::Endpoint7 => RXENABLE_SETUP::SETUP_7::SET,
            EndpointIndex::Endpoint8 => RXENABLE_SETUP::SETUP_8::SET,
            EndpointIndex::Endpoint9 => RXENABLE_SETUP::SETUP_9::SET,
            EndpointIndex::Endpoint10 => RXENABLE_SETUP::SETUP_10::SET,
            EndpointIndex::Endpoint11 => RXENABLE_SETUP::SETUP_11::SET,
        }
    }

    pub(super) const fn to_set_in_iso_field_value(self) -> FieldValue<u32, IN_ISO::Register> {
        match self {
            EndpointIndex::Endpoint0 => IN_ISO::ISO_0::SET,
            EndpointIndex::Endpoint1 => IN_ISO::ISO_1::SET,
            EndpointIndex::Endpoint2 => IN_ISO::ISO_2::SET,
            EndpointIndex::Endpoint3 => IN_ISO::ISO_3::SET,
            EndpointIndex::Endpoint4 => IN_ISO::ISO_4::SET,
            EndpointIndex::Endpoint5 => IN_ISO::ISO_5::SET,
            EndpointIndex::Endpoint6 => IN_ISO::ISO_6::SET,
            EndpointIndex::Endpoint7 => IN_ISO::ISO_7::SET,
            EndpointIndex::Endpoint8 => IN_ISO::ISO_8::SET,
            EndpointIndex::Endpoint9 => IN_ISO::ISO_9::SET,
            EndpointIndex::Endpoint10 => IN_ISO::ISO_10::SET,
            EndpointIndex::Endpoint11 => IN_ISO::ISO_11::SET,
        }
    }

    pub(super) const fn to_set_out_iso_field_value(self) -> FieldValue<u32, OUT_ISO::Register> {
        match self {
            EndpointIndex::Endpoint0 => OUT_ISO::ISO_0::SET,
            EndpointIndex::Endpoint1 => OUT_ISO::ISO_1::SET,
            EndpointIndex::Endpoint2 => OUT_ISO::ISO_2::SET,
            EndpointIndex::Endpoint3 => OUT_ISO::ISO_3::SET,
            EndpointIndex::Endpoint4 => OUT_ISO::ISO_4::SET,
            EndpointIndex::Endpoint5 => OUT_ISO::ISO_5::SET,
            EndpointIndex::Endpoint6 => OUT_ISO::ISO_6::SET,
            EndpointIndex::Endpoint7 => OUT_ISO::ISO_7::SET,
            EndpointIndex::Endpoint8 => OUT_ISO::ISO_8::SET,
            EndpointIndex::Endpoint9 => OUT_ISO::ISO_9::SET,
            EndpointIndex::Endpoint10 => OUT_ISO::ISO_10::SET,
            EndpointIndex::Endpoint11 => OUT_ISO::ISO_11::SET,
        }
    }
}
