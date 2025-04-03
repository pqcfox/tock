// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::bank::{Bank, BANK_SIZE, DATA_PAGES_PER_BANK, NUMBER_OF_BANKS};
use super::chunk::{
    ImmutableChunkIterator, MutableChunkIterator, PageChunkIterator, PageChunkIteratorEmpty,
    WORDS_PER_CHUNK,
};
use super::fifo_level::FifoLevel;
use super::flash_address::FlashAddress;
use super::info_partition_type::InfoPartitionType;
use super::memory_protection::{
    DataMemoryProtectionRegion, DataMemoryProtectionRegionBase, DataMemoryProtectionRegionIndex,
    DataMemoryProtectionRegionList, DefaultMemoryProtectionRegion, EccEnabledStatus,
    EraseEnabledStatus, HighEnduranceEnabledStatus, Info0MemoryProtectionRegionIndex,
    Info0MemoryProtectionRegionList, Info1MemoryProtectionRegionIndex,
    Info1MemoryProtectionRegionList, Info2MemoryProtectionRegionIndex,
    Info2MemoryProtectionRegionList, InfoMemoryProtectionRegion, MemoryProtectionConfiguration,
    MemoryProtectionRegionStatus, ReadEnabledStatus, ScrambleEnabledStatus, WriteEnabledStatus,
};
use super::page::{DataFlashCtrlPage, FlashCtrlPage, InfoFlashCtrlPage, RawFlashCtrlPage};
use super::page_index::{DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex};
use super::page_position::{
    DataPagePosition, Info0PagePosition, Info1PagePosition, Info2PagePosition, InfoPagePosition,
};

use crate::registers::flash_ctrl_regs::{
    FlashCtrlRegisters, ADDR, BANK0_INFO0_PAGE_CFG, BANK0_INFO0_REGWEN, BANK0_INFO1_PAGE_CFG,
    BANK0_INFO1_REGWEN, BANK0_INFO2_PAGE_CFG, BANK0_INFO2_REGWEN, BANK1_INFO0_PAGE_CFG,
    BANK1_INFO0_REGWEN, BANK1_INFO1_PAGE_CFG, BANK1_INFO1_REGWEN, BANK1_INFO2_PAGE_CFG,
    BANK1_INFO2_REGWEN, CONTROL, DEFAULT_REGION, ERR_CODE, FIFO_LVL, INTR, MP_REGION,
    MP_REGION_CFG, OP_STATUS, REGION_CFG_REGWEN, STATUS,
};
use crate::registers::top_earlgrey::{FLASH_CTRL_CORE_BASE_ADDR, FLASH_CTRL_MEM_BASE_ADDR};
use crate::utils;

use kernel::hil::flash as flash_hil;
use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::registers::FieldValue;
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::StaticRef;
use kernel::ErrorCode;

use core::cell::Cell;
use core::num::NonZeroUsize;

/// Magic value to write to a 4-bit register that represents "CLEAR" or "DISABLED".
const DISABLE_MAGIC_VALUE: u32 = 0x09;
/// Magic value to write to a 4-bit register that represents "SET" or "ENABLED".
const ENABLE_MAGIC_VALUE: u32 = 0x06;

/// The base of flash registers
pub(super) const FLASH_CTRL_BASE: StaticRef<FlashCtrlRegisters> =
    unsafe { StaticRef::new(FLASH_CTRL_CORE_BASE_ADDR as *const FlashCtrlRegisters) };
pub(super) const FLASH_HOST_STARTING_ADDRESS_OFFSET: NonZeroUsize =
    // PANIC: 0x20000000 != 0
    utils::create_non_zero_usize(FLASH_CTRL_MEM_BASE_ADDR);
/// The size of the flash in bytes
pub(super) const FLASH_SIZE: NonZeroUsize = match BANK_SIZE.checked_mul(NUMBER_OF_BANKS) {
    Some(flash_size) => flash_size,
    // BANK_SIZE * NUMBER_OF_BANKS = 512KiB * 2 = 1MiB ==> multiplication does not overflow
    None => unreachable!(),
};

/// Flash busy status
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum BusyStatus {
    /// The peripheral is busy
    Busy,
    /// The peripheral is not busy
    NotBusy,
}

/// Read/Write flash operation
#[derive(PartialEq, Eq, Clone, Copy)]
enum ReadWriteFlashOperationType {
    Read,
    Write,
}

impl From<ReadWriteFlashOperationType> for FieldValue<u32, CONTROL::Register> {
    fn from(value: ReadWriteFlashOperationType) -> Self {
        match value {
            ReadWriteFlashOperationType::Read => CONTROL::OP::READ,
            ReadWriteFlashOperationType::Write => CONTROL::OP::PROG,
        }
    }
}

/// Possible flash operations
#[derive(PartialEq, Eq, Clone, Copy)]
enum FlashOperationType {
    /// Read a page
    Read,
    /// Write a page
    Write,
    /// Erase a page
    Erase,
}

impl From<FlashOperationType> for FieldValue<u32, CONTROL::Register> {
    fn from(value: FlashOperationType) -> Self {
        match value {
            FlashOperationType::Read => CONTROL::OP::READ,
            FlashOperationType::Write => CONTROL::OP::PROG,
            FlashOperationType::Erase => CONTROL::OP::ERASE,
        }
    }
}

/// Partition types
enum PartitionType {
    /// Data partition
    Data,
    /// Info partition
    Info,
}

/// Erase types
enum EraseType {
    /// Erase of a page
    PageErase,
    /// Erase of a bank
    // This is not currently used.
    #[allow(unused)]
    BankErase,
}

/// Flash error codes.
#[derive(Clone, Copy, Debug)]
enum FlashErrorCode {
    /// Undefined operation supplied
    Operation,
    /// Flash access has encountered an access permission error
    MemoryProtection,
    /// Flash read error. Possible reasons:
    ///
    /// + Reliability ECC
    /// + Storage integrity errors.
    Read,
    /// Flash program (write) error. This could be a program integrity error.
    Program,
    /// Flash program window resolution error.
    ProgramResolution,
    /// Flash program selected unavailable type.
    ProgramType,
    /// A shadow register encountered an update error.
    Update,
    /// A recoverable error has been encourented in the flash macro.
    Macro,
}

pub struct FlashCtrl<'a> {
    registers: StaticRef<FlashCtrlRegisters>,
    data_client: OptionalCell<&'a dyn flash_hil::Client<FlashCtrl<'a>>>,
    info_client: OptionalCell<&'a dyn flash_hil::InfoClient<FlashCtrl<'a>>>,
    page_chunk_iterator: OptionalCell<PageChunkIterator<'static>>,
    is_busy: Cell<BusyStatus>,
}

#[derive(Clone, Copy)]
pub enum FlashCtrlInterrupt {
    /// Program FIFO empty
    ProgEmpty,
    /// Program FIFO drained to level
    ProgLvl,
    /// Read FIFO full
    RdFull,
    /// Read FIFO filled to level
    RdLvl,
    /// Operation complete
    OpDone,
    /// Correctable error encountered
    CorrErr,
}

macro_rules! convert_info_memory_protection_region_to_register_value {
    {$function:ident, $cfg:ident} => {

        /// Convert a [InfoMemoryProtectionRegion] to a value suitable to be written to the appropriate
        /// register
        ///
        /// # Parameters
        ///
        /// + `info_memory_protection_region`: [InfoMemoryProtectionRegion] to be converted
        ///
        /// # Return value
        ///
        /// The register value used to configure the appropriate register.
        fn $function(region: &InfoMemoryProtectionRegion) -> FieldValue<u32, $cfg::Register> {
            let high_endurance_enabled = match region.is_high_endurance_enabled()
            {
                HighEnduranceEnabledStatus::Disabled => $cfg::HE_EN_0.val(DISABLE_MAGIC_VALUE),
                HighEnduranceEnabledStatus::Enabled => $cfg::HE_EN_0.val(ENABLE_MAGIC_VALUE),
            };

            let scramble_enabled = match region.is_scramble_enabled() {
                ScrambleEnabledStatus::Disabled => $cfg::SCRAMBLE_EN_0.val(DISABLE_MAGIC_VALUE),
                ScrambleEnabledStatus::Enabled => $cfg::SCRAMBLE_EN_0.val(ENABLE_MAGIC_VALUE),
            };

            let ecc_enabled = match region.is_ecc_enabled() {
                EccEnabledStatus::Disabled => $cfg::ECC_EN_0.val(DISABLE_MAGIC_VALUE),
                EccEnabledStatus::Enabled => $cfg::ECC_EN_0.val(ENABLE_MAGIC_VALUE),
            };

            let erase_enabled = match region.is_erase_enabled() {
                EraseEnabledStatus::Disabled => $cfg::ERASE_EN_0.val(DISABLE_MAGIC_VALUE),
                EraseEnabledStatus::Enabled => $cfg::ERASE_EN_0.val(ENABLE_MAGIC_VALUE),
            };

            let write_enabled = match region.is_write_enabled() {
                WriteEnabledStatus::Disabled => $cfg::PROG_EN_0.val(DISABLE_MAGIC_VALUE),
                WriteEnabledStatus::Enabled => $cfg::PROG_EN_0.val(ENABLE_MAGIC_VALUE),
            };

            let read_enabled = match region.is_read_enabled() {
                ReadEnabledStatus::Disabled => $cfg::RD_EN_0.val(DISABLE_MAGIC_VALUE),
                ReadEnabledStatus::Enabled => $cfg::RD_EN_0.val(ENABLE_MAGIC_VALUE),
            };

            let is_region_enabled = match region.is_enabled() {
                MemoryProtectionRegionStatus::Disabled => $cfg::EN_0.val(DISABLE_MAGIC_VALUE),
                MemoryProtectionRegionStatus::Enabled => $cfg::EN_0.val(ENABLE_MAGIC_VALUE),
            };

            high_endurance_enabled
                + scramble_enabled
                + ecc_enabled
                + erase_enabled
                + write_enabled
                + read_enabled
                + is_region_enabled
        }
    }
}
convert_info_memory_protection_region_to_register_value! {convert_bank0_info0_memory_protection_region_to_register_value, BANK0_INFO0_PAGE_CFG}
convert_info_memory_protection_region_to_register_value! {convert_bank0_info1_memory_protection_region_to_register_value, BANK0_INFO1_PAGE_CFG}
convert_info_memory_protection_region_to_register_value! {convert_bank0_info2_memory_protection_region_to_register_value, BANK0_INFO2_PAGE_CFG}
convert_info_memory_protection_region_to_register_value! {convert_bank1_info0_memory_protection_region_to_register_value, BANK1_INFO0_PAGE_CFG}
convert_info_memory_protection_region_to_register_value! {convert_bank1_info1_memory_protection_region_to_register_value, BANK1_INFO1_PAGE_CFG}
convert_info_memory_protection_region_to_register_value! {convert_bank1_info2_memory_protection_region_to_register_value, BANK1_INFO2_PAGE_CFG}

impl FlashCtrl<'_> {
    /// [FlashCtrl] constructor
    ///
    /// # Parameters
    ///
    /// + `memory_protection_configuration`: memory protection configuration used to configure the
    /// access to the flash
    ///
    /// # Return value
    ///
    /// A new [FlashCtrl] instance.
    pub fn new(memory_protection_configuration: MemoryProtectionConfiguration) -> Self {
        let flash_ctrl = Self {
            registers: FLASH_CTRL_BASE,
            data_client: OptionalCell::empty(),
            info_client: OptionalCell::empty(),
            page_chunk_iterator: OptionalCell::empty(),
            is_busy: Cell::new(BusyStatus::NotBusy),
        };

        // Lock down `kCertificateInfoPages` as it appears ROM_EXT doesn't do this!
        // kFlashCtrlInfoPageAttestationKeySeeds
        flash_ctrl.registers.bank0_info0_regwen[4].set(0);
        // kFlashCtrlInfoPageTpmCerts
        flash_ctrl.registers.bank1_info0_regwen[4].set(0);
        #[cfg(not(feature = "unlock_dice_info_pages"))]
        {
            // kFlashCtrlInfoPageUdsCertificate
            flash_ctrl.registers.bank1_info0_regwen[6].set(0);
            // kFlashCtrlInfoPageCdi0Certificate
            flash_ctrl.registers.bank1_info0_regwen[8].set(0);
            // kFlashCtrlInfoPageCdi1Certificate
            flash_ctrl.registers.bank1_info0_regwen[9].set(0);
        }

        flash_ctrl.init(memory_protection_configuration);
        flash_ctrl
    }

    /// Init FIFO levels.
    fn init_fifo_levels(&self) {
        // The number of flash words to be read - 1
        const FIFO_READ_LEVEL: FifoLevel = FifoLevel::Level15;
        const FIFO_WRITE_LEVEL: FifoLevel = FifoLevel::Level0;

        self.registers.fifo_lvl.modify(
            FIFO_LVL::RD.val(FIFO_READ_LEVEL.inner() as u32)
                + FIFO_LVL::PROG.val(FIFO_WRITE_LEVEL.inner() as u32),
        );
    }

    /// Initialize the flash peripheral
    ///
    /// This method sets the appropriate FIFO levels, configures and locks memory protection and
    /// enable all required interrupts.
    ///
    /// # Parameters
    ///
    /// + `memory_protection_configuration`: the flash memory protection configuration to be used
    fn init(&self, memory_protection_configuration: MemoryProtectionConfiguration) {
        if cfg!(feature = "sival") {
            // When using ROM_EXT, the operation done bit is set when Tock boots. Clear it.
            self.clear_operation_done_interrupt();
            // Operation done status is also set. Clear it.
            self.clear_operation_done_status();
            // It looks like a flash operation error occurs durring ROM_EXT. Clear all error codes.
            self.clear_all_error_codes();
        }
        self.init_fifo_levels();
        self.configure_memory_protection(memory_protection_configuration);
        self.enable_interrupts();
    }

    /// Configure default region permissions.
    ///
    /// The default region permissions apply over a flash memory area if no memory protection
    /// region is defined.
    ///
    /// # Parameters
    ///
    /// + `default_memory_protection_region`: [DefaultMemoryProtectionRegion]
    fn configure_default_region_permissions(
        &self,
        default_memory_protection_region: &DefaultMemoryProtectionRegion,
    ) {
        let high_endurance_enabled = match default_memory_protection_region
            .is_high_endurance_enabled()
        {
            HighEnduranceEnabledStatus::Disabled => DEFAULT_REGION::HE_EN.val(DISABLE_MAGIC_VALUE),
            HighEnduranceEnabledStatus::Enabled => DEFAULT_REGION::HE_EN.val(ENABLE_MAGIC_VALUE),
        };

        let scramble_enabled = match default_memory_protection_region.is_scramble_enabled() {
            ScrambleEnabledStatus::Disabled => DEFAULT_REGION::SCRAMBLE_EN.val(DISABLE_MAGIC_VALUE),
            ScrambleEnabledStatus::Enabled => DEFAULT_REGION::SCRAMBLE_EN.val(ENABLE_MAGIC_VALUE),
        };

        let ecc_enabled = match default_memory_protection_region.is_ecc_enabled() {
            EccEnabledStatus::Disabled => DEFAULT_REGION::ECC_EN.val(DISABLE_MAGIC_VALUE),
            EccEnabledStatus::Enabled => DEFAULT_REGION::ECC_EN.val(ENABLE_MAGIC_VALUE),
        };

        let erase_enabled = match default_memory_protection_region.is_erase_enabled() {
            EraseEnabledStatus::Disabled => DEFAULT_REGION::ERASE_EN.val(DISABLE_MAGIC_VALUE),
            EraseEnabledStatus::Enabled => DEFAULT_REGION::ERASE_EN.val(ENABLE_MAGIC_VALUE),
        };

        let write_enabled = match default_memory_protection_region.is_write_enabled() {
            WriteEnabledStatus::Disabled => DEFAULT_REGION::PROG_EN.val(DISABLE_MAGIC_VALUE),
            WriteEnabledStatus::Enabled => DEFAULT_REGION::PROG_EN.val(ENABLE_MAGIC_VALUE),
        };

        let read_enabled = match default_memory_protection_region.is_read_enabled() {
            ReadEnabledStatus::Disabled => DEFAULT_REGION::RD_EN.val(DISABLE_MAGIC_VALUE),
            ReadEnabledStatus::Enabled => DEFAULT_REGION::RD_EN.val(ENABLE_MAGIC_VALUE),
        };

        self.registers.default_region.modify(
            high_endurance_enabled
                + scramble_enabled
                + ecc_enabled
                + erase_enabled
                + write_enabled
                + read_enabled,
        );
    }

    /// Convert a [DataMemoryProtectionRegionBase] to register value, so it can be written to a
    /// register
    fn convert_memory_protection_region_base_to_register_value(
        memory_protection_region_base: DataMemoryProtectionRegionBase,
    ) -> u32 {
        // u32 == usize on Earlgrey
        (match memory_protection_region_base.inner() {
            DataPagePosition::Bank0(page_index) => page_index.to_usize(),
            DataPagePosition::Bank1(page_index) => {
                DATA_PAGES_PER_BANK.get() + page_index.to_usize()
            }
        }) as u32
    }

    /// Configure the area covered by a data memory protection region
    ///
    /// # Parameters:
    ///
    /// + `memory_protection_region_register`: the register to configure the area covered by
    /// `memory_protection_region`
    /// + `memory_protection_region`: [DataMemoryProtectionRegion] that needs to be configured
    fn configure_area_data_memory_protection_region(
        &self,
        memory_protection_region_register: &ReadWrite<u32, MP_REGION::Register>,
        memory_protection_region: &DataMemoryProtectionRegion,
    ) {
        let memory_protection_region_base = MP_REGION::BASE_0.val(
            Self::convert_memory_protection_region_base_to_register_value(
                memory_protection_region.get_base(),
            ),
        );
        // u32 == usize on Earlgrey
        let memory_protection_region_size =
            MP_REGION::SIZE_0.val(memory_protection_region.get_size().inner() as u32);

        memory_protection_region_register
            .modify(memory_protection_region_base + memory_protection_region_size);
    }

    /// Configure the access permissions associated with the given data memory protection region
    ///
    /// # Parameters
    ///
    /// + `memory_protection_region_register`: the register that needs to be configured by
    /// `memory_protection_region`
    /// + `memory_protection_region`: [DataMemoryProtectionRegion] used to configure
    /// `memory_protection_region`
    fn configure_access_data_memory_protection_region(
        &self,
        memory_protection_region_register: &ReadWrite<u32, MP_REGION_CFG::Register>,
        memory_protection_region: &DataMemoryProtectionRegion,
    ) {
        let high_endurance_enabled = match memory_protection_region.is_high_endurance_enabled() {
            HighEnduranceEnabledStatus::Disabled => MP_REGION_CFG::HE_EN_0.val(DISABLE_MAGIC_VALUE),
            HighEnduranceEnabledStatus::Enabled => MP_REGION_CFG::HE_EN_0.val(ENABLE_MAGIC_VALUE),
        };

        let scramble_enabled = match memory_protection_region.is_scramble_enabled() {
            ScrambleEnabledStatus::Disabled => {
                MP_REGION_CFG::SCRAMBLE_EN_0.val(DISABLE_MAGIC_VALUE)
            }
            ScrambleEnabledStatus::Enabled => MP_REGION_CFG::SCRAMBLE_EN_0.val(ENABLE_MAGIC_VALUE),
        };

        let ecc_enabled = match memory_protection_region.is_ecc_enabled() {
            EccEnabledStatus::Disabled => MP_REGION_CFG::ECC_EN_0.val(DISABLE_MAGIC_VALUE),
            EccEnabledStatus::Enabled => MP_REGION_CFG::ECC_EN_0.val(ENABLE_MAGIC_VALUE),
        };

        let erase_enabled = match memory_protection_region.is_erase_enabled() {
            EraseEnabledStatus::Disabled => MP_REGION_CFG::ERASE_EN_0.val(DISABLE_MAGIC_VALUE),
            EraseEnabledStatus::Enabled => MP_REGION_CFG::ERASE_EN_0.val(ENABLE_MAGIC_VALUE),
        };

        let write_enabled = match memory_protection_region.is_write_enabled() {
            WriteEnabledStatus::Disabled => MP_REGION_CFG::PROG_EN_0.val(DISABLE_MAGIC_VALUE),
            WriteEnabledStatus::Enabled => MP_REGION_CFG::PROG_EN_0.val(ENABLE_MAGIC_VALUE),
        };

        let read_enabled = match memory_protection_region.is_read_enabled() {
            ReadEnabledStatus::Disabled => MP_REGION_CFG::RD_EN_0.val(DISABLE_MAGIC_VALUE),
            ReadEnabledStatus::Enabled => MP_REGION_CFG::RD_EN_0.val(ENABLE_MAGIC_VALUE),
        };

        let is_region_enabled = match memory_protection_region.is_enabled() {
            MemoryProtectionRegionStatus::Disabled => MP_REGION_CFG::EN_0.val(DISABLE_MAGIC_VALUE),
            MemoryProtectionRegionStatus::Enabled => MP_REGION_CFG::EN_0.val(ENABLE_MAGIC_VALUE),
        };

        memory_protection_region_register.modify(
            high_endurance_enabled
                + scramble_enabled
                + ecc_enabled
                + erase_enabled
                + write_enabled
                + read_enabled
                + is_region_enabled,
        );
    }

    /// Configure the access permissions associated with the given info0 memory protection region,
    /// bank0.
    ///
    /// # Parameters
    ///
    /// + `info0_page_index`: the [Info0PageIndex] indicating the region to be configured
    /// +  `field_value`: the value to be written to the register indicating the region
    /// configuration
    fn configure_access_bank0_info0_memory_protection_region(
        &self,
        info0_page_index: Info0PageIndex,
        field_value: FieldValue<u32, BANK0_INFO0_PAGE_CFG::Register>,
    ) {
        // PANIC: Info0PageIndex guarantees correct access to bank0_info0_page_cfg
        let register = self
            .registers
            .bank0_info0_page_cfg
            .get(info0_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

    /// Configure the access permissions associated with the given info0 memory protection region,
    /// bank1.
    ///
    /// # Parameters
    ///
    /// + `info0_page_index`: the [Info0PageIndex] indicating the region to be configured
    /// +  `field_value`: the value to be written to the register indicating the region
    /// configuration
    fn configure_access_bank1_info0_memory_protection_region(
        &self,
        info0_page_index: Info0PageIndex,
        field_value: FieldValue<u32, BANK1_INFO0_PAGE_CFG::Register>,
    ) {
        // PANIC: Info0PageIndex guarantees correct access to bank1_info0_page_cfg
        let register = self
            .registers
            .bank1_info0_page_cfg
            .get(info0_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

    /// Configure the access permissions associated with the given info1 memory protection region,
    /// bank0.
    ///
    /// # Parameters
    ///
    /// + `info1_page_index`: the [Info1PageIndex] indicating the region to be configured
    /// +  `field_value`: the value to be written to the register indicating the region
    /// configuration
    fn configure_access_bank0_info1_memory_protection_region(
        &self,
        info1_page_index: Info1PageIndex,
        field_value: FieldValue<u32, BANK0_INFO1_PAGE_CFG::Register>,
    ) {
        // PANIC: Info1PageIndex guarantees correct access to bank0_info1_page_cfg
        let register = self
            .registers
            .bank0_info1_page_cfg
            .get(info1_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

    /// Configure the access permissions associated with the given info1 memory protection region,
    /// bank1.
    ///
    /// # Parameters
    ///
    /// + `info1_page_index`: the [Info1PageIndex] indicating the region to be configured
    /// +  `field_value`: the value to be written to the register indicating the region
    /// configuration
    fn configure_access_bank1_info1_memory_protection_region(
        &self,
        info1_page_index: Info1PageIndex,
        field_value: FieldValue<u32, BANK1_INFO1_PAGE_CFG::Register>,
    ) {
        // PANIC: Info1PageIndex guarantees correct access to bank1_info1_page_cfg
        let register = self
            .registers
            .bank1_info1_page_cfg
            .get(info1_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

    /// Configure the access permissions associated with the given info2 memory protection region,
    /// bank0.
    ///
    /// # Parameters
    ///
    /// + `info2_page_index`: the [Info2PageIndex] indicating the region to be configured
    /// +  `field_value`: the value to be written to the register indicating the region
    /// configuration
    fn configure_access_bank0_info2_memory_protection_region(
        &self,
        info2_page_index: Info2PageIndex,
        field_value: FieldValue<u32, BANK0_INFO2_PAGE_CFG::Register>,
    ) {
        // PANIC: Info2PageIndex guarantees correct access to bank0_info2_page_cfg
        let register = self
            .registers
            .bank0_info2_page_cfg
            .get(info2_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

    /// Configure the access permissions associated with the given info2 memory protection region,
    /// bank1.
    ///
    /// # Parameters
    ///
    /// + `info2_page_index`: the [Info2PageIndex] indicating the region to be configured
    /// +  `field_value`: the value to be written to the register indicating the region
    /// configuration
    fn configure_access_bank1_info2_memory_protection_region(
        &self,
        info2_page_index: Info2PageIndex,
        field_value: FieldValue<u32, BANK1_INFO2_PAGE_CFG::Register>,
    ) {
        // PANIC: Info2PageIndex guarantees correct access to bank1_info2_page_cfg
        let register = self
            .registers
            .bank1_info2_page_cfg
            .get(info2_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

    /// Configure access permissions for info0 memory protection region.
    ///
    /// # Parameters
    ///
    /// + `info0_memory_protection_region_index`: [Info0MemoryProtectionRegionIndex] indicating the
    /// info0 memory protection region to be configured
    /// + `info0_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    fn configure_access_info0_memory_protection_region(
        &self,
        info0_memory_protection_region_index: Info0MemoryProtectionRegionIndex,
        info0_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        match info0_memory_protection_region_index {
            Info0MemoryProtectionRegionIndex::Bank0(info0_page_index) => self
                .configure_access_bank0_info0_memory_protection_region(
                    info0_page_index,
                    convert_bank0_info0_memory_protection_region_to_register_value(
                        info0_memory_protection_region,
                    ),
                ),
            Info0MemoryProtectionRegionIndex::Bank1(info0_page_index) => self
                .configure_access_bank1_info0_memory_protection_region(
                    info0_page_index,
                    convert_bank1_info0_memory_protection_region_to_register_value(
                        info0_memory_protection_region,
                    ),
                ),
        }
    }

    /// Configure access permissions for info1 memory protection region.
    ///
    /// # Parameters
    ///
    /// + `info1_memory_protection_region_index`: [Info1MemoryProtectionRegionIndex] indicating the
    /// info1 memory protection region to be configured
    /// + `info1_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    fn configure_access_info1_memory_protection_region(
        &self,
        info1_memory_protection_region_index: Info1MemoryProtectionRegionIndex,
        info1_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        match info1_memory_protection_region_index {
            Info1MemoryProtectionRegionIndex::Bank0(info1_page_index) => self
                .configure_access_bank0_info1_memory_protection_region(
                    info1_page_index,
                    convert_bank0_info1_memory_protection_region_to_register_value(
                        info1_memory_protection_region,
                    ),
                ),
            Info1MemoryProtectionRegionIndex::Bank1(info1_page_index) => self
                .configure_access_bank1_info1_memory_protection_region(
                    info1_page_index,
                    convert_bank1_info1_memory_protection_region_to_register_value(
                        info1_memory_protection_region,
                    ),
                ),
        }
    }

    /// Configure access permissions for info2 memory protection region.
    ///
    /// # Parameters
    ///
    /// + `info2_memory_protection_region_index`: [Info2MemoryProtectionRegionIndex] indicating the
    /// info2 memory protection region to be configured
    /// + `info2_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    fn configure_access_info2_memory_protection_region(
        &self,
        info2_memory_protection_region_index: Info2MemoryProtectionRegionIndex,
        info2_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        match info2_memory_protection_region_index {
            Info2MemoryProtectionRegionIndex::Bank0(info2_page_index) => self
                .configure_access_bank0_info2_memory_protection_region(
                    info2_page_index,
                    convert_bank0_info2_memory_protection_region_to_register_value(
                        info2_memory_protection_region,
                    ),
                ),
            Info2MemoryProtectionRegionIndex::Bank1(info2_page_index) => self
                .configure_access_bank1_info2_memory_protection_region(
                    info2_page_index,
                    convert_bank1_info2_memory_protection_region_to_register_value(
                        info2_memory_protection_region,
                    ),
                ),
        }
    }

    /// Configure access permissions for data memory protection region.
    ///
    /// # Parameters
    ///
    /// + `data_memory_protection_region_index`: [DataMemoryProtectionRegionIndex] indicating the
    /// data memory protection region to be configured
    /// + `data_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    pub(super) fn configure_data_memory_protection_region(
        &self,
        index: DataMemoryProtectionRegionIndex,
        data_memory_protection_region: &DataMemoryProtectionRegion,
    ) {
        // PANIC: DataMemoryProtectionRegionIndex is a type that guarantees correct accesses to all
        // memory protection region arrays
        let memory_protection_region_register =
            self.registers.mp_region.get(index.inner()).unwrap();

        self.configure_area_data_memory_protection_region(
            memory_protection_region_register,
            data_memory_protection_region,
        );

        // PANIC: DataMemoryProtectionRegionIndex is a type that guarantees correct accesses to all
        // memory protection region arrays
        let memory_protection_configure_region_register =
            self.registers.mp_region_cfg.get(index.inner()).unwrap();

        self.configure_access_data_memory_protection_region(
            memory_protection_configure_region_register,
            data_memory_protection_region,
        );
    }

    /// Lock data memory protection region.
    ///
    /// After locking, the region can no longer be configured.
    ///
    /// # Parameters
    ///
    /// + `data_memory_protection_region_index`: the [DataMemoryProtectionRegionIndex] indicating
    /// the region to be locked.
    fn lock_data_memory_protection_region(
        &self,
        data_memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) {
        // PANIC: DataMemoryProtectionRegionIndex is a type that guarantees panic accesses to all
        // memory protection region arrays
        let memory_protection_region_lock_write_enable_register = self
            .registers
            .region_cfg_regwen
            .get(data_memory_protection_region_index.inner())
            .unwrap();

        memory_protection_region_lock_write_enable_register
            .modify(REGION_CFG_REGWEN::REGION_0::REGION_LOCKED);
    }

    /// Configure and lock a data memory protection region
    ///
    /// # Parameters
    ///
    /// + `index`: [DataMemoryProtectionRegionIndex] indicating the region to be configured and
    /// locked.
    /// + `data_memory_protection_region`: [DataMemoryProtectionRegion] configuration
    fn configure_and_lock_data_memory_protection_region(
        &self,
        index: DataMemoryProtectionRegionIndex,
        data_memory_protection_region: &DataMemoryProtectionRegion,
    ) {
        self.configure_data_memory_protection_region(index, data_memory_protection_region);
        self.lock_data_memory_protection_region(index);
    }

    /// Configure info0 memory protection region
    ///
    /// # Parameters
    ///
    /// + `info0_memory_protection_region_index`: [Info0MemoryProtectionRegionIndex] indicating the
    /// info0 memory protection region to be configured.
    /// + `info0_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    pub(super) fn configure_info0_memory_protection_region(
        &self,
        info0_memory_protection_region_index: Info0MemoryProtectionRegionIndex,
        info0_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        self.configure_access_info0_memory_protection_region(
            info0_memory_protection_region_index,
            info0_memory_protection_region,
        );
    }

    fn lock_bank0_info0_memory_protection_region(&self, info0_page_index: Info0PageIndex) {
        // PANIC: Info0PageIndex guarantees correct access to bank0_info0_page_cfg
        let register = self
            .registers
            .bank0_info0_regwen
            .get(info0_page_index.to_usize())
            .unwrap();

        register.modify(BANK0_INFO0_REGWEN::REGION_0::PAGE_LOCKED);
    }

    /// Configure and lock a info2 memory protection region
    ///
    /// # Parameters
    ///
    /// + `index`: [Info2MemoryProtectionRegionIndex] indicating the region to be configured and
    /// locked.
    /// + `info2_memory_protection_region`: [Info2MemoryProtectionRegion] configuration
    fn lock_bank1_info0_memory_protection_region(&self, info0_page_index: Info0PageIndex) {
        // PANIC: Info0PageIndex guarantees correct access to bank0_info0_page_cfg
        let register = self
            .registers
            .bank1_info0_regwen
            .get(info0_page_index.to_usize())
            .unwrap();

        register.modify(BANK1_INFO0_REGWEN::REGION_0::PAGE_LOCKED);
    }

    fn lock_info0_memory_protection_region(
        &self,
        info0_memory_protection_region_index: Info0MemoryProtectionRegionIndex,
    ) {
        match info0_memory_protection_region_index {
            Info0MemoryProtectionRegionIndex::Bank0(info0_page_index) => {
                self.lock_bank0_info0_memory_protection_region(info0_page_index)
            }
            Info0MemoryProtectionRegionIndex::Bank1(info0_page_index) => {
                self.lock_bank1_info0_memory_protection_region(info0_page_index)
            }
        }
    }

    fn configure_and_lock_info0_memory_protection_region(
        &self,
        index: Info0MemoryProtectionRegionIndex,
        info0_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        self.configure_info0_memory_protection_region(index, info0_memory_protection_region);
        self.lock_info0_memory_protection_region(index);
    }

    /// Configure info1 memory protection region
    ///
    /// # Parameters
    ///
    /// + `info1_memory_protection_region_index`: [Info1MemoryProtectionRegionIndex] indicating the
    /// info1 memory protection region to be configured.
    /// + `info1_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    pub(super) fn configure_info1_memory_protection_region(
        &self,
        info1_memory_protection_region_index: Info1MemoryProtectionRegionIndex,
        info1_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        self.configure_access_info1_memory_protection_region(
            info1_memory_protection_region_index,
            info1_memory_protection_region,
        );
    }

    fn lock_bank0_info1_memory_protection_region(&self, info1_page_index: Info1PageIndex) {
        // PANIC: Info1PageIndex guarantees correct access to bank0_info1_page_cfg
        let register = self
            .registers
            .bank0_info1_regwen
            .get(info1_page_index.to_usize())
            .unwrap();

        register.modify(BANK0_INFO1_REGWEN::REGION_0::PAGE_LOCKED);
    }

    fn lock_bank1_info1_memory_protection_region(&self, info1_page_index: Info1PageIndex) {
        // PANIC: Info1PageIndex guarantees correct access to bank0_info1_page_cfg
        let register = self
            .registers
            .bank1_info1_regwen
            .get(info1_page_index.to_usize())
            .unwrap();

        register.modify(BANK1_INFO1_REGWEN::REGION_0::PAGE_LOCKED);
    }

    fn lock_info1_memory_protection_region(
        &self,
        info1_memory_protection_region_index: Info1MemoryProtectionRegionIndex,
    ) {
        match info1_memory_protection_region_index {
            Info1MemoryProtectionRegionIndex::Bank0(info1_page_index) => {
                self.lock_bank0_info1_memory_protection_region(info1_page_index)
            }
            Info1MemoryProtectionRegionIndex::Bank1(info1_page_index) => {
                self.lock_bank1_info1_memory_protection_region(info1_page_index)
            }
        }
    }

    fn configure_and_lock_info1_memory_protection_region(
        &self,
        index: Info1MemoryProtectionRegionIndex,
        info1_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        self.configure_info1_memory_protection_region(index, info1_memory_protection_region);
        self.lock_info1_memory_protection_region(index);
    }

    /// Configure info2 memory protection region
    ///
    /// # Parameters
    ///
    /// + `info2_memory_protection_region_index`: [Info2MemoryProtectionRegionIndex] indicating the
    /// info2 memory protection region to be configured.
    /// + `info2_memory_protection_region`: [InfoMemoryProtectionRegion] configuration
    pub(super) fn configure_info2_memory_protection_region(
        &self,
        info2_memory_protection_region_index: Info2MemoryProtectionRegionIndex,
        info2_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        self.configure_access_info2_memory_protection_region(
            info2_memory_protection_region_index,
            info2_memory_protection_region,
        );
    }

    fn lock_bank0_info2_memory_protection_region(&self, info2_page_index: Info2PageIndex) {
        // PANIC: Info2PageIndex guarantees correct access to bank0_info2_page_cfg
        let register = self
            .registers
            .bank0_info2_regwen
            .get(info2_page_index.to_usize())
            .unwrap();

        register.modify(BANK0_INFO2_REGWEN::REGION_0::PAGE_LOCKED);
    }

    fn lock_bank1_info2_memory_protection_region(&self, info2_page_index: Info2PageIndex) {
        // PANIC: Info2PageIndex guarantees correct access to bank0_info2_page_cfg
        let register = self
            .registers
            .bank1_info2_regwen
            .get(info2_page_index.to_usize())
            .unwrap();

        register.modify(BANK1_INFO2_REGWEN::REGION_0::PAGE_LOCKED);
    }

    fn lock_info2_memory_protection_region(
        &self,
        info2_memory_protection_region_index: Info2MemoryProtectionRegionIndex,
    ) {
        match info2_memory_protection_region_index {
            Info2MemoryProtectionRegionIndex::Bank0(info2_page_index) => {
                self.lock_bank0_info2_memory_protection_region(info2_page_index)
            }
            Info2MemoryProtectionRegionIndex::Bank1(info2_page_index) => {
                self.lock_bank1_info2_memory_protection_region(info2_page_index)
            }
        }
    }

    fn configure_and_lock_info2_memory_protection_region(
        &self,
        index: Info2MemoryProtectionRegionIndex,
        info2_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        self.configure_info2_memory_protection_region(index, info2_memory_protection_region);
        self.lock_info2_memory_protection_region(index);
    }

    fn configure_data_memory_protection_regions(
        &self,
        data_memory_protection_regions: &DataMemoryProtectionRegionList,
    ) {
        for (data_memory_protection_region_index, data_memory_protection_region) in
            data_memory_protection_regions.as_iterator()
        {
            self.configure_and_lock_data_memory_protection_region(
                data_memory_protection_region_index,
                data_memory_protection_region,
            );
        }
    }

    /// Configure all info0 memory protection regions
    ///
    /// # Parameters
    ///
    /// + `info0_memory_protection_regions`: the list of info0 memory protection regions to be
    /// configured
    fn configure_info0_memory_protection_regions(
        &self,
        info0_memory_protection_regions: &Info0MemoryProtectionRegionList,
    ) {
        for (info0_memory_protection_region_index, info0_memory_protection_region) in
            info0_memory_protection_regions.as_iterator()
        {
            self.configure_and_lock_info0_memory_protection_region(
                info0_memory_protection_region_index,
                info0_memory_protection_region,
            );
        }
    }

    /// Configure all info1 memory protection regions
    ///
    /// # Parameters
    ///
    /// + `info1_memory_protection_regions`: the list of info1 memory protection regions to be
    /// configured
    fn configure_info1_memory_protection_regions(
        &self,
        info1_memory_protection_regions: &Info1MemoryProtectionRegionList,
    ) {
        for (info1_memory_protection_region_index, info1_memory_protection_region) in
            info1_memory_protection_regions.as_iterator()
        {
            self.configure_and_lock_info1_memory_protection_region(
                info1_memory_protection_region_index,
                info1_memory_protection_region,
            );
        }
    }

    /// Configure all info2 memory protection regions
    ///
    /// # Parameters
    ///
    /// + `info2_memory_protection_regions`: the list of info2 memory protection regions to be
    /// configured
    fn configure_info2_memory_protection_regions(
        &self,
        info2_memory_protection_regions: &Info2MemoryProtectionRegionList,
    ) {
        for (info2_memory_protection_region_index, info2_memory_protection_region) in
            info2_memory_protection_regions.as_iterator()
        {
            self.configure_and_lock_info2_memory_protection_region(
                info2_memory_protection_region_index,
                info2_memory_protection_region,
            );
        }
    }

    /// Enable all required flash interrupts
    fn enable_interrupts(&self) {
        self.registers
            .intr_enable
            .modify(INTR::CORR_ERR::SET + INTR::OP_DONE::SET);
    }

    /// Configure the flash memory protection
    ///
    /// # Parameters
    ///
    /// + `memory_protection_configuration`: flash memory protection configuration to be configured
    fn configure_memory_protection(
        &self,
        memory_protection_configuration: MemoryProtectionConfiguration,
    ) {
        self.configure_default_region_permissions(
            memory_protection_configuration.get_default_memory_protection_region(),
        );
        self.configure_data_memory_protection_regions(
            memory_protection_configuration.get_data_memory_protection_regions(),
        );
        self.configure_info0_memory_protection_regions(
            memory_protection_configuration.get_info0_memory_protection_regions(),
        );
        self.configure_info1_memory_protection_regions(
            memory_protection_configuration.get_info1_memory_protection_regions(),
        );
        self.configure_info2_memory_protection_regions(
            memory_protection_configuration.get_info2_memory_protection_regions(),
        );
    }

    /// Check if the flash peripheral is busy
    ///
    /// # Return value
    ///
    /// [BusyStatus] indicating if the flash peripheral is busy
    pub(super) fn is_busy(&self) -> BusyStatus {
        self.is_busy.get()
    }

    /// Convert erase type to register value
    ///
    /// Converts [EraseType] to a value suitable to be written to a register
    ///
    /// # Parameters
    ///
    /// + `erase_type`: [EraseType] to be converted
    ///
    /// # Return value
    ///
    /// The corresponding register value
    const fn convert_erase_type_to_register_value(
        erase_type: EraseType,
    ) -> FieldValue<u32, CONTROL::Register> {
        match erase_type {
            EraseType::PageErase => CONTROL::ERASE_SEL::PAGE_ERASE,
            EraseType::BankErase => CONTROL::ERASE_SEL::BANK_ERASE,
        }
    }

    /// Convert partition type to register value
    ///
    /// Converts [PartitionType] to a value suitable to be written to a register
    ///
    /// # Parameters
    ///
    /// + `partition_type`: [PartitionType] to be converted
    ///
    /// # Return value
    ///
    /// The corresponding register value
    const fn convert_partition_type_to_register_value(
        partition_type: PartitionType,
    ) -> FieldValue<u32, CONTROL::Register> {
        match partition_type {
            // CLEAR === DATA
            PartitionType::Data => CONTROL::PARTITION_SEL::CLEAR,
            // SET === INFO
            PartitionType::Info => CONTROL::PARTITION_SEL::SET,
        }
    }

    /// Convert info partition type to register value
    ///
    /// Converts [InfoPartitionType] to a value suitable to be written to a register
    ///
    /// # Parameters
    ///
    /// + `info_partition_type`: [InfoPartitionType] to be converted
    ///
    /// # Return value
    ///
    /// The corresponding register value
    const fn convert_info_partition_type_to_register_value(
        info_partition_type: InfoPartitionType,
    ) -> FieldValue<u32, CONTROL::Register> {
        match info_partition_type {
            InfoPartitionType::Type0 => CONTROL::INFO_SEL.val(0),
            InfoPartitionType::Type1 => CONTROL::INFO_SEL.val(1),
            InfoPartitionType::Type2 => CONTROL::INFO_SEL.val(2),
        }
    }

    /// Read a word from the read FIFO
    ///
    /// # Return value
    ///
    /// The read word
    fn read_word(&self) -> usize {
        // usize == u32 on Earlgrey
        self.registers.rd_fifo[0].get() as usize
    }

    /// Read a chunk from the read FIFO
    ///
    /// # Parameters
    ///
    /// + `chunk_iterator`: a word iterator over a mutable chunk where read words will be stored
    fn read_chunk(&self, chunk_iterator: MutableChunkIterator) {
        for word in chunk_iterator {
            *word = self.read_word();
        }
    }

    /// Read a chunk from the read FIFO and store it in the currently registered page chunk
    /// iterator
    ///
    /// # Panic
    ///
    /// This method panics if there is no page chunk stored.
    fn read_data(&self) {
        self.operate_on_page_chunk_iterator(|page_chunk_iterator| {
            if let Some(chunk) = page_chunk_iterator.next_mutable() {
                let chunk_iterator = MutableChunkIterator::new(chunk);
                self.read_chunk(chunk_iterator);
            }
        });
    }

    /// Make flash peripheral read the next available [Chunk].
    ///
    /// # Panic
    ///
    /// This method panics if there is no page chunk stored.
    fn start_next_read(&self) {
        self.operate_on_page_chunk_iterator(|page_chunk_iterator| {
            self.configure_address_register(page_chunk_iterator.get_current_chunk_flash_addres());
            self.start_flash_operation();
        });
    }

    /// Write a word to the program FIFO
    ///
    /// # Parameters
    ///
    /// + `word`: the word to be written
    fn write_word(&self, word: usize) {
        self.registers.prog_fifo[0].set(word as u32);
    }

    /// Write the given chunk to the program FIFO
    ///
    /// # Parameters
    ///
    /// + `chunk_iterator`: an iterator over all words of the chunk
    fn write_chunk(&self, chunk_iterator: ImmutableChunkIterator) {
        for &word in chunk_iterator {
            self.write_word(word);
        }
    }

    /// Make the flash write the next available [Chunk] from the stored page chunk iterator
    ///
    /// # Panic
    ///
    /// This method panics if there is no page chunk stored.
    fn write_data(&self) {
        self.operate_on_page_chunk_iterator(|page_chunk_iterator| {
            if let Some(immutable_page_chunk_iterator_item) = page_chunk_iterator.next_immutable() {
                let (chunk, chunk_flash_address) = immutable_page_chunk_iterator_item.inner();
                self.configure_address_register(chunk_flash_address);
                self.start_flash_operation();
                let chunk_iterator = ImmutableChunkIterator::new(chunk);
                self.write_chunk(chunk_iterator);
            }
        });
    }

    /// Converts a raw page number to data page index and bank.
    ///
    /// A raw page number is a page number provided by a capsule. This method attempts to map the
    /// given raw page number to a data page index and bank.
    ///
    /// # Parameters:
    ///
    /// + `page_number`: the given raw page number
    ///
    /// # Return value
    ///
    /// + Ok((data_page_index, bank)): if the raw page number is valid
    /// + Err(()): if the raw page number is invalid
    const fn convert_raw_page_number_to_data_page_index_and_bank(
        page_number: usize,
    ) -> Result<(DataPageIndex, Bank), ()> {
        if page_number < DATA_PAGES_PER_BANK.get() {
            // CAST: Because of the if condition, page_number < 256, so the cast is safe.
            let data_page_index = DataPageIndex::new(page_number as u8);
            Ok((data_page_index, Bank::Bank0))
        } else if page_number < 2 * DATA_PAGES_PER_BANK.get() {
            // CAST: Because of the if condition, page_number - DATA_PAGES_PER_BANK < 256, so the cast is safe.
            let data_page_index =
                DataPageIndex::new((page_number - DATA_PAGES_PER_BANK.get()) as u8);
            Ok((data_page_index, Bank::Bank1))
        } else {
            Err(())
        }
    }

    /// Configure the control register for read/write data partition
    ///
    /// Configures the control register for reading/writing data from data partitions.
    ///
    /// # Parameters
    ///
    /// + `number_bus_words`: the number of bus words to be read/written by the next operation
    /// + `flash_operation_type`: the desired flash operation type
    fn configure_control_register_for_read_write_data_partition(
        &self,
        number_bus_words: NonZeroUsize,
        flash_operation_type: ReadWriteFlashOperationType,
    ) {
        // The NUM field value must be configured to the number of bus words to be written minus 1
        let number_bus_words = CONTROL::NUM.val((number_bus_words.get() - 1) as u32);

        let partition_type_select =
            Self::convert_partition_type_to_register_value(PartitionType::Data);

        let operation_type = flash_operation_type.into();

        self.registers.control.modify(
            number_bus_words
                + partition_type_select
                + CONTROL::PROG_SEL::NORMAL_PROGRAM
                + operation_type
                + CONTROL::START::CLEAR,
        );
    }

    /// Configure the control register for erase data partition
    ///
    /// Configures the control register for erasing data from data partitions.
    ///
    /// # Parameters
    ///
    /// + `erase_type`: the type of the desired erase
    fn configure_control_register_for_erase_data_partition(&self, erase_type: EraseType) {
        let partition_type_select =
            Self::convert_partition_type_to_register_value(PartitionType::Data);

        let erase_type_select = Self::convert_erase_type_to_register_value(erase_type);

        let operation_type = CONTROL::OP::ERASE;

        self.registers.control.modify(
            partition_type_select + erase_type_select + operation_type + CONTROL::START::CLEAR,
        );
    }

    /// Configure the control register for the next operation to be carried on data partitions.
    ///
    /// + For read/write, configure the peripheral for reading/writing a [Chunk].
    /// + For erase, configure the peripheral for erasing a page.
    ///
    /// # Parameters:
    ///
    /// + `flash_operation_type`: indicates the desired flash operation
    fn configure_control_register_for_page_operation_data_partition(
        &self,
        flash_operation_type: FlashOperationType,
    ) {
        match flash_operation_type {
            FlashOperationType::Erase => {
                self.configure_control_register_for_erase_data_partition(EraseType::PageErase)
            }
            FlashOperationType::Read => self
                .configure_control_register_for_read_write_data_partition(
                    WORDS_PER_CHUNK,
                    ReadWriteFlashOperationType::Read,
                ),
            FlashOperationType::Write => self
                .configure_control_register_for_read_write_data_partition(
                    WORDS_PER_CHUNK,
                    ReadWriteFlashOperationType::Write,
                ),
        }
    }

    /// Configure the control register for read/write info partition
    ///
    /// Configures the control register for reading/writing data from info partitions.
    ///
    /// # Parameters
    ///
    /// + `info_partition_type`: the desired info partition to be affected by the next read/write
    /// operation
    /// + `number_bus_words`: the number of bus words to be read/written by the next operation
    /// + `flash_operation_type`: the desired read/write flash operation type
    fn configure_control_register_for_read_write_info_partition(
        &self,
        info_partition_type: InfoPartitionType,
        number_bus_words: NonZeroUsize,
        flash_operation_type: ReadWriteFlashOperationType,
    ) {
        // The NUM field value must be configured to the number of bus words to be written minus 1
        let number_bus_words = CONTROL::NUM.val((number_bus_words.get() - 1) as u32);

        let info_partition_type_select =
            Self::convert_info_partition_type_to_register_value(info_partition_type);

        let partition_type_select =
            Self::convert_partition_type_to_register_value(PartitionType::Info);

        let operation_type = flash_operation_type.into();

        self.registers.control.modify(
            number_bus_words
                + info_partition_type_select
                + partition_type_select
                + CONTROL::PROG_SEL::NORMAL_PROGRAM
                + operation_type
                + CONTROL::START::CLEAR,
        );
    }

    /// Configure the control register for erase info partition
    ///
    /// Configures the control register for erasing data from info partitions.
    ///
    /// # Parameters
    ///
    /// + `info_partition_type`: the desired info partition to be affected by the next erase
    /// operation
    /// + `erase_type`: the type of the desired erase
    fn configure_control_register_for_erase_info_partition(
        &self,
        info_partition_type: InfoPartitionType,
        erase_type: EraseType,
    ) {
        let info_partition_type_select =
            Self::convert_info_partition_type_to_register_value(info_partition_type);

        let partition_type_select =
            Self::convert_partition_type_to_register_value(PartitionType::Info);

        let erase_type_select = Self::convert_erase_type_to_register_value(erase_type);

        let operation_type = CONTROL::OP::ERASE;

        self.registers.control.modify(
            info_partition_type_select
                + partition_type_select
                + erase_type_select
                + operation_type
                + CONTROL::START::CLEAR,
        );
    }

    /// Configure the control register for the next operation to be carried on info partitions.
    ///
    /// + For read/write, configure the peripheral for reading/writing a [Chunk].
    /// + For erase, configure the peripheral for erasing a page.
    ///
    /// # Parameters:
    ///
    /// + `info_partition_type`: the desired info partition to be affected by the next flash
    /// operation
    /// + `flash_operation_type`: indicates the desired flash operation
    fn configure_control_register_for_page_operation_info_partition(
        &self,
        info_partition_type: InfoPartitionType,
        flash_operation_type: FlashOperationType,
    ) {
        match flash_operation_type {
            FlashOperationType::Erase => self.configure_control_register_for_erase_info_partition(
                info_partition_type,
                EraseType::PageErase,
            ),
            FlashOperationType::Read => self
                .configure_control_register_for_read_write_info_partition(
                    info_partition_type,
                    WORDS_PER_CHUNK,
                    ReadWriteFlashOperationType::Read,
                ),
            FlashOperationType::Write => self
                .configure_control_register_for_read_write_info_partition(
                    info_partition_type,
                    WORDS_PER_CHUNK,
                    ReadWriteFlashOperationType::Write,
                ),
        }
    }

    /// Configures the address register
    ///
    /// # Parameters
    ///
    /// + `flash_address`: the starting address for the next flash operation
    fn configure_address_register(&self, flash_address: FlashAddress) {
        self.registers
            .addr
            .modify(ADDR::START.val(flash_address.to_usize() as u32));
    }

    /// Configure control and address registers for the given flash operation on data partition
    ///
    /// # Parameters
    ///
    /// + `data_page_position`: the desired data flash page to be impacted by the next flash
    /// operation
    /// + `flash_operation_type`: the desired flash operation
    fn prepare_page_operation_data_partition(
        &self,
        data_page_position: DataPagePosition,
        flash_operation_type: FlashOperationType,
    ) {
        self.configure_control_register_for_page_operation_data_partition(flash_operation_type);
        self.configure_address_register(data_page_position.to_flash_ptr());
    }

    /// Configure control and address registers for the given flash operation on info partition
    ///
    /// # Parameters
    ///
    /// + `info_page_position`: the desired info flash page to be impacted by the next flash
    /// operation
    /// + `flash_operation_type`: the desired flash operation
    fn prepare_page_operation_info_partition(
        &self,
        info_page_position: InfoPagePosition,
        flash_operation_type: FlashOperationType,
    ) {
        let info_partition_type = info_page_position.to_info_partition_type();

        self.configure_control_register_for_page_operation_info_partition(
            info_partition_type,
            flash_operation_type,
        );
        self.configure_address_register(info_page_position.to_flash_ptr());
    }

    /// Determine whether the read FIFO is empty
    ///
    /// # Return value
    ///
    /// + false: the read FIFO is not empty
    /// + true: the read FIFO is empty
    pub(super) fn is_status_rd_empty_set(&self) -> bool {
        self.registers.status.is_set(STATUS::RD_EMPTY)
    }

    /// Flush any data that the read FIFO may contain
    fn flush_read_buffer(&self) {
        while !self.is_status_rd_empty_set() {
            self.read_word();
        }
    }

    /// Start flash operation
    ///
    /// The control and address registers must be configured by a
    /// [prepare_page_operation_data_partition] or [prepare_page_operation_info_partition] call.
    fn start_flash_operation(&self) {
        self.registers.control.modify(CONTROL::START::SET);
        self.is_busy.set(BusyStatus::Busy);
    }

    /// Stop flash operation
    fn stop_flash_operation(&self) {
        self.registers.control.modify(CONTROL::START::CLEAR);
        self.is_busy.set(BusyStatus::NotBusy);
    }

    /// Take the stored page chunk iterator
    ///
    /// # Return value
    ///
    /// [Option<PageChunkIterator>] representing the current stored page chunk iterator
    /// or `None` if none is present.
    fn take_page_chunk_iterator(&self) -> Option<PageChunkIterator<'static>> {
        self.page_chunk_iterator.take()
    }

    /// Store the given page chunk iterator
    ///
    /// # Parameters
    ///
    /// + `page_chunk_iterator`: the page chunk iterator to be stored
    fn set_page_chunk_iterator(&self, page_chunk_iterator: PageChunkIterator<'static>) {
        self.page_chunk_iterator.set(page_chunk_iterator);
    }

    /// Helper function to apply a closure on the stored page chunk iterator.
    /// No-op if there is no page chunk iterator stored.
    ///
    /// # Parameters
    ///
    /// + `closure`: the closure to be applied on the stored page chunk iterator
    fn operate_on_page_chunk_iterator<F>(&self, closure: F)
    where
        F: FnOnce(&mut PageChunkIterator<'static>),
    {
        if let Some(mut page_chunk_iterator) = self.take_page_chunk_iterator() {
            closure(&mut page_chunk_iterator);
            self.set_page_chunk_iterator(page_chunk_iterator);
        };
    }

    /// Create a new info page position
    ///
    /// # Parameters
    ///
    /// + `info_type`: the type of the page
    /// + `bank`: the bank position of the page
    /// + `raw_page_number`: the raw page number of the page
    ///
    /// # Return value
    ///
    /// + Ok(InfoPagePosition) if `raw_page_number` is valid
    /// + Err(()) if `raw_page_number` is invalid
    fn new_info_page_position(
        info_type: InfoPartitionType,
        bank: Bank,
        raw_page_number: usize,
    ) -> Result<InfoPagePosition, ()> {
        match info_type {
            InfoPartitionType::Type0 => Ok(InfoPagePosition::Type0(Info0PagePosition::new(
                bank,
                Info0PageIndex::new(raw_page_number)?,
            ))),
            InfoPartitionType::Type1 => Ok(InfoPagePosition::Type1(Info1PagePosition::new(
                bank,
                Info1PageIndex::new(raw_page_number)?,
            ))),
            InfoPartitionType::Type2 => Ok(InfoPagePosition::Type2(Info2PagePosition::new(
                bank,
                Info2PageIndex::new(raw_page_number)?,
            ))),
        }
    }

    /// Read a data page
    ///
    /// As all internal methods, it assumes valid parameters through the type system.
    ///
    /// # Parameters
    ///
    /// + `page`: the data page to be read
    fn internal_read_data_page(
        &self,
        page: DataFlashCtrlPage<'static>,
    ) -> Result<(), (ErrorCode, DataFlashCtrlPage<'static>)> {
        if self.is_busy() == BusyStatus::Busy {
            return Err((ErrorCode::BUSY, page));
        }

        self.prepare_page_operation_data_partition(page.get_position(), FlashOperationType::Read);
        let page_chunk_iterator = PageChunkIterator::new(FlashCtrlPage::DataPage(page));
        self.set_page_chunk_iterator(page_chunk_iterator);
        self.start_next_read();

        Ok(())
    }

    /// Write a data page
    ///
    /// As all internal methods, it assumes valid parameters through the type system.
    ///
    /// # Parameters
    ///
    /// + `page`: the data page to be written
    fn internal_write_data_page(
        &self,
        page: DataFlashCtrlPage<'static>,
    ) -> Result<(), (ErrorCode, DataFlashCtrlPage<'static>)> {
        if self.is_busy() == BusyStatus::Busy {
            return Err((ErrorCode::BUSY, page));
        }

        self.prepare_page_operation_data_partition(page.get_position(), FlashOperationType::Write);
        let page_chunk_iterator = PageChunkIterator::new(FlashCtrlPage::DataPage(page));
        self.set_page_chunk_iterator(page_chunk_iterator);
        self.write_data();

        Ok(())
    }

    /// Erase a data page
    ///
    /// As all internal methods, it assumes valid parameters through the type system.
    ///
    /// # Parameters
    ///
    /// + `data_page_position`: indicates what page to be erased
    fn internal_erase_data_page(
        &self,
        data_page_position: DataPagePosition,
    ) -> Result<(), ErrorCode> {
        if self.is_busy() == BusyStatus::Busy {
            return Err(ErrorCode::BUSY);
        }

        self.prepare_page_operation_data_partition(data_page_position, FlashOperationType::Erase);
        self.start_flash_operation();

        Ok(())
    }

    /// Read a info page
    ///
    /// As all internal methods, it assumes valid parameters through the type system.
    ///
    /// # Parameters
    ///
    /// + `page`: the info page to be read
    fn internal_read_info_page(
        &self,
        page: InfoFlashCtrlPage<'static>,
    ) -> Result<(), (ErrorCode, InfoFlashCtrlPage<'static>)> {
        if self.is_busy() == BusyStatus::Busy {
            return Err((ErrorCode::BUSY, page));
        }

        self.prepare_page_operation_info_partition(page.get_position(), FlashOperationType::Read);
        let page_chunk_iterator = PageChunkIterator::new(FlashCtrlPage::InfoPage(page));
        self.set_page_chunk_iterator(page_chunk_iterator);
        self.start_next_read();

        Ok(())
    }

    /// Write a info page
    ///
    /// As all internal methods, it assumes valid parameters through the type system.
    ///
    /// # Parameters
    ///
    /// + `page`: the info page to be written
    fn internal_write_info_page(
        &self,
        page: InfoFlashCtrlPage<'static>,
    ) -> Result<(), (ErrorCode, InfoFlashCtrlPage<'static>)> {
        if self.is_busy() == BusyStatus::Busy {
            return Err((ErrorCode::BUSY, page));
        }

        self.prepare_page_operation_info_partition(page.get_position(), FlashOperationType::Write);
        let page_chunk_iterator = PageChunkIterator::new(FlashCtrlPage::InfoPage(page));
        self.set_page_chunk_iterator(page_chunk_iterator);
        self.write_data();

        Ok(())
    }

    /// Erase a info page
    ///
    /// As all internal methods, it assumes valid parameters through the type system.
    ///
    /// # Parameters
    ///
    /// + `info_page_position`: indicates what page to be erased
    fn internal_erase_info_page(
        &self,
        info_page_position: InfoPagePosition,
    ) -> Result<(), ErrorCode> {
        if self.is_busy() == BusyStatus::Busy {
            return Err(ErrorCode::BUSY);
        }

        self.prepare_page_operation_info_partition(info_page_position, FlashOperationType::Erase);
        self.start_flash_operation();

        Ok(())
    }

    /// Read the configured partition type from the control register
    ///
    /// # Return value
    ///
    /// The configured [PartitionType]
    fn read_partition_type(&self) -> PartitionType {
        match self.registers.control.read(CONTROL::PARTITION_SEL) {
            0b0 => PartitionType::Data,
            // The only other available value is 0b1
            _ => PartitionType::Info,
        }
    }

    /// Read the control register to determine the flash operation that just finished
    ///
    /// # Return value
    ///
    /// [FlashOperationType] indicating the finished operation
    fn get_finished_operation_type(&self) -> FlashOperationType {
        match self.registers.control.read(CONTROL::OP) {
            0b00 => FlashOperationType::Read,
            0b01 => FlashOperationType::Write,
            // The only other available value is 0b10
            _ => FlashOperationType::Erase,
        }
    }

    /// Clear operation done interrupt
    fn clear_operation_done_interrupt(&self) {
        self.registers.intr_state.modify(INTR::OP_DONE::SET);
    }

    /// Clear operation done status
    fn clear_operation_done_status(&self) {
        self.registers.op_status.modify(OP_STATUS::DONE::CLEAR);
    }

    /// Get the error code of the flash operation
    ///
    /// # Return value
    ///
    /// + Some(flash_error_code): an error occurred during the last flash operation
    /// + None: no error occurred during the last flash operation
    fn get_error_code(&self) -> Option<FlashErrorCode> {
        const OP_ERR_VALUE: u32 = ERR_CODE::OP_ERR::SET.value;
        const MP_ERR_VALUE: u32 = ERR_CODE::MP_ERR::SET.value;
        const RD_ERR_VALUE: u32 = ERR_CODE::RD_ERR::SET.value;
        const PROG_ERR_VALUE: u32 = ERR_CODE::PROG_ERR::SET.value;
        const PROG_WIN_ERR_VALUE: u32 = ERR_CODE::PROG_WIN_ERR::SET.value;
        const PROG_TYPE_ERR_VALUE: u32 = ERR_CODE::PROG_TYPE_ERR::SET.value;
        const UPDATE_ERR_VALUE: u32 = ERR_CODE::UPDATE_ERR::SET.value;
        const MACRO_ERR_VALUE: u32 = ERR_CODE::MACRO_ERR::SET.value;

        match self.registers.err_code.get() {
            OP_ERR_VALUE => Some(FlashErrorCode::Operation),
            MP_ERR_VALUE => Some(FlashErrorCode::MemoryProtection),
            RD_ERR_VALUE => Some(FlashErrorCode::Read),
            PROG_ERR_VALUE => Some(FlashErrorCode::Program),
            PROG_WIN_ERR_VALUE => Some(FlashErrorCode::ProgramResolution),
            PROG_TYPE_ERR_VALUE => Some(FlashErrorCode::ProgramType),
            UPDATE_ERR_VALUE => Some(FlashErrorCode::Update),
            MACRO_ERR_VALUE => Some(FlashErrorCode::Macro),
            _ => None,
        }
    }

    /// Convert the specific hardware error code to the HIL error understood by capsules.
    ///
    /// # Parameters
    ///
    /// + `error_code`: the hardware error to be converted
    ///
    /// # Return value
    ///
    /// [flash_hil::Error] indicating the flash error occurred as described by the flash HIL
    fn convert_error_code_to_flash_error(error_code: FlashErrorCode) -> flash_hil::Error {
        match error_code {
            FlashErrorCode::Read | FlashErrorCode::Program => flash_hil::Error::FlashError,
            FlashErrorCode::MemoryProtection => flash_hil::Error::FlashMemoryProtectionError,
            error_code @ (FlashErrorCode::Operation
            | FlashErrorCode::Update
            | FlashErrorCode::Macro
            | FlashErrorCode::ProgramResolution
            | FlashErrorCode::ProgramType) => {
                // This part of code is reached only if the driver malfunctions
                panic!(
                    "Error code {:?} occurred. This means that the driver contains a bug",
                    error_code
                );
            }
        }
    }

    /// Clear error status
    fn clear_error_status(&self) {
        self.registers.op_status.modify(OP_STATUS::ERR::CLEAR);
    }

    /// Clear the given error code. It also clears the error status.
    ///
    /// # Parameters
    ///
    /// + `flash_error_code`: the flash error code to be cleared
    fn clear_error_code(&self, flash_error_code: FlashErrorCode) {
        let clear_value = match flash_error_code {
            FlashErrorCode::Operation => ERR_CODE::OP_ERR::SET,
            FlashErrorCode::MemoryProtection => ERR_CODE::MP_ERR::SET,
            FlashErrorCode::Read => ERR_CODE::RD_ERR::SET,
            FlashErrorCode::Program => ERR_CODE::PROG_ERR::SET,
            FlashErrorCode::ProgramResolution => ERR_CODE::PROG_WIN_ERR::SET,
            FlashErrorCode::ProgramType => ERR_CODE::PROG_TYPE_ERR::SET,
            FlashErrorCode::Update => ERR_CODE::UPDATE_ERR::SET,
            FlashErrorCode::Macro => ERR_CODE::MACRO_ERR::SET,
        };

        self.registers.err_code.modify(clear_value);
        self.clear_error_status();
    }

    /// Clear all error codes. It also clears the error status.
    fn clear_all_error_codes(&self) {
        // 8 possible error codes
        self.registers.err_code.set(0b1111_1111);
        self.clear_error_status();
    }

    fn read_complete(
        &self,
        raw_page: &'static mut RawFlashCtrlPage,
        result: Result<(), flash_hil::Error>,
    ) {
        match self.read_partition_type() {
            PartitionType::Data => self
                .data_client
                .map(|data_client| data_client.read_complete(raw_page, result)),
            PartitionType::Info => self
                .info_client
                .map(|info_client| info_client.info_read_complete(raw_page, result)),
        };
    }

    fn write_complete(
        &self,
        raw_page: &'static mut RawFlashCtrlPage,
        result: Result<(), flash_hil::Error>,
    ) {
        match self.read_partition_type() {
            PartitionType::Data => self
                .data_client
                .map(|data_client| data_client.write_complete(raw_page, result)),
            PartitionType::Info => self
                .info_client
                .map(|info_client| info_client.info_write_complete(raw_page, result)),
        };
    }

    fn erase_complete(&self, result: Result<(), flash_hil::Error>) {
        match self.read_partition_type() {
            PartitionType::Data => self
                .data_client
                .map(|data_client| data_client.erase_complete(result)),
            PartitionType::Info => self
                .info_client
                .map(|info_client| info_client.info_erase_complete(result)),
        };
    }

    /// Handler for operation done interrupt
    pub(crate) fn handle_operation_done(&self) {
        self.clear_operation_done_status();
        self.stop_flash_operation();

        let finished_operation = self.get_finished_operation_type();

        if let Some(error_code) = self.get_error_code() {
            self.clear_error_code(error_code);
            let error = Self::convert_error_code_to_flash_error(error_code);
            match finished_operation {
                FlashOperationType::Read => {
                    self.flush_read_buffer();
                    if let Some(page_chunk_iterator) = self.take_page_chunk_iterator() {
                        let raw_page = page_chunk_iterator.to_raw_page();
                        self.read_complete(raw_page, Err(error));
                    };
                }
                FlashOperationType::Write => {
                    // This may never panic because before an operation starts, the user of the
                    // driver has to provide a reference to a page from which the iterator is
                    // created and stored.
                    if let Some(page_chunk_iterator) = self.take_page_chunk_iterator() {
                        let raw_page = page_chunk_iterator.to_raw_page();
                        self.write_complete(raw_page, Err(error));
                    }
                }
                FlashOperationType::Erase => self.erase_complete(Err(error)),
            }

            return;
        }

        match finished_operation {
            FlashOperationType::Read => {
                self.read_data();
                if let Some(page_chunk_iterator) = self.take_page_chunk_iterator() {
                    let empty_status = page_chunk_iterator.empty();

                    if PageChunkIteratorEmpty::Empty == empty_status {
                        let raw_page = page_chunk_iterator.to_raw_page();
                        self.read_complete(raw_page, Ok(()));
                    } else {
                        self.set_page_chunk_iterator(page_chunk_iterator);
                        self.start_next_read();
                    }
                }
            }
            FlashOperationType::Write => {
                // This may never panic because before an operation starts, the user of the
                // driver has to provide a reference to a page from which the iterator is
                // created and stored.
                if let Some(page_chunk_iterator) = self.take_page_chunk_iterator() {
                    let empty_status = page_chunk_iterator.empty();
                    if PageChunkIteratorEmpty::Empty == empty_status {
                        let raw_page = page_chunk_iterator.to_raw_page();
                        self.write_complete(raw_page, Ok(()));
                    } else {
                        self.page_chunk_iterator.set(page_chunk_iterator);
                        self.write_data();
                    }
                }
            }
            FlashOperationType::Erase => self.erase_complete(Ok(())),
        }
    }

    // This method is only used for tests
    #[cfg(feature = "test_flash_ctrl")]
    pub(super) fn get_registers(&self) -> &StaticRef<FlashCtrlRegisters> {
        &self.registers
    }

    pub fn handle_interrupt(&self, interrupt: FlashCtrlInterrupt) {
        let regs = &self.registers;
        match interrupt {
            FlashCtrlInterrupt::ProgEmpty => {
                regs.intr_state.modify(INTR::PROG_EMPTY::SET);
                // TODO: handle this
            }
            FlashCtrlInterrupt::ProgLvl => {
                regs.intr_state.modify(INTR::PROG_LVL::SET);
                // TODO: handle this
            }
            FlashCtrlInterrupt::RdFull => {
                regs.intr_state.modify(INTR::RD_FULL::SET);
                // TODO: handle this
            }
            FlashCtrlInterrupt::RdLvl => {
                regs.intr_state.modify(INTR::RD_LVL::SET);
                // TODO: handle this
            }
            FlashCtrlInterrupt::OpDone => {
                self.clear_operation_done_interrupt();
                self.handle_operation_done();
            }
            FlashCtrlInterrupt::CorrErr => {
                regs.intr_state.modify(INTR::CORR_ERR::SET);
                // TODO: handle this
            }
        }
    }
}

impl<'a, Client: flash_hil::Client<FlashCtrl<'a>>> flash_hil::HasClient<'a, Client>
    for FlashCtrl<'a>
{
    fn set_client(&'a self, data_client: &'a Client) {
        self.data_client.set(data_client);
    }
}

impl<'a, InfoClient: flash_hil::InfoClient<FlashCtrl<'a>>> flash_hil::HasInfoClient<'a, InfoClient>
    for FlashCtrl<'a>
{
    fn set_info_client(&'a self, info_client: &'a InfoClient) {
        self.info_client.set(info_client);
    }
}

impl flash_hil::Flash for FlashCtrl<'_> {
    type Page = RawFlashCtrlPage;

    fn read_page(
        &self,
        page_number: usize,
        raw_page: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        let (data_page_index, bank) =
            match Self::convert_raw_page_number_to_data_page_index_and_bank(page_number) {
                Ok(tuple) => tuple,
                Err(()) => return Err((ErrorCode::INVAL, raw_page)),
            };

        let data_page_position = DataPagePosition::new(bank, data_page_index);

        let data_page = DataFlashCtrlPage::new(data_page_position, raw_page);

        self.internal_read_data_page(data_page)
            .map_err(|(error_code, data_page)| (error_code, data_page.to_raw_page()))
    }

    fn write_page(
        &self,
        page_number: usize,
        raw_page: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        let (data_page_index, bank) =
            match Self::convert_raw_page_number_to_data_page_index_and_bank(page_number) {
                Ok(tuple) => tuple,
                Err(()) => return Err((ErrorCode::INVAL, raw_page)),
            };

        let data_page_position = DataPagePosition::new(bank, data_page_index);

        let data_page = DataFlashCtrlPage::new(data_page_position, raw_page);

        self.internal_write_data_page(data_page)
            .map_err(|(error_code, data_page)| (error_code, data_page.to_raw_page()))
    }

    fn erase_page(&self, page_number: usize) -> Result<(), ErrorCode> {
        let (data_page_index, bank) =
            match Self::convert_raw_page_number_to_data_page_index_and_bank(page_number) {
                Ok(tuple) => tuple,
                Err(()) => return Err(ErrorCode::INVAL),
            };

        let data_page_position = DataPagePosition::new(bank, data_page_index);

        self.internal_erase_data_page(data_page_position)
    }
}

impl flash_hil::InfoFlash for FlashCtrl<'_> {
    type InfoType = InfoPartitionType;
    type BankType = Bank;
    type Page = RawFlashCtrlPage;

    fn read_info_page(
        &self,
        info_type: Self::InfoType,
        bank: Self::BankType,
        page_number: usize,
        raw_page: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        let info_page_position = match Self::new_info_page_position(info_type, bank, page_number) {
            Ok(info_page_position) => info_page_position,
            Err(()) => return Err((ErrorCode::INVAL, raw_page)),
        };
        let info_page = InfoFlashCtrlPage::new(info_page_position, raw_page);

        self.internal_read_info_page(info_page)
            .map_err(|(error_code, info_page)| (error_code, info_page.to_raw_page()))
    }

    fn write_info_page(
        &self,
        info_type: Self::InfoType,
        bank: Self::BankType,
        page_number: usize,
        raw_page: &'static mut Self::Page,
    ) -> Result<(), (ErrorCode, &'static mut Self::Page)> {
        let info_page_position = match Self::new_info_page_position(info_type, bank, page_number) {
            Ok(info_page_position) => info_page_position,
            Err(()) => return Err((ErrorCode::INVAL, raw_page)),
        };
        let info_page = InfoFlashCtrlPage::new(info_page_position, raw_page);

        self.internal_write_info_page(info_page)
            .map_err(|(error_code, info_page)| (error_code, info_page.to_raw_page()))
    }

    fn erase_info_page(
        &self,
        info_type: Self::InfoType,
        bank: Self::BankType,
        page_number: usize,
    ) -> Result<(), ErrorCode> {
        let info_page_position = match Self::new_info_page_position(info_type, bank, page_number) {
            Ok(info_page_position) => info_page_position,
            Err(()) => return Err(ErrorCode::INVAL),
        };

        self.internal_erase_info_page(info_page_position)
    }
}
