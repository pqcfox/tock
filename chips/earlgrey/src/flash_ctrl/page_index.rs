// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::bank::DATA_PAGES_PER_BANK;

use crate::utils;

use core::num::NonZeroU8;

/// The maximum data page index within a bank
#[allow(unused)]
pub(super) const MAX_DATA_PAGE_INDEX: NonZeroU8 =
    // PANIC: 256 - 1 = 255 != 0
    // CAST: 256 - 1 = 255 is a valid u8 value
    utils::create_non_zero_u8((DATA_PAGES_PER_BANK.get() - 1) as u8);

/// The index of a data page index relative to a bank
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DataPageIndex(u8);

impl DataPageIndex {
    /// Create a new data page index corresponding to the given value, if valid.
    /// # Parameters
    /// + *value*: the corresponding value of the [DataPageIndex]
    ///
    /// # Return value
    ///
    /// The newly constructed [DataPageIndex]
    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    /// Returns the inner value of this data page index
    ///
    /// # Return value
    ///
    /// + [usize]: the inner value of this data page
    pub(super) const fn to_usize(self) -> usize {
        // CAST: width(usize) = width(u32) > width(u8) on RISC-V 32-bit platforms.
        self.0 as usize
    }
}

/// A info0 page index relative to a bank
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Info0PageIndex {
    Index0,
    Index1,
    Index2,
    Index3,
    Index4,
    Index5,
    Index6,
    Index7,
    Index8,
    Index9,
}

impl Info0PageIndex {
    /// [Info0PageIndex] constructor
    ///
    /// # Parameters
    ///
    /// + `page_number`: the page number that must represent a info0 page index
    ///
    /// # Return value
    ///
    /// + Ok(Info0PageIndex) if page_number < 10
    /// + Err(()) if page_number >= 10
    pub(super) const fn new(page_number: usize) -> Result<Self, ()> {
        match page_number {
            0 => Ok(Info0PageIndex::Index0),
            1 => Ok(Info0PageIndex::Index1),
            2 => Ok(Info0PageIndex::Index2),
            3 => Ok(Info0PageIndex::Index3),
            4 => Ok(Info0PageIndex::Index4),
            5 => Ok(Info0PageIndex::Index5),
            6 => Ok(Info0PageIndex::Index6),
            7 => Ok(Info0PageIndex::Index7),
            8 => Ok(Info0PageIndex::Index8),
            9 => Ok(Info0PageIndex::Index9),
            _ => Err(()),
        }
    }

    /// Convert the enum to the underlying integral type
    ///
    /// # Return value
    ///
    /// The underlying integral value
    pub(super) const fn to_usize(self) -> usize {
        // Since the enum is represented as u8, the cast is safe
        self as usize
    }

    /// Return the next index following this index.
    ///
    /// Indices are sorted ascendantly: Index0, Index1, ... Index9
    ///
    /// # Return value
    ///
    /// + Some(index) => self != Index9
    /// + None => self == Index9
    pub(super) const fn next_index(self) -> Option<Self> {
        match self {
            Info0PageIndex::Index0 => Some(Info0PageIndex::Index1),
            Info0PageIndex::Index1 => Some(Info0PageIndex::Index2),
            Info0PageIndex::Index2 => Some(Info0PageIndex::Index3),
            Info0PageIndex::Index3 => Some(Info0PageIndex::Index4),
            Info0PageIndex::Index4 => Some(Info0PageIndex::Index5),
            Info0PageIndex::Index5 => Some(Info0PageIndex::Index6),
            Info0PageIndex::Index6 => Some(Info0PageIndex::Index7),
            Info0PageIndex::Index7 => Some(Info0PageIndex::Index8),
            Info0PageIndex::Index8 => Some(Info0PageIndex::Index9),
            Info0PageIndex::Index9 => None,
        }
    }
}

/// A info1 page index relative to a bank
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Info1PageIndex {
    Index0,
}

impl Info1PageIndex {
    /// [Info1PageIndex] constructor
    ///
    /// # Parameters
    ///
    /// + `page_number`: the page number that must represent a info1 page index
    ///
    /// # Return value
    ///
    /// + Ok(page_index) if page_number < 1
    /// + Err(()) if page_number >= 1
    pub(super) const fn new(page_number: usize) -> Result<Self, ()> {
        match page_number {
            0 => Ok(Info1PageIndex::Index0),
            _ => Err(()),
        }
    }

    /// Convert the page index to its underlying integral value
    ///
    /// # Return value
    ///
    /// The underlying integral value
    pub(super) const fn to_usize(self) -> usize {
        // Since the enum is represented as u8, the cast is safe
        self as usize
    }

    /// Return the next index following this index.
    ///
    /// Indices are sorted ascendantly: Index0
    ///
    /// # Return value
    ///
    /// + None => self == Index0
    pub(super) const fn next_index(self) -> Option<Self> {
        match self {
            Info1PageIndex::Index0 => None,
        }
    }
}

/// A info2 page index relative to a bank
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Info2PageIndex {
    Index0,
    Index1,
}

impl Info2PageIndex {
    /// [Info2PageIndex] constructor
    ///
    /// # Parameters
    ///
    /// + `page_number`: the page number that must represent a info2 page index
    ///
    /// # Return value
    ///
    /// + Ok(Info1PageIndex) if page_number < 2
    /// + Err(()) if page_number >= 2
    pub(super) const fn new(page_number: usize) -> Result<Self, ()> {
        match page_number {
            0 => Ok(Info2PageIndex::Index0),
            1 => Ok(Info2PageIndex::Index1),
            _ => Err(()),
        }
    }

    /// Convert the enum to the underlying integral type
    ///
    /// # Return value
    ///
    /// The underlying integral value
    pub(super) const fn to_usize(self) -> usize {
        // Since the enum is represented as u8, the cast is safe
        self as usize
    }

    /// Return the next index following this index.
    ///
    /// Indices are sorted ascendantly: Index0, Index1.
    ///
    /// # Return value
    ///
    /// + Some(index) => self != Index1
    /// + None => self == Index1
    pub(super) const fn next_index(self) -> Option<Self> {
        match self {
            Info2PageIndex::Index0 => Some(Info2PageIndex::Index1),
            Info2PageIndex::Index1 => None,
        }
    }
}

#[cfg(feature = "test_flash_ctrl")]
pub(in super::super) mod tests {
    use super::super::tests::{print_test_footer, print_test_header};
    use super::{
        DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex, MAX_DATA_PAGE_INDEX,
    };

    macro_rules! test_page_number_to_data_index_conversion {
        ($page_number:expr, $expected_data_page_index:expr) => {{
            let data_page_index = DataPageIndex::new($page_number);
            assert_eq!(
                $expected_data_page_index, data_page_index,
                "Expected page index {:?}, got {:?}",
                $expected_data_page_index, data_page_index
            );
        }};
    }

    macro_rules! test_successful_page_number_to_info0_index_conversion {
        ($page_number:expr, $expected_info0_page_index:expr) => {
            match Info0PageIndex::new($page_number) {
                Ok(info0_page_index) => {
                    assert_eq!(
                        $expected_info0_page_index, info0_page_index,
                        "Expected page index {:?}, got {:?}",
                        $expected_info0_page_index, info0_page_index
                    );
                }
                Err(()) => panic!(
                    "Converting page number {} to a info0 page index must succeed",
                    $page_number
                ),
            };
        };
    }

    macro_rules! test_unsuccessful_page_number_to_info0_index_conversion {
        ($page_number:expr) => {{
            let info0_page_index_result = Info0PageIndex::new($page_number);
            assert!(
                info0_page_index_result.is_err(),
                "Converting page number {} to a info0 page index must fail",
                $page_number
            );
        }};
    }

    macro_rules! test_successful_page_number_to_info1_index_conversion {
        ($page_number:expr, $expected_info1_page_index:expr) => {
            match Info1PageIndex::new($page_number) {
                Ok(info1_page_index) => {
                    assert_eq!(
                        $expected_info1_page_index, info1_page_index,
                        "Expected page index {:?}, got {:?}",
                        $expected_info1_page_index, info1_page_index
                    );
                }
                Err(()) => panic!(
                    "Converting page number {} to a info1 page index must succeed",
                    $page_number
                ),
            };
        };
    }

    macro_rules! test_unsuccessful_page_number_to_info1_index_conversion {
        ($page_number:expr) => {{
            let info1_page_index_result = Info1PageIndex::new($page_number);
            assert!(
                info1_page_index_result.is_err(),
                "Converting page number {} to a info1 page index must fail",
                $page_number
            );
        }};
    }

    macro_rules! test_successful_page_number_to_info2_index_conversion {
        ($page_number:expr, $expected_info2_page_index:expr) => {
            match Info2PageIndex::new($page_number) {
                Ok(info2_page_index) => {
                    assert_eq!(
                        $expected_info2_page_index, info2_page_index,
                        "Expected page index {:?}, got {:?}",
                        $expected_info2_page_index, info2_page_index
                    );
                }
                Err(()) => panic!(
                    "Converting page number {} to a info2 page index must succeed",
                    $page_number
                ),
            };
        };
    }

    macro_rules! test_unsuccessful_page_number_to_info2_index_conversion {
        ($page_number:expr) => {{
            let info2_page_index_result = Info2PageIndex::new($page_number);
            assert!(
                info2_page_index_result.is_err(),
                "Converting page number {} to a info2 page index must fail",
                $page_number
            );
        }};
    }

    #[cfg_attr(test, test)]
    fn test_data_page_index_constructor() {
        print_test_header("DataPageIndex::new()");

        let page_number: u8 = 0;
        let expected_data_page_index = DataPageIndex::new(page_number);
        test_page_number_to_data_index_conversion!(page_number, expected_data_page_index);

        let page_number: u8 = 1;
        let expected_data_page_index = DataPageIndex::new(page_number);
        test_page_number_to_data_index_conversion!(page_number, expected_data_page_index);

        let page_number: u8 = MAX_DATA_PAGE_INDEX.get();
        let expected_data_page_index = DataPageIndex::new(page_number);
        test_page_number_to_data_index_conversion!(page_number, expected_data_page_index);

        print_test_footer("DataPageIndex::new()");
    }

    #[cfg_attr(test, test)]
    fn test_info0_page_index_constructor() {
        print_test_header("Info0PageIndex::new()");

        let page_number = 0;
        let expected_info0_page_index = Info0PageIndex::Index0;
        test_successful_page_number_to_info0_index_conversion!(
            page_number,
            expected_info0_page_index
        );

        let page_number = 1;
        let expected_info0_page_index = Info0PageIndex::Index1;
        test_successful_page_number_to_info0_index_conversion!(
            page_number,
            expected_info0_page_index
        );

        let page_number = 9;
        // SAFETY: 9 is a valid page number
        let expected_info0_page_index = Info0PageIndex::Index9;
        test_successful_page_number_to_info0_index_conversion!(
            page_number,
            expected_info0_page_index
        );

        let page_number = 10;
        test_unsuccessful_page_number_to_info0_index_conversion!(page_number);

        let page_number = 11;
        test_unsuccessful_page_number_to_info0_index_conversion!(page_number);

        print_test_footer("Info0PageIndex::new()");
    }

    #[cfg_attr(test, test)]
    fn test_info1_page_index_constructor() {
        print_test_header("Info1PageIndex::new()");

        let page_number = 0;
        let expected_info1_page_index = Info1PageIndex::Index0;
        test_successful_page_number_to_info1_index_conversion!(
            page_number,
            expected_info1_page_index
        );

        let page_number = 1;
        test_unsuccessful_page_number_to_info1_index_conversion!(page_number);

        let page_number = 2;
        test_unsuccessful_page_number_to_info1_index_conversion!(page_number);

        print_test_footer("Info1PageIndex::new()");
    }

    #[cfg_attr(test, test)]
    fn test_info2_page_index_constructor() {
        print_test_header("Info2PageIndex::new()");

        let page_number = 0;
        let expected_info2_page_index = Info2PageIndex::Index0;
        test_successful_page_number_to_info2_index_conversion!(
            page_number,
            expected_info2_page_index
        );

        let page_number = 1;
        let expected_info2_page_index = Info2PageIndex::Index1;
        test_successful_page_number_to_info2_index_conversion!(
            page_number,
            expected_info2_page_index
        );

        let page_number = 2;
        test_unsuccessful_page_number_to_info2_index_conversion!(page_number);

        let page_number = 3;
        test_unsuccessful_page_number_to_info2_index_conversion!(page_number);

        print_test_footer("Info2PageIndex::new()");
    }

    pub(in super::super) fn run_all() {
        test_data_page_index_constructor();
        test_info0_page_index_constructor();
        test_info1_page_index_constructor();
        test_info2_page_index_constructor();
    }
}
