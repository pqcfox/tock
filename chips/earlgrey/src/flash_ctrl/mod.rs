// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

mod bank;
mod chunk;
mod fifo_level;
mod flash_address;
mod flash_ctrl;
mod info_partition_type;
mod memory_protection;
mod page;
mod page_index;
mod page_position;
#[cfg(feature = "test_flash_ctrl")]
pub mod tests;

pub use bank::{Bank, DATA_PAGES_PER_BANK};
pub use flash_address::FlashAddress;
pub use flash_ctrl::FlashCtrl;
pub use memory_protection::{
    DataMemoryProtectionRegionBase, DataMemoryProtectionRegionIndex,
    DataMemoryProtectionRegionSize, DefaultMemoryProtectionRegion,
    Info0MemoryProtectionRegionIndex, Info1MemoryProtectionRegionIndex,
    Info2MemoryProtectionRegionIndex, MemoryProtectionConfiguration,
};
pub use page::{RawFlashCtrlPage, EARLGREY_PAGE_SIZE};
pub use page_index::{DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex};
pub use page_position::DataPagePosition;
