// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! USB address

use super::utils;

use core::num::NonZeroU8;

const MAXIMUM_USB_ADDRESS: NonZeroU8 = utils::create_non_zero_u8(127);

/// Non default USB address
#[derive(Clone, Copy)]
pub(super) struct NonDefaultUsbAddress(NonZeroU8);

impl NonDefaultUsbAddress {
    /// Tries to create a non-default USB address from the given value
    ///
    /// # Parameters
    ///
    /// + `value`: a NonZeroU8 value supposed to represent a non-default USB address
    ///
    /// # Return value
    ///
    /// + Ok: `value` is valid (<= MAXIMUM_USB_ADDRESS)
    /// + Err: `value` is invalid (> MAXIMUM_USB_ADDRESS)
    pub(super) const fn try_from_non_zero_u8(value: NonZeroU8) -> Result<Self, ()> {
        if value.get() > MAXIMUM_USB_ADDRESS.get() {
            Err(())
        } else {
            Ok(Self(value))
        }
    }

    /// Converts the non-default USB address to u8
    ///
    /// # Return value
    ///
    /// The u8 representation of `self`
    pub(super) const fn to_u8(self) -> u8 {
        self.0.get()
    }
}

/// USB address
#[repr(u8)]
#[derive(Clone, Copy)]
pub(super) enum UsbAddress {
    /// Default USB address
    Default = 0,
    /// Non-default USB address
    NonDefault(NonDefaultUsbAddress),
}

impl UsbAddress {
    /// Tries to create a USB address from the given value
    ///
    /// # Parameters
    ///
    /// + `value`: a u8 value supposed to represent a USB address
    ///
    /// # Return value
    ///
    /// + Ok: `value` is valid (<= MAXIMUM_USB_ADDRESS)
    /// + Err: `value` is invalid (> MAXIMUM_USB_ADDRESS)
    pub(super) const fn try_from_u8(value: u8) -> Result<Self, ()> {
        match NonZeroU8::new(value) {
            Some(non_zero_u8) => match NonDefaultUsbAddress::try_from_non_zero_u8(non_zero_u8) {
                Ok(non_default_usb_address) => Ok(UsbAddress::NonDefault(non_default_usb_address)),
                Err(()) => Err(()),
            },
            None => Ok(UsbAddress::Default),
        }
    }

    /// Returns the default USB address
    ///
    /// # Return value
    ///
    /// Default USB address
    pub(super) const fn default() -> Self {
        UsbAddress::Default
    }

    /// Converts the USB address to u8
    ///
    /// # Return value
    ///
    /// The u8 representation of `self`
    pub(super) const fn to_u8(self) -> u8 {
        match self {
            UsbAddress::Default => 0,
            UsbAddress::NonDefault(non_default_usb_address) => non_default_usb_address.to_u8(),
        }
    }
}
