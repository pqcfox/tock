// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::bank::{Bank, BANK1_STARTING_FLASH_ADDRESS};
use super::flash_ctrl::{FLASH_HOST_STARTING_ADDRESS_OFFSET, FLASH_SIZE};

use core::cmp::Ordering;

/// Error describing why the giving host address is not a flash address
#[derive(Debug)]
pub enum InvalidHostAddressError {
    /// The host address is below the assigned flash address space (in the host address space)
    TooLow,
    /// The host address is above the assigned flash address space (in the host address space)
    TooHigh,
}

/// An address in the flash address space.
///
/// OpenTitan has two address spaces: host address space and flash address space. This type
/// provides type safety for functions that work within the flash address space.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FlashAddress(usize);

impl FlashAddress {
    /// Create a new flash address from the given pointer.
    ///
    /// # Parameters
    ///
    /// + `flash_address`: a number representing the address the flash address should point to
    ///
    /// # Return value
    ///
    /// [FlashAddress]
    ///
    /// # Safety
    ///
    /// The caller must ensure that `flash_address` is a valid flash address, i.e. address < [FLASH_SIZE]
    pub(super) const unsafe fn new_unchecked(flash_address: usize) -> Self {
        Self(flash_address)
    }

    /// Create a new flash address from the given pointer.
    ///
    /// # Parameters
    ///
    /// + `flash_address`: a number representing the address the flash address should point to
    ///
    /// # Return value
    ///
    /// + Ok([FlashAddress]): the resulting flash address
    /// + Err(()): the given `flash_address` is not valid
    pub(super) const fn new(flash_address: usize) -> Result<Self, ()> {
        if flash_address >= FLASH_SIZE.get() {
            Err(())
        } else {
            Ok(Self(flash_address))
        }
    }

    /// Create a new flash address from the given host address
    ///
    /// # Parameters
    ///
    /// + `host_address`: address from the host address space
    ///
    /// # Return value
    ///
    /// + Ok(flash_address) if host_address is valid
    /// + Err(invalid_host_address_error) if host_address is invalid
    pub fn new_from_host_address(host_address: *const u8) -> Result<Self, InvalidHostAddressError> {
        if (host_address as usize) < FLASH_HOST_STARTING_ADDRESS_OFFSET.get() {
            Err(InvalidHostAddressError::TooLow)
        } else {
            let translated_flash_address =
                host_address as usize - FLASH_HOST_STARTING_ADDRESS_OFFSET.get();

            Self::new(translated_flash_address).map_err(|()| InvalidHostAddressError::TooHigh)
        }
    }

    /// Increment the given flash address by `difference` bytes.
    ///
    /// # Parameters
    ///
    /// + `difference`: the difference in bytes to increment the flash address
    ///
    /// # Return value
    ///
    /// The new flash address obtained after incrementing.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `self + difference` is a valid flash address.
    pub(super) const unsafe fn add_unchecked(self, difference: usize) -> Self {
        Self(self.to_usize() + difference)
    }

    /// Increment the given flash address by `difference` bytes.
    ///
    /// # Parameters
    ///
    /// + `difference`: the difference in bytes to increment the flash address
    ///
    /// # Return value
    ///
    /// + Ok(incremented_flash_address) if the increment would result in a valid flash address
    /// + Err(()) if the increment would result in an invalid flash address
    #[allow(unused)]
    pub(super) const fn add(self, difference: usize) -> Result<Self, ()> {
        let new_address = self.to_usize() + difference;

        Self::new(new_address)
    }

    /// Subtract two flash addresses
    ///
    /// # Parameters
    ///
    /// + `other_flash_address`: the FlashAddress that will be subtracted
    ///
    /// # Return value
    ///
    /// The difference of `other_flash_address` - self
    pub(super) const fn subtract(self, other_flash_address: FlashAddress) -> isize {
        // CAST: The flash address is 1MiB in size and starts from 0x0, so the maximum value
        // returned by to_usize() is 1MiB - 1 which is a valid isize value
        let value = self.to_usize() as isize;
        // CAST: The flash address is 1MiB in size and starts from 0x0, so the maximum value
        // returned by to_usize() is 1MiB - 1 which is a valid isize value
        let other_value = other_flash_address.to_usize() as isize;

        other_value - value
    }

    /// Compare two flash addresses
    ///
    /// # Parameters
    ///
    /// + `other_flash_address`: the other FlashAddress to be compared
    ///
    /// # Return value
    ///
    /// + Ordering::Less: `self` < `other_flash_address`
    /// + Ordering::Equal: `self` == `other_flash_address`
    /// + Ordering::Greater: `self` > `other_flash_address`
    const fn compare(self, other_flash_address: FlashAddress) -> Ordering {
        let self_value = self.to_usize();
        let other_value = other_flash_address.to_usize();

        if self_value < other_value {
            Ordering::Less
        } else if self_value == other_value {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }

    /// Convert the flash address to a number
    ///
    /// # Return value
    ///
    /// The address as a usize this pointer points to
    pub(super) const fn to_usize(self) -> usize {
        self.0
    }

    /// Convert the flash address to bank
    ///
    /// # Return value
    ///
    /// The bank this address belongs to
    pub(super) const fn to_bank(self) -> Bank {
        match self.compare(BANK1_STARTING_FLASH_ADDRESS) {
            Ordering::Less => Bank::Bank0,
            _ => Bank::Bank1,
        }
    }
}

#[cfg(feature = "test_flash_ctrl")]
pub(super) mod tests {
    use super::super::tests::{print_test_footer, print_test_header};
    use super::FlashAddress;

    macro_rules! check_valid_flash_address_add {
        ($old_address:expr, $difference:expr) => {{
            match $old_address.add($difference) {
                Ok(new_address) => {
                    let actual_inner_value = new_address.to_usize();
                    let expected_inner_value = $old_address.to_usize() + $difference;

                    assert_eq!(
                        expected_inner_value, actual_inner_value,
                        "Adding didn't produce the correct value"
                    );
                }
                Err(()) => panic!(
                    "Adding {} to {:?} should result in a valid flash address",
                    $difference, $old_address
                ),
            }
        }};
    }

    macro_rules! check_invalid_flash_address_add {
        ($flash_address:expr, $difference:expr) => {{
            if let Ok(_) = $flash_address.add($difference) {
                panic!(
                    "Adding {} to {:?} should result in an invalid flash address",
                    $difference, $flash_address
                );
            }
        }};
    }

    fn test_valid_flash_address_add() {
        print_test_header("FlashAddress::add() - valid");

        // SAFETY: 0 is a valid flash address
        let flash_address = unsafe { FlashAddress::new_unchecked(0) };
        check_valid_flash_address_add!(flash_address, 2);
        // SAFETY: 4 is a valid flash address
        let flash_address = unsafe { FlashAddress::new_unchecked(4) };
        check_valid_flash_address_add!(flash_address, 7);
        // SAFETY: 123 is a valid flash address
        let flash_address = unsafe { FlashAddress::new_unchecked(123) };
        check_valid_flash_address_add!(flash_address, 123);
        // SAFETY: 98765 is a valid flash address
        let flash_address = unsafe { FlashAddress::new_unchecked(98765) };
        check_valid_flash_address_add!(flash_address, 4321);

        print_test_footer("FlashAddress::add() - valid");
    }

    fn test_invalid_flash_address_add() {
        print_test_header("FlashAddress::add() - invalid");

        // SAFETY: 1 << 20 - 1 is a valid flash address
        let flash_address = unsafe { FlashAddress::new_unchecked((1 << 20) - 1) };
        check_invalid_flash_address_add!(flash_address, 1);

        // SAFETY: 1 << 10 is a valid flash address
        let flash_address = unsafe { FlashAddress::new_unchecked(1 << 10) };
        check_invalid_flash_address_add!(flash_address, 1 << 20);

        print_test_footer("FlashAddress::add() - invalid");
    }

    #[cfg_attr(test, test)]
    fn test_flash_address_add() {
        test_valid_flash_address_add();
        test_invalid_flash_address_add();
    }

    macro_rules! check_flash_address_subtraction {
        ($flash_address1:expr, $flash_address2:expr, $expected_difference:expr) => {{
            assert_eq!(
                $flash_address1.subtract($flash_address2),
                $expected_difference,
                "Expected the difference {:?} - {:?} to be {}",
                $flash_address2,
                $flash_address1,
                $expected_difference
            );
        }};
    }

    #[cfg_attr(test, test)]
    fn test_flash_address_subtraction() {
        print_test_header("FlashAddress::subtract()");

        let flash_address1 = unsafe { FlashAddress::new_unchecked(0) };
        let flash_address2 = unsafe { FlashAddress::new_unchecked(1) };
        check_flash_address_subtraction!(flash_address1, flash_address2, 1);

        let flash_address1 = unsafe { FlashAddress::new_unchecked(1) };
        let flash_address2 = unsafe { FlashAddress::new_unchecked(0) };
        check_flash_address_subtraction!(flash_address1, flash_address2, -1);

        let flash_address1 = unsafe { FlashAddress::new_unchecked(5) };
        let flash_address2 = unsafe { FlashAddress::new_unchecked(2024) };
        check_flash_address_subtraction!(flash_address1, flash_address2, 2019);

        let flash_address1 = unsafe { FlashAddress::new_unchecked(216) };
        let flash_address2 = unsafe { FlashAddress::new_unchecked(162) };
        check_flash_address_subtraction!(flash_address1, flash_address2, -54);

        print_test_footer("FlashAddress::subtract()");
    }

    pub(in super::super) fn run_all() {
        test_flash_address_add();
        test_flash_address_subtraction();
    }
}
