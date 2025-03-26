// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::bank::{Bank, DATA_PAGES_PER_BANK};
use super::flash_address::FlashAddress;
use super::page_index::{DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex};
use super::page_position::{
    DataPagePosition, Info0PagePosition, Info1PagePosition, Info2PagePosition,
};

use core::num::NonZeroU16;

/// The status of read access.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ReadEnabledStatus {
    /// Read is disabled
    Disabled,
    /// Read is enabled
    Enabled,
}

/// The status of write access
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum WriteEnabledStatus {
    /// Write is disabled
    Disabled,
    /// Write is enabled
    Enabled,
}

/// The status of erase access
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum EraseEnabledStatus {
    /// Erase is disabled
    Disabled,
    /// Erase is enabled
    Enabled,
}

/// The status of memory scrambling
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum ScrambleEnabledStatus {
    /// Scramble is disabled
    Disabled,
    /// Scramble is enabled
    Enabled,
}

/// The status of ECC
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum EccEnabledStatus {
    /// ECC is disabled
    Disabled,
    /// ECC is enabled
    Enabled,
}

/// The status of high endurance
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum HighEnduranceEnabledStatus {
    /// High endurance is disabled
    Disabled,
    /// High endurance is enabled
    Enabled,
}

/// The configuration of a memory protection region
#[derive(PartialEq, Eq, Debug)]
struct MemoryProtectionRegionConfiguration {
    read_enabled: ReadEnabledStatus,
    write_enabled: WriteEnabledStatus,
    erase_enabled: EraseEnabledStatus,
    scramble_enabled: ScrambleEnabledStatus,
    ecc_enabled: EccEnabledStatus,
    high_endurance_enabled: HighEnduranceEnabledStatus,
}

impl MemoryProtectionRegionConfiguration {
    /// [MemoryProtectionRegionConfiguration] constructor
    ///
    /// # Return value
    ///
    /// A new instance of [MemoryProtectionRegionConfiguration] that has:
    ///
    /// + read disabled
    /// + write disabled
    /// + erase disabled
    /// + high endurance disabled
    pub(super) const fn new() -> Self {
        Self {
            read_enabled: ReadEnabledStatus::Disabled,
            write_enabled: WriteEnabledStatus::Disabled,
            erase_enabled: EraseEnabledStatus::Disabled,
            scramble_enabled: ScrambleEnabledStatus::Disabled,
            ecc_enabled: EccEnabledStatus::Disabled,
            high_endurance_enabled: HighEnduranceEnabledStatus::Disabled,
        }
    }

    /// Enable read
    fn enable_read(&mut self) {
        self.read_enabled = ReadEnabledStatus::Enabled;
    }

    /// Check whether read is enabled
    ///
    /// # Return value
    ///
    /// [ReadEnabledStatus] indicating read status
    fn is_read_enabled(&self) -> ReadEnabledStatus {
        self.read_enabled
    }

    /// Enable write
    fn enable_write(&mut self) {
        self.write_enabled = WriteEnabledStatus::Enabled;
    }

    /// Check whether write is enabled
    ///
    /// # Return value
    ///
    /// [WriteEnabledStatus] indicating write status
    fn is_write_enabled(&self) -> WriteEnabledStatus {
        self.write_enabled
    }

    /// Enable erase
    fn enable_erase(&mut self) {
        self.erase_enabled = EraseEnabledStatus::Enabled;
    }

    /// Check whether erase is enabled
    ///
    /// # Return value
    ///
    /// [EraseEnabledStatus] indicating erase status
    fn is_erase_enabled(&self) -> EraseEnabledStatus {
        self.erase_enabled
    }

    /// Enable memory scrambling
    fn enable_scramble(&mut self) {
        self.scramble_enabled = ScrambleEnabledStatus::Enabled;
    }

    /// Check whether memory scrambling is enabled
    ///
    /// # Return value
    ///
    /// [ScrambleEnabledStatus] indicating memory scrambling status
    fn is_scramble_enabled(&self) -> ScrambleEnabledStatus {
        self.scramble_enabled
    }

    /// Enable ECC
    fn enable_ecc(&mut self) {
        self.ecc_enabled = EccEnabledStatus::Enabled;
    }

    /// Check whether ECC is enabled
    ///
    /// # Return value
    ///
    /// [EccEnabledStatus] indicating ECC status
    fn is_ecc_enabled(&self) -> EccEnabledStatus {
        self.ecc_enabled
    }

    /// Enable high endurance
    fn enable_high_endurance(&mut self) {
        self.high_endurance_enabled = HighEnduranceEnabledStatus::Enabled;
    }

    /// Check whether high endurance is enabled
    ///
    /// # Return value
    ///
    /// [HighEnduranceEnabledStatus] indicating high endurance status
    fn is_high_endurance_enabled(&self) -> HighEnduranceEnabledStatus {
        self.high_endurance_enabled
    }
}

/// Default memory protection configuration.
///
/// If an address range is not covered by any memory protection region, the default memory
/// protection configuration is applied.
pub struct DefaultMemoryProtectionRegion {
    configuration: MemoryProtectionRegionConfiguration,
}

impl DefaultMemoryProtectionRegion {
    /// [DefaultMemoryProtectionRegion] constructor.
    ///
    /// # Return value
    ///
    /// An instance of [DefaultMemoryProtectionRegion] that has:
    ///
    /// + read disabled
    /// + write disabled
    /// + erase disabled
    /// + high endurance disabled
    pub fn new() -> Self {
        Self {
            configuration: MemoryProtectionRegionConfiguration::new(),
        }
    }

    /// Enable read
    pub fn enable_read(mut self) -> Self {
        self.configuration.enable_read();
        self
    }

    /// Check whether read is enabled
    ///
    /// # Return value
    ///
    /// [ReadEnabledStatus] indicating read status
    pub(super) fn is_read_enabled(&self) -> ReadEnabledStatus {
        self.configuration.is_read_enabled()
    }

    /// Enable write
    pub fn enable_write(mut self) -> Self {
        self.configuration.enable_write();
        self
    }

    /// Check whether write is enabled
    ///
    /// # Return value
    ///
    /// [WriteEnabledStatus] indicating write status
    pub(super) fn is_write_enabled(&self) -> WriteEnabledStatus {
        self.configuration.is_write_enabled()
    }

    /// Enable erase
    pub fn enable_erase(mut self) -> Self {
        self.configuration.enable_erase();
        self
    }

    /// Check whether erase is enabled
    ///
    /// # Return value
    ///
    /// [EraseEnabledStatus] indicating erase status
    pub(super) fn is_erase_enabled(&self) -> EraseEnabledStatus {
        self.configuration.is_erase_enabled()
    }

    /// Enable memory scrambling
    pub fn enable_scramble(mut self) -> Self {
        self.configuration.enable_scramble();
        self
    }

    /// Check whether memory scrambling is enabled
    ///
    /// # Return value
    ///
    /// [ScrambleEnabledStatus] indicating memory scrambling status
    pub(super) fn is_scramble_enabled(&self) -> ScrambleEnabledStatus {
        self.configuration.is_scramble_enabled()
    }

    /// Enable ECC
    pub fn enable_ecc(mut self) -> Self {
        self.configuration.enable_ecc();
        self
    }

    /// Check whether ECC is enabled
    ///
    /// # Return value
    ///
    /// [EccEnabledStatus] indicating ECC status
    pub(super) fn is_ecc_enabled(&self) -> EccEnabledStatus {
        self.configuration.is_ecc_enabled()
    }

    /// Enable high endurance
    pub fn enable_high_endurance(mut self) -> Self {
        self.configuration.enable_high_endurance();
        self
    }

    /// Check whether high endurance is enabled
    ///
    /// # Return value
    ///
    /// [HighEnduranceEnabledStatus] indicating high endurance status
    pub(super) fn is_high_endurance_enabled(&self) -> HighEnduranceEnabledStatus {
        self.configuration.is_high_endurance_enabled()
    }
}

/// Indicates whether the memory protection region is disabled/enabled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum MemoryProtectionRegionStatus {
    /// The memory protection region is disabled
    Disabled,
    /// The memory protection region is enabled
    Enabled,
}

/// The base of data memory protection region.
///
/// The memory area covered by a data memory protection region is defined by:
///
/// + the starting address of the memory protection region (base)
/// + the size of the memory protection region
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DataMemoryProtectionRegionBase(DataPagePosition);

impl DataMemoryProtectionRegionBase {
    /// [DataMemoryProtectionRegionBase] constructor
    ///
    /// # Parameters
    ///
    /// + page_index: the data page used as a base. The granularity of the base of a data memory
    /// protection region is page
    pub const fn new(page_index: DataPagePosition) -> Self {
        Self(page_index)
    }

    /// Get the base of a data memory protection region that would cover the given flash address
    ///
    /// # Parameters
    ///
    /// + `flash_address`: the flash address indicating the start of a data memory protection region
    ///
    /// # Return value
    ///
    /// + data_memory_protection_region_base: the [DataMemoryProtectionRegionBase] that would cover
    /// the given flash address
    const fn new_from_flash_address(flash_address: FlashAddress) -> DataMemoryProtectionRegionBase {
        Self(DataPagePosition::new_from_flash_address(flash_address))
    }

    /// Return the size of a data memory protection region that would cover regions
    /// [self; other_base].
    ///
    /// The computed size is for both-end inclusive range, i.e. calling self.subtract(self) will
    /// return DataMemoryProtectionRegionSize::Size1.
    ///
    /// # Parameters
    ///
    /// + other_base: the right-end of the range
    ///
    /// # Return value
    ///
    /// + Ok(data_memory_protection_region_size): the size of a data memory protection region that
    /// would cover [self; other_base] pages
    /// + Err(()): other_base < self
    fn subtract(
        self,
        other_base: DataMemoryProtectionRegionBase,
    ) -> Result<DataMemoryProtectionRegionSize, ()> {
        let linear_index = match self.inner() {
            DataPagePosition::Bank0(page_index) => page_index.to_usize(),
            DataPagePosition::Bank1(page_index) => {
                DATA_PAGES_PER_BANK.get() + page_index.to_usize()
            }
        };

        let other_linear_index = match other_base.inner() {
            DataPagePosition::Bank0(page_index) => page_index.to_usize(),
            DataPagePosition::Bank1(page_index) => {
                DATA_PAGES_PER_BANK.get() + page_index.to_usize()
            }
        };

        if other_linear_index < linear_index {
            return Err(());
        }

        // PANIC:
        //
        // + other_linear_index - linear_index >= 0 because of the previous if statement [1]
        // + linear_index/other_linear_index <= 511 (bank1, data_page_index255) =>
        //   DATA_PAGES_PER_BANK + 255 = 256 + 255 = 511 [2]
        // + other_linear_index - linear_index <= max(other_linear_index) - min(linear_index) =
        //   511 - 0 = 511 [3]
        // + 1 <= other_linear_index - linear_index + 1 <= 512 from (1) and (3) [4]
        Ok(create_data_memory_protection_region_size(
            (other_linear_index - linear_index + 1) as u16,
        ))
    }

    /// Return the underlying data page position
    ///
    /// # Return value
    ///
    /// [DataPagePosition]
    pub(super) const fn inner(self) -> DataPagePosition {
        self.0
    }
}

/// The size of a data memory protection region in pages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DataMemoryProtectionRegionSize(NonZeroU16);

impl DataMemoryProtectionRegionSize {
    pub const fn new(value: NonZeroU16) -> Result<Self, ()> {
        if value.get() > 512 {
            Err(())
        } else {
            Ok(Self(value))
        }
    }

    /// Return the underlying integral value of the enum
    ///
    /// # Return value
    ///
    /// [usize] representing the underlying integral value
    pub(super) const fn inner(self) -> u16 {
        self.0.get()
    }
}

/// Helper function to create a data memory protection region size
///
/// # Panic
///
/// Panics if either value is 0 or greater than 512
fn create_data_memory_protection_region_size(value: u16) -> DataMemoryProtectionRegionSize {
    let raw_data_memory_protection_region_size = NonZeroU16::new(value).unwrap();
    DataMemoryProtectionRegionSize::new(raw_data_memory_protection_region_size).unwrap()
}

/// Default raw data memory protection region size
const DEFAULT_RAW_DATA_MEMORY_PROTECTION_REGION_SIZE: NonZeroU16 = match NonZeroU16::new(1) {
    Some(non_zero_u16) => non_zero_u16,
    None => unreachable!(),
};

/// Default data memory protection region size
const DEFAULT_DATA_MEMORY_PROTECTION_REGION_SIZE: DataMemoryProtectionRegionSize =
    match DataMemoryProtectionRegionSize::new(DEFAULT_RAW_DATA_MEMORY_PROTECTION_REGION_SIZE) {
        Ok(default_data_memory_protection_region_size) => {
            default_data_memory_protection_region_size
        }
        Err(()) => unreachable!(),
    };

/// A data memory protection region
#[derive(PartialEq, Eq, Debug)]
pub struct DataMemoryProtectionRegion {
    status: MemoryProtectionRegionStatus,
    base: DataMemoryProtectionRegionBase,
    size: DataMemoryProtectionRegionSize,
    configuration: MemoryProtectionRegionConfiguration,
}

impl DataMemoryProtectionRegion {
    /// [DataMemoryProtectionRegion] constructor
    ///
    /// # Return value
    ///
    /// A new instance of [DataMemoryProtectionRegion] that:
    ///
    /// + is disabled
    /// + base is set to bank 0, page 0
    /// + size is 1
    /// + read access disabled
    /// + write access disabled
    /// + erase access disabled
    /// + high endurance disabled
    pub(super) const fn new() -> Self {
        Self {
            status: MemoryProtectionRegionStatus::Disabled,
            base: DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(
                0,
            ))),
            size: DEFAULT_DATA_MEMORY_PROTECTION_REGION_SIZE,
            configuration: MemoryProtectionRegionConfiguration::new(),
        }
    }

    /// Enable the region
    fn enable(&mut self) {
        self.status = MemoryProtectionRegionStatus::Enabled;
    }

    /// Check whether the region is enabled
    ///
    /// # Return value
    ///
    /// [MemoryProtectionRegionStatus] indicating the status of the region
    pub(super) fn is_enabled(&self) -> MemoryProtectionRegionStatus {
        self.status
    }

    /// Set the base
    fn set_base(&mut self, base: DataMemoryProtectionRegionBase) {
        self.base = base;
    }

    /// Get the current base
    ///
    /// # Return value
    ///
    /// [DataMemoryProtectionRegionBase] indicating the configured region base
    pub(super) fn get_base(&self) -> DataMemoryProtectionRegionBase {
        self.base
    }

    /// Set the size
    fn set_size(&mut self, size: DataMemoryProtectionRegionSize) {
        self.size = size;
    }

    /// Get the current size
    ///
    /// # Return value
    ///
    /// [DataMemoryProtectionRegionSize] indicating the configured region size
    pub(super) fn get_size(&self) -> DataMemoryProtectionRegionSize {
        self.size
    }

    /// Make region readable
    pub(super) fn enable_read(&mut self) {
        self.configuration.enable_read();
    }

    /// Check whether the region is readable
    ///
    /// # Return value
    ///
    /// [ReadEnabledStatus] indicating the read access status
    pub(super) fn is_read_enabled(&self) -> ReadEnabledStatus {
        self.configuration.is_read_enabled()
    }

    /// Make region writeable
    pub(super) fn enable_write(&mut self) {
        self.configuration.enable_write();
    }

    /// Check whether the region is writeable
    ///
    /// # Return value
    ///
    /// [WriteEnabledStatus] indicating the write access status
    pub(super) fn is_write_enabled(&self) -> WriteEnabledStatus {
        self.configuration.is_write_enabled()
    }

    /// Make region erasable
    pub(super) fn enable_erase(&mut self) {
        self.configuration.enable_erase();
    }

    /// Check whether the region is erasable
    ///
    /// # Return value
    ///
    /// [EraseEnabledStatus] indicating the erase access status
    pub(super) fn is_erase_enabled(&self) -> EraseEnabledStatus {
        self.configuration.is_erase_enabled()
    }

    /// Enable memory scrambling for the region
    pub(super) fn enable_scramble(&mut self) {
        self.configuration.enable_scramble();
    }

    /// Check whether memory scrambling is enabled for the region
    ///
    /// [ScrambleEnabledStatus] indicating whether memory scrambling is enabled
    pub(super) fn is_scramble_enabled(&self) -> ScrambleEnabledStatus {
        self.configuration.is_scramble_enabled()
    }

    /// Enable ECC for the region
    pub(super) fn enable_ecc(&mut self) {
        self.configuration.enable_ecc();
    }

    /// Check whether ECC is enabled for the region
    ///
    /// [EccEnabledStatus] indicating whether ECC is enabled
    pub(super) fn is_ecc_enabled(&self) -> EccEnabledStatus {
        self.configuration.is_ecc_enabled()
    }

    /// Enable high endurance for the region
    pub(super) fn enable_high_endurance(&mut self) {
        self.configuration.enable_high_endurance();
    }

    /// Check whether high endurance is enabled for the region
    ///
    /// [HighEnduranceEnabledStatus] indicating whether high endurance is enabled
    pub(super) fn is_high_endurance_enabled(&self) -> HighEnduranceEnabledStatus {
        self.configuration.is_high_endurance_enabled()
    }
}

// This constant is used to initialize an array of DataMemoryProtectionRegion
const NEW_DATA_MEMORY_PROTECTION_REGION: DataMemoryProtectionRegion =
    DataMemoryProtectionRegion::new();

/// An info memory protection region
#[derive(PartialEq, Eq, Debug)]
pub struct InfoMemoryProtectionRegion {
    status: MemoryProtectionRegionStatus,
    configuration: MemoryProtectionRegionConfiguration,
}

impl InfoMemoryProtectionRegion {
    /// [InfoMemoryProtectionRegion] constructor
    ///
    /// # Return value
    ///
    /// A new instance of [InfoMemoryProtectionRegion] that:
    ///
    /// + is disabled
    /// + read access disabled
    /// + write access disabled
    /// + erase access disabled
    /// + high endurance disabled
    pub(super) const fn new() -> Self {
        Self {
            status: MemoryProtectionRegionStatus::Disabled,
            configuration: MemoryProtectionRegionConfiguration::new(),
        }
    }

    /// Enable the region
    pub(super) fn enable(&mut self) {
        self.status = MemoryProtectionRegionStatus::Enabled;
    }

    /// Check whether the region is enabled
    ///
    /// # Return value
    ///
    /// [MemoryProtectionRegionStatus] indicating the status of the region
    pub(super) fn is_enabled(&self) -> MemoryProtectionRegionStatus {
        self.status
    }

    /// Make region readable
    pub(super) fn enable_read(&mut self) {
        self.configuration.enable_read();
    }

    /// Check whether the region is readable
    ///
    /// # Return value
    ///
    /// [ReadEnabledStatus] indicating the read access status
    pub(super) fn is_read_enabled(&self) -> ReadEnabledStatus {
        self.configuration.is_read_enabled()
    }

    /// Make region writeable
    pub(super) fn enable_write(&mut self) {
        self.configuration.enable_write();
    }

    /// Check whether the region is writeable
    ///
    /// # Return value
    ///
    /// [WriteEnabledStatus] indicating the write access status
    pub(super) fn is_write_enabled(&self) -> WriteEnabledStatus {
        self.configuration.is_write_enabled()
    }

    /// Make region erasable
    pub(super) fn enable_erase(&mut self) {
        self.configuration.enable_erase();
    }

    /// Check whether the region is erasable
    ///
    /// # Return value
    ///
    /// [EraseEnabledStatus] indicating the erase access status
    pub(super) fn is_erase_enabled(&self) -> EraseEnabledStatus {
        self.configuration.is_erase_enabled()
    }

    /// Enable memory scrambling for the region
    pub(super) fn enable_scramble(&mut self) {
        self.configuration.enable_scramble();
    }

    /// Check whether memory scrambling is enabled for the region
    ///
    /// [ScrambleEnabledStatus] indicating whether memory scrambling is enabled
    pub(super) fn is_scramble_enabled(&self) -> ScrambleEnabledStatus {
        self.configuration.is_scramble_enabled()
    }

    /// Enable ECC for the region
    pub(super) fn enable_ecc(&mut self) {
        self.configuration.enable_ecc();
    }

    /// Check whether ECC is enabled for the region
    ///
    /// [EccEnabledStatus] indicating whether ECC is enabled
    pub(super) fn is_ecc_enabled(&self) -> EccEnabledStatus {
        self.configuration.is_ecc_enabled()
    }

    /// Enable high endurance for the region
    pub(super) fn enable_high_endurance(&mut self) {
        self.configuration.enable_high_endurance();
    }

    /// Check whether high endurance is enabled for the region
    ///
    /// [HighEnduranceEnabledStatus] indicating whether high endurance is enabled
    pub(super) fn is_high_endurance_enabled(&self) -> HighEnduranceEnabledStatus {
        self.configuration.is_high_endurance_enabled()
    }
}

// This constant is used to initialize an array of InfoMemoryProtectionRegion
const NEW_INFO_MEMORY_PROTECTION_REGION: InfoMemoryProtectionRegion =
    InfoMemoryProtectionRegion::new();

use crate::registers::flash_ctrl_regs::{
    FLASH_CTRL_PARAM_NUM_INFOS0, FLASH_CTRL_PARAM_NUM_INFOS1, FLASH_CTRL_PARAM_NUM_INFOS2,
    FLASH_CTRL_PARAM_NUM_REGIONS,
};

use core::num::NonZeroUsize;

const fn create_non_zero_usize(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(non_zero_usize) => non_zero_usize,
        None => panic!("Attempt to create invalid NonZeroUsize"),
    }
}

/// The number of data memory protection regions
pub(super) const NUMBER_DATA_MEMORY_PROTECTION_REGIONS: NonZeroUsize =
    // PANIC: 8 != 0
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    create_non_zero_usize(FLASH_CTRL_PARAM_NUM_REGIONS as usize);
/// The number of info0 memory protection regions per bank
pub(super) const NUMBER_INFO0_MEMORY_PROTECTION_REGIONS_PER_BANK: NonZeroUsize =
    // PANIC: 10 != 0
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    create_non_zero_usize(FLASH_CTRL_PARAM_NUM_INFOS0 as usize);
/// The number of info1 memory protection regions per bank
pub(super) const NUMBER_INFO1_MEMORY_PROTECTION_REGIONS_PER_BANK: NonZeroUsize =
    // PANIC: 1 != 0
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    create_non_zero_usize(FLASH_CTRL_PARAM_NUM_INFOS1 as usize);
/// The number of info2 memory protection regions per bank
pub(super) const NUMBER_INFO2_MEMORY_PROTECTION_REGIONS_PER_BANK: NonZeroUsize =
    // PANIC: 2 != 0
    // CAST: u32 == usize on RISC-V 32-bit platforms.
    create_non_zero_usize(FLASH_CTRL_PARAM_NUM_INFOS2 as usize);

/// The index used to identify a data memory protection region
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum DataMemoryProtectionRegionIndex {
    /// First data memory protection region
    Index0,
    /// Second data memory protection region
    Index1,
    /// Third data memory protection region
    Index2,
    /// Fourth data memory protection region
    Index3,
    /// Fifth data memory protection region
    Index4,
    /// Sixth data memory protection region
    Index5,
    /// Seventh data memory protection region
    Index6,
    /// Eighth data memory protection region
    Index7,
}

impl DataMemoryProtectionRegionIndex {
    /// Return the underlying integral value
    ///
    /// # Return value
    ///
    /// [usize] representing the underlying integral value
    pub(super) const fn inner(self) -> usize {
        // The cast is safe since the enum is marked as repr(usize)
        self as usize
    }

    /// Determine the index that follows this index.
    ///
    /// Indices are ordered ascendantly: Index0, Index1, ..., Index7
    ///
    /// # Return value
    ///
    /// + Some(index): the following index
    /// + None: no following index (self == Index7)
    fn next_index(self) -> Option<Self> {
        match self {
            DataMemoryProtectionRegionIndex::Index0 => {
                Some(DataMemoryProtectionRegionIndex::Index1)
            }
            DataMemoryProtectionRegionIndex::Index1 => {
                Some(DataMemoryProtectionRegionIndex::Index2)
            }
            DataMemoryProtectionRegionIndex::Index2 => {
                Some(DataMemoryProtectionRegionIndex::Index3)
            }
            DataMemoryProtectionRegionIndex::Index3 => {
                Some(DataMemoryProtectionRegionIndex::Index4)
            }
            DataMemoryProtectionRegionIndex::Index4 => {
                Some(DataMemoryProtectionRegionIndex::Index5)
            }
            DataMemoryProtectionRegionIndex::Index5 => {
                Some(DataMemoryProtectionRegionIndex::Index6)
            }
            DataMemoryProtectionRegionIndex::Index6 => {
                Some(DataMemoryProtectionRegionIndex::Index7)
            }
            DataMemoryProtectionRegionIndex::Index7 => None,
        }
    }
}

// There is one info memory protection configuration region per info page
/// The index used to identify an info0 memory protection region
pub type Info0MemoryProtectionRegionIndex = Info0PagePosition;
/// The index used to identify an info1 memory protection region
pub type Info1MemoryProtectionRegionIndex = Info1PagePosition;
/// The index used to identify an info2 memory protection region
pub type Info2MemoryProtectionRegionIndex = Info2PagePosition;

/// This macro implements automatically the function `next_index()` for info memory protection
/// indices
macro_rules! implement_next_index {
    ($info_memory_protection_index_type:ident, $info_page_index_type:ident) => {
        impl $info_memory_protection_index_type {
            fn next_index(self) -> Option<Self> {
                match self {
                    $info_memory_protection_index_type::Bank0(info_page_index) => {
                        match info_page_index.next_index() {
                            Some(next_info_page_index) => Some(
                                $info_memory_protection_index_type::Bank0(next_info_page_index),
                            ),
                            None => Some($info_memory_protection_index_type::Bank1(
                                $info_page_index_type::Index0,
                            )),
                        }
                    }
                    $info_memory_protection_index_type::Bank1(info_page_index) => {
                        match info_page_index.next_index() {
                            Some(next_info_page_index) => Some(
                                $info_memory_protection_index_type::Bank1(next_info_page_index),
                            ),
                            None => None,
                        }
                    }
                }
            }
        }
    };
}

implement_next_index!(Info0MemoryProtectionRegionIndex, Info0PageIndex);
implement_next_index!(Info1MemoryProtectionRegionIndex, Info1PageIndex);
implement_next_index!(Info2MemoryProtectionRegionIndex, Info2PageIndex);

/// An iterator over data memory protection region indices
pub(super) struct DataMemoryProtectionRegionIndexIterator(Option<DataMemoryProtectionRegionIndex>);

impl DataMemoryProtectionRegionIndexIterator {
    /// [DataMemoryProtectionRegionIndexIterator] constructor
    ///
    /// # Return value
    ///
    /// A new instance of [DataMemoryProtectionRegionIndexIterator].
    fn new() -> Self {
        Self(Some(DataMemoryProtectionRegionIndex::Index0))
    }
}

impl Iterator for DataMemoryProtectionRegionIndexIterator {
    type Item = DataMemoryProtectionRegionIndex;

    fn next(&mut self) -> Option<Self::Item> {
        let current_page_index = self.0;
        self.0 = self
            .0
            .map_or(None, DataMemoryProtectionRegionIndex::next_index);
        current_page_index
    }
}

/// Implement boilerplate code for info memory protection region index iterator
macro_rules! implement_info_memory_protection_region_index_iterator {
    ($name:ident, $memory_protection_region_index_type:ident, $page_index:ident) => {
        pub(super) struct $name(Option<$memory_protection_region_index_type>);

        impl $name {
            fn new() -> Self {
                Self(Some($memory_protection_region_index_type::new(
                    Bank::Bank0,
                    $page_index::Index0,
                )))
            }
        }

        impl Iterator for $name {
            type Item = $memory_protection_region_index_type;

            fn next(&mut self) -> Option<Self::Item> {
                let current_page_index = self.0;
                self.0 = self
                    .0
                    .map_or(None, $memory_protection_region_index_type::next_index);
                current_page_index
            }
        }
    };
}

implement_info_memory_protection_region_index_iterator!(
    Info0MemoryProtectionRegionIndexIterator,
    Info0MemoryProtectionRegionIndex,
    Info0PageIndex
);

implement_info_memory_protection_region_index_iterator!(
    Info1MemoryProtectionRegionIndexIterator,
    Info1MemoryProtectionRegionIndex,
    Info1PageIndex
);

implement_info_memory_protection_region_index_iterator!(
    Info2MemoryProtectionRegionIndexIterator,
    Info2MemoryProtectionRegionIndex,
    Info2PageIndex
);

/// The list of data memory protection regions
pub(super) struct DataMemoryProtectionRegionList {
    data_memory_protection_regions:
        [DataMemoryProtectionRegion; NUMBER_DATA_MEMORY_PROTECTION_REGIONS.get()],
}

impl DataMemoryProtectionRegionList {
    /// [DataMemoryProtectionRegionList]
    ///
    /// # Return value
    ///
    /// A new instance of [DataMemoryProtectionRegionList]
    fn new() -> Self {
        Self {
            data_memory_protection_regions: [NEW_DATA_MEMORY_PROTECTION_REGION;
                NUMBER_DATA_MEMORY_PROTECTION_REGIONS.get()],
        }
    }

    /// Return the data memory protection region corresponding to the given index
    ///
    /// # Parameters
    ///
    /// + `data_memory_protection_region_index`: the index used to identify the region
    ///
    /// # Return value
    ///
    /// A mutable reference to [DataMemoryProtectionRegion]
    fn get_mut(
        &mut self,
        data_memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> &mut DataMemoryProtectionRegion {
        let raw_index = data_memory_protection_region_index.inner();
        // PANIC: DataMemoryProtectionRegionIndex guarantees safe access to the data memory
        // protection region array
        self.data_memory_protection_regions
            .get_mut(raw_index)
            .unwrap()
    }

    /// Return the data memory protection region corresponding to the given index
    ///
    /// # Parameters
    ///
    /// + `data_memory_protection_region_index`: the index used to identify the region
    ///
    /// # Return value
    ///
    /// An immutable reference to [DataMemoryProtectionRegion]
    fn get(
        &self,
        data_memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> &DataMemoryProtectionRegion {
        let raw_index = data_memory_protection_region_index.inner();
        // PANIC: DataMemoryProtectionRegionIndex guarantees safe access to the data memory
        // protection region array
        self.data_memory_protection_regions.get(raw_index).unwrap()
    }

    /// Convert the list to an iterator over all regions
    ///
    /// # Return value
    ///
    /// A new instance of [DataMemoryProtectionRegionListIterator]
    pub(super) fn as_iterator(&self) -> DataMemoryProtectionRegionListIterator {
        DataMemoryProtectionRegionListIterator::new(self)
    }
}

/// Automatically implement an info memory protection region list
macro_rules! implement_info_memory_protection_region_list {
    ($name:ident, $size:expr, $memory_protection_region_index:ident, $info_memory_protection_region_list_iterator:ident) => {
        pub(super) struct $name {
            bank0: [InfoMemoryProtectionRegion; $size],
            bank1: [InfoMemoryProtectionRegion; $size],
        }

        impl $name {
            fn new() -> Self {
                Self {
                    bank0: [NEW_INFO_MEMORY_PROTECTION_REGION; $size],
                    bank1: [NEW_INFO_MEMORY_PROTECTION_REGION; $size],
                }
            }

            fn get_mut(
                &mut self,
                memory_protection_region_index: $memory_protection_region_index,
            ) -> &mut InfoMemoryProtectionRegion {
                match memory_protection_region_index {
                    // PANIC: the type of page index guarantees safe access to the bank array
                    $memory_protection_region_index::Bank0(page_index) => {
                        self.bank0.get_mut(page_index.to_usize()).unwrap()
                    }
                    // PANIC: the type of page index guarantees safe access to the bank array
                    $memory_protection_region_index::Bank1(page_index) => {
                        self.bank1.get_mut(page_index.to_usize()).unwrap()
                    }
                }
            }

            fn get(
                &self,
                memory_protection_region_index: $memory_protection_region_index,
            ) -> &InfoMemoryProtectionRegion {
                match memory_protection_region_index {
                    // PANIC: the type of page index guarantees safe access to the bank array
                    $memory_protection_region_index::Bank0(page_index) => {
                        self.bank0.get(page_index.to_usize()).unwrap()
                    }
                    // PANIC: the type of page index guarantees safe access to the bank array
                    $memory_protection_region_index::Bank1(page_index) => {
                        self.bank1.get(page_index.to_usize()).unwrap()
                    }
                }
            }

            pub(super) fn as_iterator(&self) -> $info_memory_protection_region_list_iterator {
                $info_memory_protection_region_list_iterator::new(self)
            }
        }
    };
}

implement_info_memory_protection_region_list!(
    Info0MemoryProtectionRegionList,
    NUMBER_INFO0_MEMORY_PROTECTION_REGIONS_PER_BANK.get(),
    Info0MemoryProtectionRegionIndex,
    Info0MemoryProtectionRegionListIterator
);

implement_info_memory_protection_region_list!(
    Info1MemoryProtectionRegionList,
    NUMBER_INFO1_MEMORY_PROTECTION_REGIONS_PER_BANK.get(),
    Info1MemoryProtectionRegionIndex,
    Info1MemoryProtectionRegionListIterator
);

implement_info_memory_protection_region_list!(
    Info2MemoryProtectionRegionList,
    NUMBER_INFO2_MEMORY_PROTECTION_REGIONS_PER_BANK.get(),
    Info2MemoryProtectionRegionIndex,
    Info2MemoryProtectionRegionListIterator
);

/// Automatically implement memory protection region list iterator
macro_rules! implement_memory_protection_region_list_iterator {
    (
        $name:ident,
        $index_iterator_type:ident,
        $list_type:ident,
        $info_memory_protection_region_index:ident,
        $memory_protection_region_type:ident
    ) => {
        pub(super) struct $name<'a> {
            region_index_iterator: $index_iterator_type,
            memory_protection_region_list: &'a $list_type,
        }

        impl<'a> $name<'a> {
            fn new(memory_protection_region_list: &'a $list_type) -> Self {
                Self {
                    region_index_iterator: $index_iterator_type::new(),
                    memory_protection_region_list,
                }
            }
        }

        impl<'a> Iterator for $name<'a> {
            type Item = (
                $info_memory_protection_region_index,
                &'a $memory_protection_region_type,
            );

            fn next(&mut self) -> Option<Self::Item> {
                self.region_index_iterator.next().map(|next_region_index| {
                    (
                        next_region_index,
                        self.memory_protection_region_list.get(next_region_index),
                    )
                })
            }
        }
    };
}

/// Automatically implement info memory protection region list iterator
macro_rules! implement_info_memory_protection_region_list_iterator {
    (
        $name:ident,
        $index_iterator_type:ident,
        $list_type:ident,
        $info_memory_protection_region_index:ident
    ) => {
        implement_memory_protection_region_list_iterator!(
            $name,
            $index_iterator_type,
            $list_type,
            $info_memory_protection_region_index,
            InfoMemoryProtectionRegion
        );
    };
}

implement_memory_protection_region_list_iterator!(
    DataMemoryProtectionRegionListIterator,
    DataMemoryProtectionRegionIndexIterator,
    DataMemoryProtectionRegionList,
    DataMemoryProtectionRegionIndex,
    DataMemoryProtectionRegion
);

implement_info_memory_protection_region_list_iterator!(
    Info0MemoryProtectionRegionListIterator,
    Info0MemoryProtectionRegionIndexIterator,
    Info0MemoryProtectionRegionList,
    Info0MemoryProtectionRegionIndex
);

implement_info_memory_protection_region_list_iterator!(
    Info1MemoryProtectionRegionListIterator,
    Info1MemoryProtectionRegionIndexIterator,
    Info1MemoryProtectionRegionList,
    Info1MemoryProtectionRegionIndex
);

implement_info_memory_protection_region_list_iterator!(
    Info2MemoryProtectionRegionListIterator,
    Info2MemoryProtectionRegionIndexIterator,
    Info2MemoryProtectionRegionList,
    Info2MemoryProtectionRegionIndex
);

/// Flash memory protection configuration
pub struct MemoryProtectionConfiguration {
    default_memory_protection_region: DefaultMemoryProtectionRegion,
    data_memory_protection_regions: DataMemoryProtectionRegionList,
    info0_memory_protection_regions: Info0MemoryProtectionRegionList,
    info1_memory_protection_regions: Info1MemoryProtectionRegionList,
    info2_memory_protection_regions: Info2MemoryProtectionRegionList,
}

impl MemoryProtectionConfiguration {
    /// [MemoryProtectionConfiguration] constructor
    ///
    /// # Return value
    ///
    /// A new instance of [MemoryProtectionConfiguration]
    pub fn new(default_memory_protection_region: DefaultMemoryProtectionRegion) -> Self {
        Self {
            default_memory_protection_region,
            data_memory_protection_regions: DataMemoryProtectionRegionList::new(),
            info0_memory_protection_regions: Info0MemoryProtectionRegionList::new(),
            info1_memory_protection_regions: Info1MemoryProtectionRegionList::new(),
            info2_memory_protection_regions: Info2MemoryProtectionRegionList::new(),
        }
    }

    /// Return the default memory protection region
    ///
    /// # Return value
    ///
    /// An immutable reference to [DefaultMemoryProtectionRegion]
    pub(super) fn get_default_memory_protection_region(&self) -> &DefaultMemoryProtectionRegion {
        &self.default_memory_protection_region
    }

    /// Return the data memory protection region corresponding to the given index
    ///
    /// # Return value
    ///
    /// A mutable reference to [DataMemoryProtectionRegion]
    pub(super) fn get_data_memory_protection_region_mut(
        &mut self,
        index: DataMemoryProtectionRegionIndex,
    ) -> &mut DataMemoryProtectionRegion {
        self.data_memory_protection_regions.get_mut(index)
    }

    /// Return the info0 memory protection region corresponding to the given index
    ///
    /// # Return value
    ///
    /// A mutable reference to [InfoMemoryProtectionRegion]
    pub(super) fn get_info0_memory_protection_region_mut(
        &mut self,
        index: Info0MemoryProtectionRegionIndex,
    ) -> &mut InfoMemoryProtectionRegion {
        self.info0_memory_protection_regions.get_mut(index)
    }

    /// Return the info1 memory protection region corresponding to the given index
    ///
    /// # Return value
    ///
    /// A mutable reference to [InfoMemoryProtectionRegion]
    pub(super) fn get_info1_memory_protection_region_mut(
        &mut self,
        index: Info1MemoryProtectionRegionIndex,
    ) -> &mut InfoMemoryProtectionRegion {
        self.info1_memory_protection_regions.get_mut(index)
    }

    /// Return the info2 memory protection region corresponding to the given index
    ///
    /// # Return value
    ///
    /// A mutable reference to [InfoMemoryProtectionRegion]
    pub(super) fn get_info2_memory_protection_region_mut(
        &mut self,
        index: Info2MemoryProtectionRegionIndex,
    ) -> &mut InfoMemoryProtectionRegion {
        self.info2_memory_protection_regions.get_mut(index)
    }

    /// Enable and configure a data memory protection region
    ///
    /// # Parameters
    ///
    /// + `index`: the index of the data region to be enabled and configured
    /// + `base`: the base to be configured for the region
    /// + `size`: the size to be configured for the region
    ///
    /// # Return value
    ///
    /// [DataMemoryProtectionRegionBuilder] used to configure the region
    pub fn enable_and_configure_data_region(
        mut self,
        index: DataMemoryProtectionRegionIndex,
        base: DataMemoryProtectionRegionBase,
        size: DataMemoryProtectionRegionSize,
    ) -> DataMemoryProtectionRegionBuilder {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable();
        memory_protection_region.set_base(base);
        memory_protection_region.set_size(size);
        DataMemoryProtectionRegionBuilder::new(index, self)
    }

    /// Enable and configure a data memory protection that would cover
    /// [starting_address; ending_address] flash area
    ///
    /// Note that if one address is not page-aligned, the method will round it down.
    ///
    /// # Parameters
    ///
    /// + `index`: the index of the data region to be enabled and configured
    /// + `starting_address`: the first address to be covered
    /// + `ending_address`: the last address to be covered
    pub fn enable_and_configure_data_region_from_pointers(
        self,
        index: DataMemoryProtectionRegionIndex,
        starting_address: FlashAddress,
        ending_address: FlashAddress,
    ) -> Result<DataMemoryProtectionRegionBuilder, ()> {
        let base = DataMemoryProtectionRegionBase::new_from_flash_address(starting_address);
        let end = DataMemoryProtectionRegionBase::new_from_flash_address(ending_address);
        let size = base.subtract(end)?;
        Ok(self.enable_and_configure_data_region(index, base, size))
    }

    /// Enable and configure a info0 memory protection region
    ///
    /// # Parameters
    ///
    /// + `index`: the index of the info0 region to be enabled and configured
    ///
    /// # Return value
    ///
    /// [Info0MemoryProtectionRegionBuilder] used to configure the region
    pub fn enable_and_configure_info0_region(
        mut self,
        index: Info0MemoryProtectionRegionIndex,
    ) -> Info0MemoryProtectionRegionBuilder {
        let info0_memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        info0_memory_protection_region.enable();
        Info0MemoryProtectionRegionBuilder::new(index, self)
    }

    /// Enable and configure a info1 memory protection region
    ///
    /// # Parameters
    ///
    /// + `index`: the index of the info1 region to be enabled and configured
    ///
    /// # Return value
    ///
    /// [Info1MemoryProtectionRegionBuilder] used to configure the region
    pub fn enable_and_configure_info1_region(
        mut self,
        index: Info1MemoryProtectionRegionIndex,
    ) -> Info1MemoryProtectionRegionBuilder {
        let info1_memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        info1_memory_protection_region.enable();
        Info1MemoryProtectionRegionBuilder::new(index, self)
    }

    /// Enable and configure a info2 memory protection region
    ///
    /// # Parameters
    ///
    /// + `index`: the index of the info2 region to be enabled and configured
    ///
    /// # Return value
    ///
    /// [Info2MemoryProtectionRegionBuilder] used to configure the region
    pub fn enable_and_configure_info2_region(
        mut self,
        index: Info2MemoryProtectionRegionIndex,
    ) -> Info2MemoryProtectionRegionBuilder {
        let info2_memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        info2_memory_protection_region.enable();
        Info2MemoryProtectionRegionBuilder::new(index, self)
    }

    /// Make data memory protection region readable
    fn enable_read_data(&mut self, index: DataMemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable_read();
    }

    /// Make data memory protection region writeable
    fn enable_write_data(&mut self, index: DataMemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable_write();
    }

    /// Make data memory protection region erasable
    fn enable_erase_data(&mut self, index: DataMemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable_erase();
    }

    /// Enable memory scrambling for data memory protection region
    fn enable_scramble_data(&mut self, index: DataMemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable_scramble();
    }

    /// Enable ECC for data memory protection region
    fn enable_ecc_data(&mut self, index: DataMemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable_ecc();
    }

    /// Enable high endurance for data memory protection region
    fn enable_high_endurance_data(&mut self, index: DataMemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_data_memory_protection_region_mut(index);
        memory_protection_region.enable_high_endurance();
    }

    /// Make info0 memory protection region readable
    fn enable_read_info0(&mut self, index: Info0MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        memory_protection_region.enable_read();
    }

    /// Make info0 memory protection region writeable
    fn enable_write_info0(&mut self, index: Info0MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        memory_protection_region.enable_write();
    }

    /// Make info0 memory protection region erasable
    fn enable_erase_info0(&mut self, index: Info0MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        memory_protection_region.enable_erase();
    }

    /// Enable memory scrambling for info0 memory protection region
    fn enable_scramble_info0(&mut self, index: Info0MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        memory_protection_region.enable_scramble();
    }

    /// Enable ECC for info0 memory protection region
    fn enable_ecc_info0(&mut self, index: Info0MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        memory_protection_region.enable_ecc();
    }

    /// Enable high endurance for info0 memory protection region
    fn enable_high_endurance_info0(&mut self, index: Info0MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info0_memory_protection_region_mut(index);
        memory_protection_region.enable_high_endurance();
    }

    /// Make info1 memory protection region readable
    fn enable_read_info1(&mut self, index: Info1MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        memory_protection_region.enable_read();
    }

    /// Make info1 memory protection region writeable
    fn enable_write_info1(&mut self, index: Info1MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        memory_protection_region.enable_write();
    }

    /// Make info1 memory protection region erasable
    fn enable_erase_info1(&mut self, index: Info1MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        memory_protection_region.enable_erase();
    }

    /// Enable memory scrambling for info1 memory protection region
    fn enable_scramble_info1(&mut self, index: Info1MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        memory_protection_region.enable_scramble();
    }

    /// Enable ECC for info1 memory protection region
    fn enable_ecc_info1(&mut self, index: Info1MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        memory_protection_region.enable_ecc();
    }

    /// Enable high endurance for info1 memory protection region
    fn enable_high_endurance_info1(&mut self, index: Info1MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info1_memory_protection_region_mut(index);
        memory_protection_region.enable_high_endurance();
    }

    /// Make info2 memory protection region readable
    fn enable_read_info2(&mut self, index: Info2MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        memory_protection_region.enable_read();
    }

    /// Make info2 memory protection region writeable
    fn enable_write_info2(&mut self, index: Info2MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        memory_protection_region.enable_write();
    }

    /// Make info2 memory protection region erasable
    fn enable_erase_info2(&mut self, index: Info2MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        memory_protection_region.enable_erase();
    }

    /// Enable memory scrambling for info2 memory protection region
    fn enable_scramble_info2(&mut self, index: Info2MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        memory_protection_region.enable_scramble();
    }

    /// Enable ECC for info2 memory protection region
    fn enable_ecc_info2(&mut self, index: Info2MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        memory_protection_region.enable_ecc();
    }

    /// Enable high endurance for info2 memory protection region
    fn enable_high_endurance_info2(&mut self, index: Info2MemoryProtectionRegionIndex) {
        let memory_protection_region = self.get_info2_memory_protection_region_mut(index);
        memory_protection_region.enable_high_endurance();
    }

    /// Return the list of data memory protection regions
    pub(super) fn get_data_memory_protection_regions(&self) -> &DataMemoryProtectionRegionList {
        &self.data_memory_protection_regions
    }

    /// Return the list of info0 memory protection regions
    pub(super) fn get_info0_memory_protection_regions(&self) -> &Info0MemoryProtectionRegionList {
        &self.info0_memory_protection_regions
    }

    /// Return the list of info1 memory protection regions
    pub(super) fn get_info1_memory_protection_regions(&self) -> &Info1MemoryProtectionRegionList {
        &self.info1_memory_protection_regions
    }

    /// Return the list of info2 memory protection regions
    pub(super) fn get_info2_memory_protection_regions(&self) -> &Info2MemoryProtectionRegionList {
        &self.info2_memory_protection_regions
    }
}

/// Builder used to configure a data memory protection region
pub struct DataMemoryProtectionRegionBuilder {
    index: DataMemoryProtectionRegionIndex,
    memory_protection_configuration: MemoryProtectionConfiguration,
}

impl DataMemoryProtectionRegionBuilder {
    /// [DataMemoryProtectionRegionBuilder] constructor
    ///
    /// # Parameters
    ///
    /// + `index`: index of the data memory protection region to be configured
    /// + `memory_protection_configuration`: flash memory protection configuration
    fn new(
        index: DataMemoryProtectionRegionIndex,
        memory_protection_configuration: MemoryProtectionConfiguration,
    ) -> Self {
        Self {
            index,
            memory_protection_configuration,
        }
    }

    /// Make region readable
    pub fn enable_read(mut self) -> DataMemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_read_data(self.index);
        self
    }

    /// Make region writeable
    pub fn enable_write(mut self) -> DataMemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_write_data(self.index);
        self
    }

    /// Make region erasable
    pub fn enable_erase(mut self) -> DataMemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_erase_data(self.index);
        self
    }

    /// Enable memory scrambling
    pub fn enable_scramble(mut self) -> DataMemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_scramble_data(self.index);
        self
    }

    /// Enable ECC
    pub fn enable_ecc(mut self) -> DataMemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_ecc_data(self.index);
        self
    }

    /// Enable high endurance
    pub fn enable_high_endurance(mut self) -> DataMemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_high_endurance_data(self.index);
        self
    }

    /// Finalize the region configuration
    pub fn finalize_region(self) -> MemoryProtectionConfiguration {
        self.memory_protection_configuration
    }
}

/// Builder used to configure info0 memory protection region
pub struct Info0MemoryProtectionRegionBuilder {
    index: Info0MemoryProtectionRegionIndex,
    memory_protection_configuration: MemoryProtectionConfiguration,
}

impl Info0MemoryProtectionRegionBuilder {
    /// [Info0MemoryProtectionRegionBuilder] constructor
    ///
    /// # Parameters
    ///
    /// + `index`: index of the info0 memory protection region to be configured
    /// + `memory_protection_configuration`: flash memory protection configuration
    fn new(
        index: Info0MemoryProtectionRegionIndex,
        memory_protection_configuration: MemoryProtectionConfiguration,
    ) -> Self {
        Self {
            index,
            memory_protection_configuration,
        }
    }

    /// Make region readable
    pub fn enable_read(mut self) -> Info0MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_read_info0(self.index);
        self
    }

    /// Make region writeable
    pub fn enable_write(mut self) -> Info0MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_write_info0(self.index);
        self
    }

    /// Make region erasable
    pub fn enable_erase(mut self) -> Info0MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_erase_info0(self.index);
        self
    }

    /// Enable memory scrambling
    pub fn enable_scramble(mut self) -> Info0MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_scramble_info0(self.index);
        self
    }

    /// Enable ECC
    pub fn enable_ecc(mut self) -> Info0MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_ecc_info0(self.index);
        self
    }

    /// Enable high endurance
    pub fn enable_high_endurance(mut self) -> Info0MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_high_endurance_info0(self.index);
        self
    }

    /// Finalize the region configuration
    pub fn finalize_region(self) -> MemoryProtectionConfiguration {
        self.memory_protection_configuration
    }
}

/// Builder used to configure info1 memory protection region
pub struct Info1MemoryProtectionRegionBuilder {
    index: Info1MemoryProtectionRegionIndex,
    memory_protection_configuration: MemoryProtectionConfiguration,
}

impl Info1MemoryProtectionRegionBuilder {
    /// [Info1MemoryProtectionRegionBuilder] constructor
    ///
    /// # Parameters
    ///
    /// + `index`: index of the info1 memory protection region to be configured
    /// + `memory_protection_configuration`: flash memory protection configuration
    fn new(
        index: Info1MemoryProtectionRegionIndex,
        memory_protection_configuration: MemoryProtectionConfiguration,
    ) -> Self {
        Self {
            index,
            memory_protection_configuration,
        }
    }

    /// Make region readable
    pub fn enable_read(mut self) -> Info1MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_read_info1(self.index);
        self
    }

    /// Make region writeable
    pub fn enable_write(mut self) -> Info1MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_write_info1(self.index);
        self
    }

    /// Make region erasable
    pub fn enable_erase(mut self) -> Info1MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_erase_info1(self.index);
        self
    }

    /// Enable memory scrambling
    pub fn enable_scramble(mut self) -> Info1MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_scramble_info1(self.index);
        self
    }

    /// Enable ECC
    pub fn enable_ecc(mut self) -> Info1MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_ecc_info1(self.index);
        self
    }

    /// Enable high endurance
    pub fn enable_high_endurance(mut self) -> Info1MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_high_endurance_info1(self.index);
        self
    }

    /// Finalize the region configuration
    pub fn finalize_region(self) -> MemoryProtectionConfiguration {
        self.memory_protection_configuration
    }
}

/// Builder used to configure info2 memory protection region
pub struct Info2MemoryProtectionRegionBuilder {
    index: Info2MemoryProtectionRegionIndex,
    memory_protection_configuration: MemoryProtectionConfiguration,
}

impl Info2MemoryProtectionRegionBuilder {
    /// [Info2MemoryProtectionRegionBuilder] constructor
    ///
    /// # Parameters
    ///
    /// + `index`: index of the info2 memory protection region to be configured
    /// + `memory_protection_configuration`: flash memory protection configuration
    fn new(
        index: Info2MemoryProtectionRegionIndex,
        memory_protection_configuration: MemoryProtectionConfiguration,
    ) -> Self {
        Self {
            index,
            memory_protection_configuration,
        }
    }

    /// Make region readable
    pub fn enable_read(mut self) -> Info2MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_read_info2(self.index);
        self
    }

    /// Make region writeable
    pub fn enable_write(mut self) -> Info2MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_write_info2(self.index);
        self
    }

    /// Make region erasable
    pub fn enable_erase(mut self) -> Info2MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_erase_info2(self.index);
        self
    }

    /// Enable memory scrambling
    pub fn enable_scramble(mut self) -> Info2MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_scramble_info2(self.index);
        self
    }

    /// Enable ECC
    pub fn enable_ecc(mut self) -> Info2MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_ecc_info2(self.index);
        self
    }

    /// Enable high endurance
    pub fn enable_high_endurance(mut self) -> Info2MemoryProtectionRegionBuilder {
        self.memory_protection_configuration
            .enable_high_endurance_info2(self.index);
        self
    }

    /// Finalize the region configuration
    pub fn finalize_region(self) -> MemoryProtectionConfiguration {
        self.memory_protection_configuration
    }
}

#[cfg(feature = "test_flash_ctrl")]
pub(super) mod tests {
    use super::*;
    use crate::flash_ctrl::tests::{print_test_footer, print_test_header};

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_configuration_constructor() {
        print_test_header("MemoryProtectionRegionConfiguration::new()");

        let memory_protection_region_configuration = MemoryProtectionRegionConfiguration::new();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region_configuration.is_read_enabled(),
            "Upon creation of memory protection region configuration, read must be disabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region_configuration.is_write_enabled(),
            "Upon creation of memory protection region configuration, write must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region_configuration.is_erase_enabled(),
            "Upon creation of memory protection region configuration, erase must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region_configuration.is_high_endurance_enabled(),
            "Upon creation of memory protection region configuration, high endurance must be disabled"
        );

        print_test_footer("MemoryProtectionRegionConfiguration::new()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_configuration_enable_read() {
        print_test_header("MemoryProtectionRegionConfiguration::enable_read()");

        let mut memory_protection_region_configuration = MemoryProtectionRegionConfiguration::new();
        memory_protection_region_configuration.enable_read();

        assert_eq!(
            ReadEnabledStatus::Enabled,
            memory_protection_region_configuration.is_read_enabled(),
            "MemoryProtectionRegionConfiguration::enable_read() must change the status of read access to enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region_configuration.is_write_enabled(),
            "MemoryProtectionRegionConfiguration::enable_read() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region_configuration.is_erase_enabled(),
            "MemoryProtectionRegionConfiguration::enable_read() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region_configuration.is_high_endurance_enabled(),
            "MemoryProtectionRegionConfiguration::enable_read() must not impact the status of high endurance"
        );

        print_test_footer("MemoryProtectionRegionConfiguration::enable_read()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_configuration_enable_write() {
        print_test_header("MemoryProtectionRegionConfiguration::enable_write()");

        let mut memory_protection_region_configuration = MemoryProtectionRegionConfiguration::new();
        memory_protection_region_configuration.enable_write();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region_configuration.is_read_enabled(),
            "MemoryProtectionRegionConfiguration::enable_write() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Enabled,
            memory_protection_region_configuration.is_write_enabled(),
            "MemoryProtectionRegionConfiguration::enable_write() must change the status of write access to enabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region_configuration.is_erase_enabled(),
            "MemoryProtectionRegionConfiguration::enable_write() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region_configuration.is_high_endurance_enabled(),
            "MemoryProtectionRegionConfiguration::enable_write() must not impact the status of high endurance"
        );

        print_test_footer("MemoryProtectionRegionConfiguration::enable_write()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_configuration_enable_erase() {
        print_test_header("MemoryProtectionRegionConfiguration::enable_erase()");

        let mut memory_protection_region_configuration = MemoryProtectionRegionConfiguration::new();
        memory_protection_region_configuration.enable_erase();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region_configuration.is_read_enabled(),
            "MemoryProtectionRegionConfiguration::enable_erase() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region_configuration.is_write_enabled(),
            "MemoryProtectionRegionConfiguration::enable_erase() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Enabled,
            memory_protection_region_configuration.is_erase_enabled(),
            "MemoryProtectionRegionConfiguration::enable_erase() must change the status of erase access to enabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region_configuration.is_high_endurance_enabled(),
            "MemoryProtectionRegionConfiguration::enable_erase() must not impact the status of high endurance"
        );

        print_test_footer("MemoryProtectionRegionConfiguration::enable_erase()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_configuration_enable_high_endurance() {
        print_test_header("MemoryProtectionRegionConfiguration::enable_high_endurance()");

        let mut memory_protection_region_configuration = MemoryProtectionRegionConfiguration::new();
        memory_protection_region_configuration.enable_high_endurance();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region_configuration.is_read_enabled(),
            "MemoryProtectionRegionConfiguration::enable_high_endurance() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region_configuration.is_write_enabled(),
            "MemoryProtectionRegionConfiguration::enable_high_endurance() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region_configuration.is_erase_enabled(),
            "MemoryProtectionRegionConfiguration::enable_high_endurance() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Enabled,
            memory_protection_region_configuration.is_high_endurance_enabled(),
            "MemoryProtectionRegionConfiguration::enable_high_endurance() must change the status of high endurance to enabled"
        );

        print_test_footer("MemoryProtectionRegionConfiguration::enable_high_endurance()");
    }

    #[cfg_attr(test, test)]
    fn test_default_memory_protection_region_constructor() {
        print_test_header("DefaultMemoryProtectionRegion::new()");

        let default_memory_protection_region = MemoryProtectionRegionConfiguration::new();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            default_memory_protection_region.is_read_enabled(),
            "Upon creation of the default memory protection region, read access must be disabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            default_memory_protection_region.is_write_enabled(),
            "Upon creation of the default memory protection region, write access must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            default_memory_protection_region.is_erase_enabled(),
            "Upon creation of the default memory protection region, erase access must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            default_memory_protection_region.is_high_endurance_enabled(),
            "Upon creation of the default memory protection region, high endurance must be disabled"
        );

        print_test_footer("DefaultMemoryProtectionRegion::new()");
    }

    #[cfg_attr(test, test)]
    fn test_default_memory_protection_region_enable_read() {
        print_test_header("DefaultMemoryProtectionRegion::enable_read()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new().enable_read();

        assert_eq!(
            ReadEnabledStatus::Enabled,
            default_memory_protection_region.is_read_enabled(),
            "DefaultMemoryProtectionRegion::enable_read() must change the status of read access to enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            default_memory_protection_region.is_write_enabled(),
            "DefaultMemoryProtectionRegion::enable_read() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            default_memory_protection_region.is_erase_enabled(),
            "DefaultMemoryProtectionRegion::enable_read() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            default_memory_protection_region.is_high_endurance_enabled(),
            "DefaultMemoryProtectionRegion::enable_read() must not impact the status of high endurance"
        );

        print_test_footer("DefaultMemoryProtectionRegion::enable_read()");
    }

    #[cfg_attr(test, test)]
    fn test_default_memory_protection_region_enable_write() {
        print_test_header("DefaultMemoryProtectionRegion::enable_write()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new().enable_write();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            default_memory_protection_region.is_read_enabled(),
            "DefaultMemoryProtectionRegion::enable_write() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Enabled,
            default_memory_protection_region.is_write_enabled(),
            "DefaultMemoryProtectionRegion::enable_write() must change the status of write access to enabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            default_memory_protection_region.is_erase_enabled(),
            "DefaultMemoryProtectionRegion::enable_write() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            default_memory_protection_region.is_high_endurance_enabled(),
            "DefaultMemoryProtectionRegion::enable_write() must not impact the status of high endurance"
        );

        print_test_footer("DefaultMemoryProtectionRegion::enable_write()");
    }

    #[cfg_attr(test, test)]
    fn test_default_memory_protection_region_enable_erase() {
        print_test_header("DefaultMemoryProtectionRegion::enable_erase()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new().enable_erase();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            default_memory_protection_region.is_read_enabled(),
            "DefaultMemoryProtectionRegion::enable_erase() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            default_memory_protection_region.is_write_enabled(),
            "DefaultMemoryProtectionRegion::enable_erase() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Enabled,
            default_memory_protection_region.is_erase_enabled(),
            "DefaultMemoryProtectionRegion::enable_erase() must change the status of erase access to enabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            default_memory_protection_region.is_high_endurance_enabled(),
            "DefaultMemoryProtectionRegion::enable_erase() must not impact the status of high endurance"
        );

        print_test_footer("DefaultMemoryProtectionRegion::enable_erase()");
    }

    #[cfg_attr(test, test)]
    fn test_default_memory_protection_region_enable_high_endurance() {
        print_test_header("DefaultMemoryProtectionRegion::enable_high_endurance()");

        let default_memory_protection_region =
            DefaultMemoryProtectionRegion::new().enable_high_endurance();

        assert_eq!(
            ReadEnabledStatus::Disabled,
            default_memory_protection_region.is_read_enabled(),
            "DefaultMemoryProtectionRegion::enable_high_endurance() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            default_memory_protection_region.is_write_enabled(),
            "DefaultMemoryProtectionRegion::enable_high_endurance() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            default_memory_protection_region.is_erase_enabled(),
            "DefaultMemoryProtectionRegion::enable_high_endurance() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Enabled,
            default_memory_protection_region.is_high_endurance_enabled(),
            "DefaultMemoryProtectionRegion::enable_high_endurance() must change the status of high endurance to enabled"
        );

        print_test_footer("DefaultMemoryProtectionRegion::enable_high_endurance()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_constructor() {
        print_test_header("DataMemoryProtectionRegion::new()");

        let memory_protection_region = DataMemoryProtectionRegion::new();

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "Upon creation of a memory protection region, it must be disabled"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "Upon creation of a memory protection region, its base must be 0"
        );

        let expected_memory_protection_region_size = create_data_memory_protection_region_size(1);

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "Upon creation of a memory protection region, its size must be 0"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Upon creation of a memory protection region, read access must be disabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Upon creation of a memory protection region, write access must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Upon creation of a memory protection region, erase access must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "Upon creation of a memory protection region, high endurance must be disabled"
        );

        print_test_footer("DataMemoryProtectionRegion::new()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_enable() {
        print_test_header("DataMemoryProtectionRegion::enable()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        memory_protection_region.enable();

        assert_eq!(
            MemoryProtectionRegionStatus::Enabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::enable() must change the status to enabled"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::enable() must not impact the region's base"
        );

        let expected_memory_protection_region_size = create_data_memory_protection_region_size(1);

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::enable(), must not impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of high endurance"
        );

        print_test_footer("DataMemoryProtectionRegion::enable()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_set_base() {
        print_test_header("DataMemoryProtectionRegion::set_base()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        let memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(10)));
        memory_protection_region.set_base(memory_protection_region_base);

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::set_base() must not change the status to enabled"
        );

        let expected_memory_protection_region_base = memory_protection_region_base;

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::set_base() must change the region's base"
        );

        let expected_memory_protection_region_size = DEFAULT_DATA_MEMORY_PROTECTION_REGION_SIZE;

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::set_size(), must not impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::set_base() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::set_base() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::set_base() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::set_base() must not impact the status of high endurance"
        );

        print_test_footer("DataMemoryProtectionRegion::set_base()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_set_size() {
        print_test_header("DataMemoryProtectionRegion::set_size()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        let memory_protection_region_size = create_data_memory_protection_region_size(3);
        memory_protection_region.set_size(memory_protection_region_size);

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::set_size() must not change the status to enabled"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::set_size() must not impact the region's base"
        );

        let expected_memory_protection_region_size = memory_protection_region_size;

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::set_size() must impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::set_size() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::set_size() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::set_size() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::set_size() must not impact the status of high endurance"
        );

        print_test_footer("DataMemoryProtectionRegion::set_size()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_enable_read() {
        print_test_header("DataMemoryProtectionRegion::enable_read()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        memory_protection_region.enable_read();

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of the memory protection region"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::enable_read() must not impact the region's base"
        );

        let expected_memory_protection_region_size = create_data_memory_protection_region_size(1);

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::enable_read(), must not impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Enabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::enable_read() must change the status of read access to enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::enable_read() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::enable_read() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::enable_read() must not impact the status of high endurance"
        );

        print_test_footer("DataMemoryProtectionRegion::enable_read()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_enable_write() {
        print_test_header("DataMemoryProtectionRegion::enable_write()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        memory_protection_region.enable_write();

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of the memory protection region"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::enable_write() must not impact the region's base"
        );

        let expected_memory_protection_region_size = DEFAULT_DATA_MEMORY_PROTECTION_REGION_SIZE;

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::enable_write(), must not impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::enable_write() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Enabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::enable_write() must change the status of write access to enabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::enable_write() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::enable_write() must not impact the status of high endurance"
        );

        print_test_footer("DataMemoryProtectionRegion::enable_write()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_enable_erase() {
        print_test_header("DataMemoryProtectionRegion::enable_erase()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        memory_protection_region.enable_erase();

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of the memory protection region"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::enable_erase() must not impact the region's base"
        );

        let expected_memory_protection_region_size = DEFAULT_DATA_MEMORY_PROTECTION_REGION_SIZE;

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::enable_erase(), must not impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::enable_erase() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::enable_erase() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Enabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::enable_erase() must change the status of erase access to enabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::enable_erase() must not impact the status of high endurance"
        );

        print_test_footer("DataMemoryProtectionRegion::enable_erase()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_enable_high_endurance() {
        print_test_header("DataMemoryProtectionRegion::enable_high_endurance()");

        let mut memory_protection_region = DataMemoryProtectionRegion::new();
        memory_protection_region.enable_high_endurance();

        assert_eq!(
            MemoryProtectionRegionStatus::Disabled,
            memory_protection_region.is_enabled(),
            "DataMemoryProtectionRegion::enable() must not impact the status of the memory protection region"
        );

        let expected_memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(0)));

        assert_eq!(
            expected_memory_protection_region_base,
            memory_protection_region.get_base(),
            "DataMemoryProtectionRegion::enable_high_endurance() must not impact the region's base"
        );

        let expected_memory_protection_region_size = DEFAULT_DATA_MEMORY_PROTECTION_REGION_SIZE;

        assert_eq!(
            expected_memory_protection_region_size,
            memory_protection_region.get_size(),
            "DataMemoryProtectionRegion::enable_high_endurance(), must not impact the region's size"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "DataMemoryProtectionRegion::enable_high_endurance() must not impact the status of read access"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "DataMemoryProtectionRegion::enable_high_endurance() must not impact the status of write access"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "DataMemoryProtectionRegion::enable_high_endurance() must not impact the status of erase access"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Enabled,
            memory_protection_region.is_high_endurance_enabled(),
            "DataMemoryProtectionRegion::enable_high_endurance() must change the status of high endurance to enabled"
        );

        print_test_footer("DataMemoryProtectionRegion::enable_high_endurance()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_index_iterator() {
        print_test_header("DataMemoryProtectionRegionIndexIterator");

        let mut memory_protection_region_index_iterator =
            DataMemoryProtectionRegionIndexIterator::new();

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index0),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index1),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index2),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index3),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index4),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index5),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index6),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(
            Some(DataMemoryProtectionRegionIndex::Index7),
            memory_protection_region_index_iterator.next()
        );

        assert_eq!(None, memory_protection_region_index_iterator.next());

        print_test_footer("DataMemoryProtectionRegionIndexIterator");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_configuration_constructor() {
        print_test_header("MemoryProtectionConfiguration::new()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new()
            .enable_read()
            .enable_high_endurance();

        let memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region);

        let default_memory_protection_region =
            memory_protection_configuration.get_default_memory_protection_region();

        assert_eq!(
            ReadEnabledStatus::Enabled,
            default_memory_protection_region.is_read_enabled(),
            "Read must be enabled for default memory protection region"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            default_memory_protection_region.is_write_enabled(),
            "Write must be disabled for default memory protection region"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            default_memory_protection_region.is_erase_enabled(),
            "Erase must be disabled for default memory protection region"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Enabled,
            default_memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be enabled for default memory protection region"
        );

        for (_data_memory_protection_region_index, data_memory_protection_region) in
            memory_protection_configuration
                .get_data_memory_protection_regions()
                .as_iterator()
        {
            assert_eq!(
                ReadEnabledStatus::Disabled,
                data_memory_protection_region.is_read_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all data regions must have read access disabled"
            );

            assert_eq!(
                WriteEnabledStatus::Disabled,
                data_memory_protection_region.is_write_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all data regions must have write access disabled"
            );

            assert_eq!(
                EraseEnabledStatus::Disabled,
                data_memory_protection_region.is_erase_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all data regions must have erase access disabled"
            );

            assert_eq!(
                HighEnduranceEnabledStatus::Disabled,
                data_memory_protection_region.is_high_endurance_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all data regions must have high endurance disabled"
            );
        }

        for (_info0_memory_protection_region_index, info0_memory_protection_region) in
            memory_protection_configuration
                .get_info0_memory_protection_regions()
                .as_iterator()
        {
            assert_eq!(
                ReadEnabledStatus::Disabled,
                info0_memory_protection_region.is_read_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info0 regions must have read access disabled"
            );

            assert_eq!(
                WriteEnabledStatus::Disabled,
                info0_memory_protection_region.is_write_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info0 regions must have write access disabled"
            );

            assert_eq!(
                EraseEnabledStatus::Disabled,
                info0_memory_protection_region.is_erase_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info0 regions must have erase access disabled"
            );

            assert_eq!(
                HighEnduranceEnabledStatus::Disabled,
                info0_memory_protection_region.is_high_endurance_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info0 regions must have high endurance disabled"
            );
        }

        for (_info1_memory_protection_region_index, info1_memory_protection_region) in
            memory_protection_configuration
                .get_info1_memory_protection_regions()
                .as_iterator()
        {
            assert_eq!(
                ReadEnabledStatus::Disabled,
                info1_memory_protection_region.is_read_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info1 regions must have read access disabled"
            );

            assert_eq!(
                WriteEnabledStatus::Disabled,
                info1_memory_protection_region.is_write_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info1 regions must have write access disabled"
            );

            assert_eq!(
                EraseEnabledStatus::Disabled,
                info1_memory_protection_region.is_erase_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info1 regions must have erase access disabled"
            );

            assert_eq!(
                HighEnduranceEnabledStatus::Disabled,
                info1_memory_protection_region.is_high_endurance_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info1 regions must have high endurance disabled"
            );
        }

        for (_info2_memory_protection_region_index, info2_memory_protection_region) in
            memory_protection_configuration
                .get_info2_memory_protection_regions()
                .as_iterator()
        {
            assert_eq!(
                ReadEnabledStatus::Disabled,
                info2_memory_protection_region.is_read_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info2 regions must have read access disabled"
            );

            assert_eq!(
                WriteEnabledStatus::Disabled,
                info2_memory_protection_region.is_write_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info2 regions must have write access disabled"
            );

            assert_eq!(
                EraseEnabledStatus::Disabled,
                info2_memory_protection_region.is_erase_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info2 regions must have erase access disabled"
            );

            assert_eq!(
                HighEnduranceEnabledStatus::Disabled,
                info2_memory_protection_region.is_high_endurance_enabled(),
                "Upon creation of MemoryProtectionConfiguration, all info2 regions must have high endurance disabled"
            );
        }

        print_test_footer("MemoryProtectionConfiguration::new()");
    }

    // Note: for this test, status, base and index are not tested. They are tested through the
    // DataMemoryProtectionRegionBuilder.
    #[cfg_attr(test, test)]
    fn test_memory_protection_configuration_enable_read() {
        print_test_header("MemoryProtectionConfiguration::enable_read()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region);

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index0;

        memory_protection_configuration.enable_read_data(memory_protection_region_index);

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            ReadEnabledStatus::Enabled,
            memory_protection_region.is_read_enabled(),
            "Read must be enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Write must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Erase must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("MemoryProtectionConfiguration::enable_read()");
    }

    // Note: for this test, status, base and index are not tested. They are tested through the
    // DataMemoryProtectionRegionBuilder.
    #[cfg_attr(test, test)]
    fn test_memory_protection_configuration_enable_write() {
        print_test_header("MemoryProtectionConfiguration::enable_write()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region);

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index1;

        memory_protection_configuration.enable_write_data(memory_protection_region_index);

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Read must be disabled"
        );

        assert_eq!(
            WriteEnabledStatus::Enabled,
            memory_protection_region.is_write_enabled(),
            "Write must be enabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Erase must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("MemoryProtectionConfiguration::enable_write()");
    }

    // Note: for this test, status, base and index are not tested. They are tested through the
    // DataMemoryProtectionRegionBuilder.
    #[cfg_attr(test, test)]
    fn test_memory_protection_configuration_enable_erase() {
        print_test_header("MemoryProtectionConfiguration::enable_erase()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region);

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index2;

        memory_protection_configuration.enable_erase_data(memory_protection_region_index);

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Read must be disabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Write must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Enabled,
            memory_protection_region.is_erase_enabled(),
            "Erase must be enabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("MemoryProtectionConfiguration::enable_erase()");
    }

    // Note: for this test, status, base and index are not tested. They are tested through the
    // DataMemoryProtectionRegionBuilder.
    #[cfg_attr(test, test)]
    fn test_memory_protection_configuration_enable_high_endurance() {
        print_test_header("MemoryProtectionConfiguration::enable_high_endurance()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region);

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index3;

        memory_protection_configuration.enable_high_endurance_data(memory_protection_region_index);

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Read must be disabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Write must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Erase must be enabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Enabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("MemoryProtectionConfiguration::enable_high_endurance()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_builder_enable_read() {
        print_test_header("DataMemoryProtectionRegionBuilder::enable_read()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index0;
        let memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank1(DataPageIndex::new(0)));
        // PANIC: 1 != 0 && 1 <= 512
        let memory_protection_region_size = create_data_memory_protection_region_size(1);

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region)
                .enable_and_configure_data_region(
                    memory_protection_region_index,
                    memory_protection_region_base,
                    memory_protection_region_size,
                )
                .enable_read()
                .finalize_region();

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            MemoryProtectionRegionStatus::Enabled,
            memory_protection_region.is_enabled(),
            "Memory protection region must be enabled after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_base,
            memory_protection_region.get_base(),
            "Memory protection region base must be set after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_size,
            memory_protection_region.get_size(),
            "Memory protection region size must be set after builder finalizes"
        );

        assert_eq!(
            ReadEnabledStatus::Enabled,
            memory_protection_region.is_read_enabled(),
            "Read access must be enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Write access must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Erase access must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("DataMemoryProtectionRegionBuilder::enable_read()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_builder_enable_write() {
        print_test_header("DataMemoryProtectionRegionBuilder::enable_write()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index1;
        let memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(1)));
        // PANIC: 2 != 0 && 2 <= 512
        let memory_protection_region_size = create_data_memory_protection_region_size(2);

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region)
                .enable_and_configure_data_region(
                    memory_protection_region_index,
                    memory_protection_region_base,
                    memory_protection_region_size,
                )
                .enable_write()
                .finalize_region();

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            MemoryProtectionRegionStatus::Enabled,
            memory_protection_region.is_enabled(),
            "Memory protection region must be enabled after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_base,
            memory_protection_region.get_base(),
            "Memory protection region base must be set after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_size,
            memory_protection_region.get_size(),
            "Memory protection region size must be set after builder finalizes"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Read access must be enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Enabled,
            memory_protection_region.is_write_enabled(),
            "Write access must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Erase access must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("DataMemoryProtectionRegionBuilder::enable_write()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_builder_enable_erase() {
        print_test_header("DataMemoryProtectionRegionBuilder::enable_erase()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index2;
        let memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank1(DataPageIndex::new(2)));
        // PANIC: 3 != 0 && 3 <= 512
        let memory_protection_region_size = create_data_memory_protection_region_size(3);

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region)
                .enable_and_configure_data_region(
                    memory_protection_region_index,
                    memory_protection_region_base,
                    memory_protection_region_size,
                )
                .enable_erase()
                .finalize_region();

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            MemoryProtectionRegionStatus::Enabled,
            memory_protection_region.is_enabled(),
            "Memory protection region must be enabled after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_base,
            memory_protection_region.get_base(),
            "Memory protection region base must be set after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_size,
            memory_protection_region.get_size(),
            "Memory protection region size must be set after builder finalizes"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Read access must be enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Write access must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Enabled,
            memory_protection_region.is_erase_enabled(),
            "Erase access must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Disabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("DataMemoryProtectionRegionBuilder::enable_erase()");
    }

    #[cfg_attr(test, test)]
    fn test_memory_protection_region_builder_enable_high_endurance() {
        print_test_header("DataMemoryProtectionRegionBuilder::enable_high_endurance()");

        let default_memory_protection_region = DefaultMemoryProtectionRegion::new();

        let memory_protection_region_index = DataMemoryProtectionRegionIndex::Index3;
        let memory_protection_region_base =
            DataMemoryProtectionRegionBase::new(DataPagePosition::Bank0(DataPageIndex::new(3)));
        // PANIC: 4 != 0 && 4 <= 512
        let memory_protection_region_size = create_data_memory_protection_region_size(4);

        let mut memory_protection_configuration =
            MemoryProtectionConfiguration::new(default_memory_protection_region)
                .enable_and_configure_data_region(
                    memory_protection_region_index,
                    memory_protection_region_base,
                    memory_protection_region_size,
                )
                .enable_high_endurance()
                .finalize_region();

        let memory_protection_region = memory_protection_configuration
            .get_data_memory_protection_region_mut(memory_protection_region_index);

        assert_eq!(
            MemoryProtectionRegionStatus::Enabled,
            memory_protection_region.is_enabled(),
            "Memory protection region must be enabled after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_base,
            memory_protection_region.get_base(),
            "Memory protection region base must be set after builder finalizes"
        );

        assert_eq!(
            memory_protection_region_size,
            memory_protection_region.get_size(),
            "Memory protection region size must be set after builder finalizes"
        );

        assert_eq!(
            ReadEnabledStatus::Disabled,
            memory_protection_region.is_read_enabled(),
            "Read access must be enabled"
        );

        assert_eq!(
            WriteEnabledStatus::Disabled,
            memory_protection_region.is_write_enabled(),
            "Write access must be disabled"
        );

        assert_eq!(
            EraseEnabledStatus::Disabled,
            memory_protection_region.is_erase_enabled(),
            "Erase access must be disabled"
        );

        assert_eq!(
            HighEnduranceEnabledStatus::Enabled,
            memory_protection_region.is_high_endurance_enabled(),
            "High endurance must be disabled"
        );

        print_test_footer("DataMemoryProtectionRegionBuilder::enable_high_endurance()");
    }

    pub(in super::super) fn run_all() {
        /* MemoryProtectionRegionConfiguration */
        test_memory_protection_region_configuration_constructor();
        test_memory_protection_region_configuration_enable_read();
        test_memory_protection_region_configuration_enable_write();
        test_memory_protection_region_configuration_enable_erase();
        test_memory_protection_region_configuration_enable_high_endurance();

        /* DefaultMemoryProtectionRegion */
        test_default_memory_protection_region_constructor();
        test_default_memory_protection_region_enable_read();
        test_default_memory_protection_region_enable_write();
        test_default_memory_protection_region_enable_erase();
        test_default_memory_protection_region_enable_high_endurance();

        /* DataMemoryProtectionRegion */
        test_memory_protection_region_constructor();
        test_memory_protection_region_enable();
        test_memory_protection_region_set_base();
        test_memory_protection_region_set_size();
        test_memory_protection_region_enable_read();
        test_memory_protection_region_enable_write();
        test_memory_protection_region_enable_erase();
        test_memory_protection_region_enable_high_endurance();

        /* MemoryProtectionRegionIterator */
        test_memory_protection_region_index_iterator();

        /* MemoryProtectionConfiguration */
        test_memory_protection_configuration_constructor();
        test_memory_protection_configuration_enable_read();
        test_memory_protection_configuration_enable_write();
        test_memory_protection_configuration_enable_erase();
        test_memory_protection_configuration_enable_high_endurance();

        /* DataMemoryProtectionRegionBuilder */
        test_memory_protection_region_builder_enable_read();
        test_memory_protection_region_builder_enable_write();
        test_memory_protection_region_builder_enable_erase();
        test_memory_protection_region_builder_enable_high_endurance();
    }
}
