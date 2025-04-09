// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::flash_address::FlashAddress;
use super::page_position::{DataPagePosition, InfoPagePosition};

use crate::registers::flash_ctrl_regs::FLASH_CTRL_PARAM_BYTES_PER_PAGE;
use crate::utils;

use core::num::NonZeroUsize;

/// Page size on Earlgrey
pub const EARLGREY_PAGE_SIZE: NonZeroUsize =
    // PANIC: 2048 != 0
    utils::create_non_zero_usize(FLASH_CTRL_PARAM_BYTES_PER_PAGE as usize);
/// Raw flash controller page
///
/// Raw flash controller are used to transmit data between the peripheral and other components such
/// as a capsule.
#[repr(C, align(4))]
pub struct RawFlashCtrlPage([u8; EARLGREY_PAGE_SIZE.get()]);

impl Default for RawFlashCtrlPage {
    fn default() -> Self {
        Self([0; EARLGREY_PAGE_SIZE.get()])
    }
}

impl AsMut<[u8]> for RawFlashCtrlPage {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut()
    }
}

impl AsMut<[u8; EARLGREY_PAGE_SIZE.get()]> for RawFlashCtrlPage {
    fn as_mut(&mut self) -> &mut [u8; EARLGREY_PAGE_SIZE.get()] {
        &mut self.0
    }
}

impl AsRef<[u8; EARLGREY_PAGE_SIZE.get()]> for RawFlashCtrlPage {
    fn as_ref(&self) -> &[u8; EARLGREY_PAGE_SIZE.get()] {
        &self.0
    }
}

/// Data flash controller page
pub(super) struct DataFlashCtrlPage<'a> {
    position: DataPagePosition,
    raw_page: &'a mut RawFlashCtrlPage,
}

impl<'a> DataFlashCtrlPage<'a> {
    /// [DataFlashCtrlPage] constructor
    ///
    /// # Parameters
    ///
    /// + `position`: data page position
    /// + `raw_page`: the underlying raw page
    pub(super) fn new(position: DataPagePosition, raw_page: &'a mut RawFlashCtrlPage) -> Self {
        Self { position, raw_page }
    }

    /// Return the page position
    ///
    /// # Return value
    ///
    /// [DataPagePosition]
    pub(super) fn get_position(&self) -> DataPagePosition {
        self.position
    }

    /// Return the starting address within the flash address space of this page
    ///
    /// # Return value
    ///
    /// The starting [FlashAddress]
    pub(super) fn get_starting_flash_address(&self) -> FlashAddress {
        self.position.to_flash_ptr()
    }

    /// Convert the data page to a raw page
    ///
    /// # Return value
    ///
    /// The underlying [RawFlashCtrlPage]
    #[allow(clippy::wrong_self_convention)]
    pub(super) fn to_raw_page(self) -> &'a mut RawFlashCtrlPage {
        self.raw_page
    }
}

impl AsMut<[u8]> for DataFlashCtrlPage<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.raw_page.as_mut()
    }
}

impl AsMut<[u8; EARLGREY_PAGE_SIZE.get()]> for DataFlashCtrlPage<'_> {
    fn as_mut(&mut self) -> &mut [u8; EARLGREY_PAGE_SIZE.get()] {
        self.raw_page.as_mut()
    }
}

/// Info flash controller page
#[allow(unused)]
pub(super) struct InfoFlashCtrlPage<'a> {
    page_position: InfoPagePosition,
    raw_page: &'a mut RawFlashCtrlPage,
}

#[allow(unused)]
impl<'a> InfoFlashCtrlPage<'a> {
    /// [InfoFlashCtrlPage] constructor
    ///
    /// # Parameters
    ///
    /// `position`: info page position
    /// `raw_page`: the underlying raw page
    ///
    /// # Return value
    ///
    /// The newly created [InfoPagePosition]
    pub(super) fn new(page_position: InfoPagePosition, raw_page: &'a mut RawFlashCtrlPage) -> Self {
        Self {
            page_position,
            raw_page,
        }
    }

    /// Get starting flash address
    ///
    /// # Return value
    ///
    /// The starting [FlashAddress]
    fn get_starting_flash_address(&self) -> FlashAddress {
        self.page_position.to_flash_ptr()
    }

    /// Get the info page position
    ///
    /// # Return value
    ///
    /// [InfoPagePosition] of this page
    pub(super) fn get_position(&self) -> InfoPagePosition {
        self.page_position
    }

    /// Convert the info page to a raw page
    ///
    /// # Return value
    ///
    /// The underlying [RawFlashCtrlPage]
    #[allow(clippy::wrong_self_convention)]
    pub(super) fn to_raw_page(self) -> &'a mut RawFlashCtrlPage {
        self.raw_page
    }
}

impl AsMut<[u8]> for InfoFlashCtrlPage<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.raw_page.as_mut()
    }
}

impl AsMut<[u8; EARLGREY_PAGE_SIZE.get()]> for InfoFlashCtrlPage<'_> {
    fn as_mut(&mut self) -> &mut [u8; EARLGREY_PAGE_SIZE.get()] {
        self.raw_page.as_mut()
    }
}

/// A flash controller page
pub(super) enum FlashCtrlPage<'a> {
    /// Data page
    DataPage(DataFlashCtrlPage<'a>),
    /// Info page
    InfoPage(InfoFlashCtrlPage<'a>),
}

impl<'a> FlashCtrlPage<'a> {
    /// [FlashCtrlPage] constructor
    ///
    /// Creates a new FlashCtrlPage from the given data page position and raw page
    ///
    /// # Parameters
    ///
    /// + `page_position`: the position of the data page
    /// + `raw_page`: the raw flash used to represent the data page content
    #[allow(unused)]
    pub(super) fn new_data_page(
        page_position: DataPagePosition,
        raw_page: &'a mut RawFlashCtrlPage,
    ) -> Self {
        FlashCtrlPage::DataPage(DataFlashCtrlPage::new(page_position, raw_page))
    }

    /// [FlashCtrlPage] constructor
    ///
    /// Creates a new FlashCtrlPage from the given info page position and raw page
    ///
    /// # Parameters
    ///
    /// + `page_position`: the position of the info page
    /// + `raw_page`: the raw flash used to represent the info page content
    // Remove the allow if used
    #[allow(unused)]
    pub(super) fn new_info_page(
        page_position: InfoPagePosition,
        raw_page: &'a mut RawFlashCtrlPage,
    ) -> Self {
        FlashCtrlPage::InfoPage(InfoFlashCtrlPage::new(page_position, raw_page))
    }

    /// Get the starting flash address
    ///
    /// # Return value
    ///
    /// The starting [FlashAddress]
    pub(super) fn get_starting_flash_address(&self) -> FlashAddress {
        match self {
            FlashCtrlPage::DataPage(data_page) => data_page.get_starting_flash_address(),
            FlashCtrlPage::InfoPage(info_page) => info_page.get_starting_flash_address(),
        }
    }

    /// Convert the flash page to a raw page
    ///
    /// # Return value
    ///
    /// The underlying [RawFlashCtrlPage]
    #[allow(clippy::wrong_self_convention)]
    pub(super) fn to_raw_page(self) -> &'a mut RawFlashCtrlPage {
        match self {
            FlashCtrlPage::DataPage(data_page) => data_page.to_raw_page(),
            FlashCtrlPage::InfoPage(info_page) => info_page.to_raw_page(),
        }
    }
}

impl AsMut<[u8]> for FlashCtrlPage<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            FlashCtrlPage::DataPage(data_page) => data_page.as_mut(),
            FlashCtrlPage::InfoPage(info_page) => info_page.as_mut(),
        }
    }
}

impl AsMut<[u8; EARLGREY_PAGE_SIZE.get()]> for FlashCtrlPage<'_> {
    fn as_mut(&mut self) -> &mut [u8; EARLGREY_PAGE_SIZE.get()] {
        match self {
            FlashCtrlPage::DataPage(data_page) => data_page.as_mut(),
            FlashCtrlPage::InfoPage(info_page) => info_page.as_mut(),
        }
    }
}

#[cfg(feature = "test_flash_ctrl")]
pub(super) mod tests {
    use super::super::bank::{BANK0_STARTING_FLASH_ADDRESS, BANK1_STARTING_FLASH_ADDRESS};
    use super::super::page_index::{DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex};
    use super::super::page_position::{
        DataPagePosition, Info0PagePosition, Info1PagePosition, Info2PagePosition, InfoPagePosition,
    };
    use super::super::tests::{print_test_footer, print_test_header};
    use super::EARLGREY_PAGE_SIZE;
    use super::{FlashCtrlPage, RawFlashCtrlPage};

    macro_rules! check_get_starting_flash_address {
        ($flash_ctrl_page:expr, $expected_flash_address:expr) => {{
            let actual_flash_address = $flash_ctrl_page.get_starting_flash_address();
            assert_eq!(
                $expected_flash_address, actual_flash_address,
                "Expected {:?}, got {:?}",
                $expected_flash_address, actual_flash_address
            );
        }};
    }

    #[cfg_attr(test, test)]
    fn test_get_starting_flash_address() {
        print_test_header("FlashCtrlPage::get_starting_flash_address()");

        let mut raw_flash_ctrl_page = RawFlashCtrlPage::default();
        let data_page_position = DataPagePosition::Bank0(DataPageIndex::new(1));
        let flash_ctrl_page =
            FlashCtrlPage::new_data_page(data_page_position, &mut raw_flash_ctrl_page);
        let offset = 1 * EARLGREY_PAGE_SIZE.get();
        // SAFETY:
        //
        // + BANK0_STARTING_FLASH_ADDRESS + offset is a valid flash address
        let expected_flash_address = unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(offset) };
        check_get_starting_flash_address!(flash_ctrl_page, expected_flash_address);

        let flash_ctrl_page = FlashCtrlPage::new_info_page(
            InfoPagePosition::Type0(Info0PagePosition::Bank1(Info0PageIndex::Index3)),
            &mut raw_flash_ctrl_page,
        );
        let offset = 3 * EARLGREY_PAGE_SIZE.get();
        // SAFETY:
        //
        // + BANK1_STARTING_FLASH_ADDRESS + offset is a valid flash address
        let expected_flash_address = unsafe { BANK1_STARTING_FLASH_ADDRESS.add_unchecked(offset) };
        check_get_starting_flash_address!(flash_ctrl_page, expected_flash_address);

        let flash_ctrl_page = FlashCtrlPage::new_info_page(
            InfoPagePosition::Type1(Info1PagePosition::Bank0(Info1PageIndex::Index0)),
            &mut raw_flash_ctrl_page,
        );
        let offset = 0;
        // SAFETY:
        //
        // + BANK0_STARTING_FLASH_ADDRESS + offset is a valid flash address
        let expected_flash_address = unsafe { BANK0_STARTING_FLASH_ADDRESS.add_unchecked(offset) };
        check_get_starting_flash_address!(flash_ctrl_page, expected_flash_address);

        let flash_ctrl_page = FlashCtrlPage::new_info_page(
            InfoPagePosition::Type2(Info2PagePosition::Bank1(Info2PageIndex::Index1)),
            &mut raw_flash_ctrl_page,
        );
        let offset = 1 * EARLGREY_PAGE_SIZE.get();
        // SAFETY:
        //
        // + BANK1_STARTING_FLASH_ADDRESS + offset is a valid flash address
        let expected_flash_address = unsafe { BANK1_STARTING_FLASH_ADDRESS.add_unchecked(offset) };
        check_get_starting_flash_address!(flash_ctrl_page, expected_flash_address);

        print_test_footer("FlashCtrlPage::get_starting_flash_address()");
    }

    pub(in super::super) fn run_all() {
        test_get_starting_flash_address();
    }
}
