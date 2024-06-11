// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use super::flash_address::{FlashAddress, InvalidHostAddressError};
use super::flash_ctrl::{FlashCtrl, FLASH_HOST_STARTING_ADDRESS_OFFSET};
use super::memory_protection::{
    DataMemoryProtectionRegion, DataMemoryProtectionRegionIndex, EraseEnabledStatus,
    HighEnduranceEnabledStatus, Info0MemoryProtectionRegionIndex, Info1MemoryProtectionRegionIndex,
    Info2MemoryProtectionRegionIndex, InfoMemoryProtectionRegion, ReadEnabledStatus,
    WriteEnabledStatus,
};
use super::page_position::{
    DataPagePosition, Info0PagePosition, Info1PagePosition, Info2PagePosition, InfoPagePosition,
};

use super::page::EARLGREY_PAGE_SIZE;

use super::page_index::{
    DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex, MAX_DATA_PAGE_INDEX,
};

use super::bank::Bank;

use super::info_partition_type::InfoPartitionType;

use crate::uart::Uart;

use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::{interfaces::Readable, ReadWrite};
use kernel::ErrorCode;

use core::fmt::Write;
use core::ops::RangeInclusive;

// The position of an info2 page which has read, write and erase enabled.
const VALID_INFO2_PAGE_POSITION: Info2PagePosition =
    Info2PagePosition::new(Bank::Bank1, Info2PageIndex::Index1);
pub const VALID_INFO2_MEMORY_PROTECTION_REGION_INDEX: Info2MemoryProtectionRegionIndex =
    VALID_INFO2_PAGE_POSITION;

struct TestWriter {
    uart: OptionalCell<&'static Uart<'static>>,
}

impl TestWriter {
    fn set_uart(&self, uart: &'static Uart) {
        self.uart.set(uart);
    }
}

impl Write for TestWriter {
    fn write_str(&mut self, string: &str) -> core::fmt::Result {
        self.uart.map(|uart| uart.transmit_sync(string.as_bytes()));
        Ok(())
    }
}

static mut TEST_WRITER: TestWriter = TestWriter {
    uart: OptionalCell::empty(),
};

macro_rules! println {
    ($msg:expr) => ({
        // If tests are running on host, there is no underlying Tock kernel, so this function becomes a
        // NOP
        if !cfg!(test) {
            // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
            unsafe {
                // The result is ignored for simplicity
                let _ = TEST_WRITER.write_fmt(format_args!("{}\r\n", $msg));
            }
        }
    });
    ($fmt:expr, $($arg:tt)+) => ({
        // If tests are running on host, there is no underlying Tock kernel, so this function becomes a
        // NOP
        if !cfg!(test) {
            // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
            unsafe {
                // The result is ignored for simplicity
                let _ = TEST_WRITER.write_fmt(format_args!("{}\r\n", format_args!($fmt, $($arg)+)));
            }
        }
    });
}

pub(super) fn print_test_header(message: &str) {
    println!("STARTING TEST: {}", message);
}

pub(super) fn print_test_footer(message: &str) {
    println!("FINISHED TEST: {}", message);
}

fn convert_address_to_page_position(
    host_address: *const u8,
) -> Result<DataPagePosition, InvalidHostAddressError> {
    let flash_address = FlashAddress::new_from_host_address(host_address)?;

    Ok(DataPagePosition::new_from_flash_address(flash_address))
}

pub fn convert_flash_slice_to_page_position_range(
    flash_test_memory: &[u8],
) -> Result<RangeInclusive<DataPagePosition>, InvalidHostAddressError> {
    let address_range = flash_test_memory.as_ptr_range();
    let (start_address, end_address) = (address_range.start, address_range.end);

    let start_page_number = convert_address_to_page_position(start_address)?;
    let end_page_number = convert_address_to_page_position(end_address)?;

    Ok(RangeInclusive::new(start_page_number, end_page_number))
}

pub fn run_all(uart: &'static Uart<'static>) {
    // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
    unsafe { TEST_WRITER.set_uart(uart) };

    super::page_index::tests::run_all();
    super::page_position::tests::run_all();
    super::page::tests::run_all();
    super::memory_protection::tests::run_all();
    super::flash_address::tests::run_all();
    super::chunk::tests::run_all();
}
