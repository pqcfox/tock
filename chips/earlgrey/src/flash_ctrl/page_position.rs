// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::bank::{Bank, BANK0_STARTING_FLASH_ADDRESS, BANK1_STARTING_FLASH_ADDRESS};
use super::flash_address::FlashAddress;
use super::info_partition_type::InfoPartitionType;
use super::page::EARLGREY_PAGE_SIZE;
use super::page_index::{DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex};

/// Implement a generic page position
macro_rules! implement_page_position {
    ($name:ident, $index:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum $name {
            Bank0($index),
            Bank1($index),
        }

        impl $name {
            pub const fn new(bank: Bank, page_index: $index) -> Self {
                match bank {
                    Bank::Bank0 => $name::Bank0(page_index),
                    Bank::Bank1 => $name::Bank1(page_index),
                }
            }

            pub(super) const fn to_flash_ptr(self) -> FlashAddress {
                match self {
                    $name::Bank0(page_index) => {
                        let difference = page_index.to_usize() * EARLGREY_PAGE_SIZE.get();
                        // SAFETY:
                        //
                        // + BANK0_STARTING_FLASH_ADDRESS + difference is a valid flash address since
                        // $name represents a valid page position
                        unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(difference) }
                    }
                    $name::Bank1(page_index) => {
                        let difference = page_index.to_usize() * EARLGREY_PAGE_SIZE.get();
                        // SAFETY:
                        //
                        // + BANK1_STARTING_FLASH_ADDRESS + difference is a valid flash address since
                        // $name represents a valid page position
                        unsafe { BANK1_STARTING_FLASH_ADDRESS.add_unchecked(difference) }
                    }
                }
            }
        }
    };
}

implement_page_position!(DataPagePosition, DataPageIndex);
implement_page_position!(Info0PagePosition, Info0PageIndex);
implement_page_position!(Info1PagePosition, Info1PageIndex);
implement_page_position!(Info2PagePosition, Info2PageIndex);

impl DataPagePosition {
    /// Construct a new data memory protection region base from the flash position represented as
    /// bank and an bank_offset relative to the beginning of the given flash.
    ///
    /// # Parameters
    ///
    /// + `bank`: the bank that the new data memory protection base should belong to
    /// + `bank_offset`: the bank_offset in bytes relative to the start of `bank`
    ///
    /// # Return value
    ///
    /// The newly constructed [DataMemoryProtectionRegionBase] that covers the address represented
    /// by (bank, bank_offset)
    ///
    /// # Safety
    ///
    /// The caller must ensure that `bank_offset` < [BANK_SIZE]
    const unsafe fn new_from_bank_and_relative_offset(bank: Bank, bank_offset: usize) -> Self {
        // CAST:
        //
        // + offset is valid (method's precondition) => offset < BANK_SIZE [1]
        // + From [1], offset / EARLGREY_PAGE_SIZE < BANK_SIZE / EARLGREY_PAGE_SIZE =
        // 512KiB / 2KiB = 256
        //
        // Since raw_data_page_index < 256, the cast is safe.
        let raw_data_page_index = (bank_offset / EARLGREY_PAGE_SIZE.get()) as u8;
        let data_page_index = DataPageIndex::new(raw_data_page_index);

        Self::new(bank, data_page_index)
    }

    /// Map the given flash address to a DataPagePosition that would cover it
    ///
    /// # Parameters
    ///
    /// + `flash_address`: the [FlashAddress] to be mapped
    ///
    /// # Return value
    ///
    /// The newly constructed [DataPagePosition]
    pub(super) const fn new_from_flash_address(flash_address: FlashAddress) -> Self {
        match flash_address.to_bank() {
            Bank::Bank0 => {
                // CAST: the match arm guarantees that flash_address is a valid bank0 address, i.e.
                // flash_address >= BANK0_STARTING_FLASH_ADDRESS, so the difference can be
                // represented as an unsigned value.
                let bank_offset = BANK0_STARTING_FLASH_ADDRESS.subtract(flash_address) as usize;
                // SAFETY: the match arm guarantees that flash_address is a valid bank0 address and
                // subtracting BANK0_STARTING_FLASH_ADDRESS from it provides a valid
                // bank0-relative offset
                unsafe { Self::new_from_bank_and_relative_offset(Bank::Bank0, bank_offset) }
            }
            Bank::Bank1 => {
                // CAST: the match arm guarantees that flash_address is a valid bank1 address, i.e.
                // flash_address >= BANK1_STARTING_FLASH_ADDRESS, so the difference can be
                // represented as an unsigned value.
                let bank_offset = BANK1_STARTING_FLASH_ADDRESS.subtract(flash_address) as usize;
                // SAFETY: the match arm guarantees that flash_address is a valid bank1 address and
                // subtracting BANK1_STARTING_FLASH_ADDRESS from it provides a valid
                // bank1-relative offset
                unsafe { Self::new_from_bank_and_relative_offset(Bank::Bank1, bank_offset) }
            }
        }
    }
}

/// Info page position
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum InfoPagePosition {
    /// Info page position for partitions of type 0
    Type0(Info0PagePosition),
    /// Info page position for partitions of type 1
    Type1(Info1PagePosition),
    /// Info page position for partitions of type 2
    Type2(Info2PagePosition),
}

impl InfoPagePosition {
    /// Convert info page position to [FlashAddress]
    ///
    /// # Return value
    ///
    /// The starting [FlashAddress] corresponding to this info page position
    pub(super) const fn to_flash_ptr(self) -> FlashAddress {
        match self {
            InfoPagePosition::Type0(info_page_position) => info_page_position.to_flash_ptr(),
            InfoPagePosition::Type1(info_page_position) => info_page_position.to_flash_ptr(),
            InfoPagePosition::Type2(info_page_position) => info_page_position.to_flash_ptr(),
        }
    }

    /// Convert info page position to [InfoPartitionType]
    ///
    /// # Return value
    ///
    /// + InfoPartitionType::Type0 if the info page position is of type 0
    /// + InfoPartitionType::Type1 if the info page position is of type 1
    /// + InfoPartitionType::Type2 if the info page position is of type 2
    pub(super) const fn to_info_partition_type(self) -> InfoPartitionType {
        match self {
            InfoPagePosition::Type0(_) => InfoPartitionType::Type0,
            InfoPagePosition::Type1(_) => InfoPartitionType::Type1,
            InfoPagePosition::Type2(_) => InfoPartitionType::Type2,
        }
    }
}

#[cfg(feature = "test_flash_ctrl")]
pub(super) mod tests {
    use super::super::page_index::{DataPageIndex, MAX_DATA_PAGE_INDEX};
    use super::{
        Info0PageIndex, Info0PagePosition, Info1PageIndex, Info1PagePosition, Info2PageIndex,
        Info2PagePosition, BANK0_STARTING_FLASH_ADDRESS, BANK1_STARTING_FLASH_ADDRESS,
        EARLGREY_PAGE_SIZE,
    };
    use crate::flash_ctrl::tests::{print_test_footer, print_test_header};
    use crate::flash_ctrl::DataPagePosition;

    macro_rules! check_to_flash_ptr {
        ($page_position:expr, $expected_flash_address:expr) => {{
            let actual_flash_address = $page_position.to_flash_ptr();
            assert_eq!(
                $expected_flash_address, actual_flash_address,
                "Expected {:?}, got {:?}",
                $expected_flash_address, actual_flash_address
            );
        }};
    }

    #[cfg_attr(test, test)]
    fn test_data_page_position_to_flash_ptr() {
        print_test_header("DataPagePosition::to_flash_ptr()");

        let raw_data_page_index = 0;
        // CAST: 0 fits in u8
        let data_page_position =
            DataPagePosition::Bank0(DataPageIndex::new(raw_data_page_index as u8));
        // SAFETY:
        //
        // + raw_data_page_index * EARLGREY_PAGE_SIZE fit in isize
        // + BANK0_STARTING_FLASH_ADDRESS + raw_data_page_index * EARLGREY_PAGE_SIZE is a valid
        // flash address
        let expected_flash_address = unsafe {
            BANK0_STARTING_FLASH_ADDRESS
                .add_unchecked(raw_data_page_index * EARLGREY_PAGE_SIZE.get())
        };
        check_to_flash_ptr!(data_page_position, expected_flash_address);

        let raw_data_page_index = 1;
        // CAST: 1 fits in u8
        let data_page_position =
            DataPagePosition::Bank0(DataPageIndex::new(raw_data_page_index as u8));
        // SAFETY:
        //
        // + raw_data_page_index * EARLGREY_PAGE_SIZE fit in isize
        // + BANK0_STARTING_FLASH_ADDRESS + raw_data_page_index * EARLGREY_PAGE_SIZE is a valid
        // flash address
        let expected_flash_address = unsafe {
            BANK0_STARTING_FLASH_ADDRESS
                .add_unchecked(raw_data_page_index * EARLGREY_PAGE_SIZE.get())
        };
        check_to_flash_ptr!(data_page_position, expected_flash_address);

        let raw_data_page_index = MAX_DATA_PAGE_INDEX.get();
        let data_page_position = DataPagePosition::Bank0(DataPageIndex::new(raw_data_page_index));
        // SAFETY:
        //
        // + raw_data_page_index * EARLGREY_PAGE_SIZE fit in isize
        // + BANK0_STARTING_FLASH_ADDRESS + raw_data_page_index * EARLGREY_PAGE_SIZE is a valid
        // flash address
        let expected_flash_address = unsafe {
            BANK0_STARTING_FLASH_ADDRESS
                // CAST: size_of(usize) = size_of(u32) > size_of(u8) on RV32I
                .add_unchecked(raw_data_page_index as usize * EARLGREY_PAGE_SIZE.get())
        };
        check_to_flash_ptr!(data_page_position, expected_flash_address);

        let raw_data_page_index = 0;
        // CAST: 0 fits in u8
        let data_page_position =
            DataPagePosition::Bank1(DataPageIndex::new(raw_data_page_index as u8));
        // SAFETY:
        //
        // + raw_data_page_index * EARLGREY_PAGE_SIZE fit in isize
        // + BANK1_STARTING_FLASH_ADDRESS + raw_data_page_index * EARLGREY_PAGE_SIZE is a valid
        // flash address
        let expected_flash_address = unsafe {
            BANK1_STARTING_FLASH_ADDRESS
                .add_unchecked(raw_data_page_index * EARLGREY_PAGE_SIZE.get())
        };
        check_to_flash_ptr!(data_page_position, expected_flash_address);

        let raw_data_page_index = 1;
        // CAST: 1 fits in u8
        let data_page_position =
            DataPagePosition::Bank1(DataPageIndex::new(raw_data_page_index as u8));
        // SAFETY:
        //
        // + raw_data_page_index * EARLGREY_PAGE_SIZE fit in isize
        // + BANK1_STARTING_FLASH_ADDRESS + raw_data_page_index * EARLGREY_PAGE_SIZE is a valid
        // flash address
        let expected_flash_address = unsafe {
            BANK1_STARTING_FLASH_ADDRESS
                .add_unchecked(raw_data_page_index * EARLGREY_PAGE_SIZE.get())
        };
        check_to_flash_ptr!(data_page_position, expected_flash_address);

        let raw_data_page_index = MAX_DATA_PAGE_INDEX.get();
        let data_page_position = DataPagePosition::Bank1(DataPageIndex::new(raw_data_page_index));
        // SAFETY:
        //
        // + raw_data_page_index * EARLGREY_PAGE_SIZE fit in isize
        // + BANK1_STARTING_FLASH_ADDRESS + raw_data_page_index * EARLGREY_PAGE_SIZE is a valid
        // flash address
        let expected_flash_address = unsafe {
            BANK1_STARTING_FLASH_ADDRESS
                // CAST: size_of(usize) = size_of(u32) > size_of(u8) on RV32I
                .add_unchecked(raw_data_page_index as usize * EARLGREY_PAGE_SIZE.get())
        };
        check_to_flash_ptr!(data_page_position, expected_flash_address);

        print_test_footer("DataPagePosition::to_flash_ptr()");
    }

    #[cfg_attr(test, test)]
    fn test_info0_page_position_to_flash_ptr() {
        print_test_header("Info0PagePosition::to_flash_ptr()");

        let info0_page_position = Info0PagePosition::Bank0(Info0PageIndex::Index0);
        let expected_flash_address = BANK0_STARTING_FLASH_ADDRESS;
        check_to_flash_ptr!(info0_page_position, expected_flash_address);

        let info0_page_position = Info0PagePosition::Bank0(Info0PageIndex::Index1);
        // SAFETY:
        //
        // + BANK0_STARTING_FLASH_ADDRESS + EARLGREY_PAGE_SIZE is a valid flash address
        let expected_flash_address =
            unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(EARLGREY_PAGE_SIZE.get()) };
        check_to_flash_ptr!(info0_page_position, expected_flash_address);

        let info0_page_position = Info0PagePosition::Bank0(Info0PageIndex::Index9);
        // SAFETY:
        //
        // + BANK0_STARTING_FLASH_ADDRESS + 9 * EARLGREY_PAGE_SIZE is a valid flash address
        let expected_flash_address =
            unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(9 * EARLGREY_PAGE_SIZE.get()) };
        check_to_flash_ptr!(info0_page_position, expected_flash_address);

        let info0_page_position = Info0PagePosition::Bank1(Info0PageIndex::Index0);
        let expected_flash_address = BANK1_STARTING_FLASH_ADDRESS;
        check_to_flash_ptr!(info0_page_position, expected_flash_address);

        let info0_page_position = Info0PagePosition::Bank1(Info0PageIndex::Index1);
        // SAFETY:
        //
        // + BANK1_STARTING_FLASH_ADDRESS + EARLGREY_PAGE_SIZE is a valid flash address
        let expected_flash_address =
            unsafe { BANK1_STARTING_FLASH_ADDRESS.add_unchecked(EARLGREY_PAGE_SIZE.get()) };
        check_to_flash_ptr!(info0_page_position, expected_flash_address);

        let info0_page_position = Info0PagePosition::Bank1(Info0PageIndex::Index9);
        // SAFETY:
        //
        // + 9 * EARLGREY_PAGE_SIZE fits in isze
        // + BANK1_STARTING_FLASH_ADDRESS + 9 * EARLGREY_PAGE_SIZE is a valid flash address
        let expected_flash_address =
            unsafe { BANK1_STARTING_FLASH_ADDRESS.add_unchecked(9 * EARLGREY_PAGE_SIZE.get()) };
        check_to_flash_ptr!(info0_page_position, expected_flash_address);

        print_test_footer("Info0PagePosition::to_flash_ptr()");
    }

    #[cfg_attr(test, test)]
    fn test_info1_page_position_to_flash_ptr() {
        print_test_header("Info1PagePosition::to_flash_ptr()");

        let info1_page_position = Info1PagePosition::Bank0(Info1PageIndex::Index0);
        let expected_flash_address = BANK0_STARTING_FLASH_ADDRESS;
        check_to_flash_ptr!(info1_page_position, expected_flash_address);

        let info1_page_position = Info1PagePosition::Bank1(Info1PageIndex::Index0);
        let expected_flash_address = BANK1_STARTING_FLASH_ADDRESS;
        check_to_flash_ptr!(info1_page_position, expected_flash_address);

        print_test_footer("Info1PagePosition::to_flash_ptr()");
    }

    #[cfg_attr(test, test)]
    fn test_info2_page_position_to_flash_ptr() {
        print_test_header("Info2PagePosition::to_flash_ptr()");

        let info2_page_position = Info2PagePosition::Bank0(Info2PageIndex::Index0);
        let expected_flash_address = BANK0_STARTING_FLASH_ADDRESS;
        check_to_flash_ptr!(info2_page_position, expected_flash_address);

        let info2_page_position = Info2PagePosition::Bank0(Info2PageIndex::Index1);
        // SAFETY:
        //
        // + BANK0_STARTING_FLASH_ADDRESS + EARLGREY_PAGE_SIZE is a valid flash address
        let expected_flash_address =
            unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(EARLGREY_PAGE_SIZE.get()) };
        check_to_flash_ptr!(info2_page_position, expected_flash_address);

        let info2_page_position = Info2PagePosition::Bank1(Info2PageIndex::Index0);
        let expected_flash_address = BANK1_STARTING_FLASH_ADDRESS;
        check_to_flash_ptr!(info2_page_position, expected_flash_address);

        let info2_page_position = Info2PagePosition::Bank1(Info2PageIndex::Index1);
        // SAFETY:
        //
        // + BANK1_STARTING_FLASH_ADDRESS + EARLGREY_PAGE_SIZE is still a valid flash address
        let expected_flash_address =
            unsafe { BANK1_STARTING_FLASH_ADDRESS.add_unchecked(EARLGREY_PAGE_SIZE.get()) };
        check_to_flash_ptr!(info2_page_position, expected_flash_address);

        print_test_footer("Info2PagePosition::to_flash_ptr()");
    }

    pub(in super::super) fn run_all() {
        test_data_page_position_to_flash_ptr();
        test_info0_page_position_to_flash_ptr();
        test_info1_page_position_to_flash_ptr();
        test_info2_page_position_to_flash_ptr();
    }
}
