// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use super::bank::{BANK_SIZE, DATA_PAGES_PER_BANK, NUMBER_OF_BANKS};
use super::fifo_level::FifoLevel;
use super::info_partition_type::InfoPartitionType;
use super::memory_protection::{
    DataMemoryProtectionRegion, DataMemoryProtectionRegionBase, DataMemoryProtectionRegionIndex,
    DataMemoryProtectionRegionList, DefaultMemoryProtectionRegion, EraseEnabledStatus,
    HighEnduranceEnabledStatus, Info0MemoryProtectionRegionIndex, Info0MemoryProtectionRegionList,
    Info1MemoryProtectionRegionIndex, Info1MemoryProtectionRegionList,
    Info2MemoryProtectionRegionIndex, Info2MemoryProtectionRegionList, InfoMemoryProtectionRegion,
    MemoryProtectionConfiguration, MemoryProtectionRegionStatus, ReadEnabledStatus,
    WriteEnabledStatus,
};
use super::page_index::{DataPageIndex, Info0PageIndex, Info1PageIndex, Info2PageIndex};
use super::page_position::{
    DataPagePosition, Info0PagePosition, Info1PagePosition, Info2PagePosition, InfoPagePosition,
};

use crate::registers::flash_ctrl_regs::{
    FlashCtrlRegisters, ADDR, CONTROL, DEFAULT_REGION, ERR_CODE, FIFO_LVL, INFO_PAGE_CFG,
    INFO_REGWEN, INTR, MP_REGION, MP_REGION_CFG, OP_STATUS, REGION_CFG_REGWEN, STATUS,
};
use crate::registers::top_earlgrey::{FLASH_CTRL_CORE_BASE_ADDR, FLASH_CTRL_MEM_BASE_ADDR};
use crate::utils;

use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::utilities::registers::FieldValue;
use kernel::utilities::registers::ReadWrite;
use kernel::utilities::StaticRef;

use core::num::NonZeroUsize;

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

pub struct FlashCtrl {
    registers: StaticRef<FlashCtrlRegisters>,
}

impl FlashCtrl {
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
        };
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
        let high_endurance_enabled =
            match default_memory_protection_region.is_high_endurance_enabled() {
                HighEnduranceEnabledStatus::Disabled => DEFAULT_REGION::HE_EN::Clear,
                HighEnduranceEnabledStatus::Enabled => DEFAULT_REGION::HE_EN::Set,
            };

        let erase_enabled = match default_memory_protection_region.is_erase_enabled() {
            EraseEnabledStatus::Disabled => DEFAULT_REGION::ERASE_EN::Clear,
            EraseEnabledStatus::Enabled => DEFAULT_REGION::ERASE_EN::Set,
        };

        let write_enabled = match default_memory_protection_region.is_write_enabled() {
            WriteEnabledStatus::Disabled => DEFAULT_REGION::PROG_EN::Clear,
            WriteEnabledStatus::Enabled => DEFAULT_REGION::PROG_EN::Set,
        };

        let read_enabled = match default_memory_protection_region.is_read_enabled() {
            ReadEnabledStatus::Disabled => DEFAULT_REGION::RD_EN::Clear,
            ReadEnabledStatus::Enabled => DEFAULT_REGION::RD_EN::Set,
        };

        self.registers
            .default_region
            .modify(high_endurance_enabled + erase_enabled + write_enabled + read_enabled);
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
        let memory_protection_region_base = MP_REGION::BASE.val(
            Self::convert_memory_protection_region_base_to_register_value(
                memory_protection_region.get_base(),
            ),
        );
        // u32 == usize on Earlgrey
        let memory_protection_region_size =
            MP_REGION::SIZE.val(memory_protection_region.get_size().inner() as u32);

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
            HighEnduranceEnabledStatus::Disabled => MP_REGION_CFG::HE_EN::Clear,
            HighEnduranceEnabledStatus::Enabled => MP_REGION_CFG::HE_EN::Set,
        };

        let erase_enabled = match memory_protection_region.is_erase_enabled() {
            EraseEnabledStatus::Disabled => MP_REGION_CFG::ERASE_EN::Clear,
            EraseEnabledStatus::Enabled => MP_REGION_CFG::ERASE_EN::Set,
        };

        let write_enabled = match memory_protection_region.is_write_enabled() {
            WriteEnabledStatus::Disabled => MP_REGION_CFG::PROG_EN::Clear,
            WriteEnabledStatus::Enabled => MP_REGION_CFG::PROG_EN::Set,
        };

        let read_enabled = match memory_protection_region.is_read_enabled() {
            ReadEnabledStatus::Disabled => MP_REGION_CFG::RD_EN::Clear,
            ReadEnabledStatus::Enabled => MP_REGION_CFG::RD_EN::Set,
        };

        let is_region_enabled = match memory_protection_region.is_enabled() {
            MemoryProtectionRegionStatus::Disabled => MP_REGION_CFG::EN::Clear,
            MemoryProtectionRegionStatus::Enabled => MP_REGION_CFG::EN::Set,
        };

        memory_protection_region_register.modify(
            high_endurance_enabled
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
        field_value: FieldValue<u32, INFO_PAGE_CFG::Register>,
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
        field_value: FieldValue<u32, INFO_PAGE_CFG::Register>,
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
        field_value: FieldValue<u32, INFO_PAGE_CFG::Register>,
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
        field_value: FieldValue<u32, INFO_PAGE_CFG::Register>,
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
        field_value: FieldValue<u32, INFO_PAGE_CFG::Register>,
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
        field_value: FieldValue<u32, INFO_PAGE_CFG::Register>,
    ) {
        // PANIC: Info2PageIndex guarantees correct access to bank1_info2_page_cfg
        let register = self
            .registers
            .bank1_info2_page_cfg
            .get(info2_page_index.to_usize())
            .unwrap();
        register.modify(field_value);
    }

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
    fn convert_info_memory_protection_region_to_register_value(
        info_memory_protection_region: &InfoMemoryProtectionRegion,
    ) -> FieldValue<u32, INFO_PAGE_CFG::Register> {
        let high_endurance_enabled = match info_memory_protection_region.is_high_endurance_enabled()
        {
            HighEnduranceEnabledStatus::Disabled => INFO_PAGE_CFG::HE_EN::Clear,
            HighEnduranceEnabledStatus::Enabled => INFO_PAGE_CFG::HE_EN::Set,
        };

        let erase_enabled = match info_memory_protection_region.is_erase_enabled() {
            EraseEnabledStatus::Disabled => INFO_PAGE_CFG::ERASE_EN::Clear,
            EraseEnabledStatus::Enabled => INFO_PAGE_CFG::ERASE_EN::Set,
        };

        let write_enabled = match info_memory_protection_region.is_write_enabled() {
            WriteEnabledStatus::Disabled => INFO_PAGE_CFG::PROG_EN::Clear,
            WriteEnabledStatus::Enabled => INFO_PAGE_CFG::PROG_EN::Set,
        };

        let read_enabled = match info_memory_protection_region.is_read_enabled() {
            ReadEnabledStatus::Disabled => INFO_PAGE_CFG::RD_EN::Clear,
            ReadEnabledStatus::Enabled => INFO_PAGE_CFG::RD_EN::Set,
        };

        let is_region_enabled = match info_memory_protection_region.is_enabled() {
            MemoryProtectionRegionStatus::Disabled => INFO_PAGE_CFG::EN::Clear,
            MemoryProtectionRegionStatus::Enabled => INFO_PAGE_CFG::EN::Set,
        };

        high_endurance_enabled + erase_enabled + write_enabled + read_enabled + is_region_enabled
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
        let register_value = Self::convert_info_memory_protection_region_to_register_value(
            info0_memory_protection_region,
        );

        match info0_memory_protection_region_index {
            Info0MemoryProtectionRegionIndex::Bank0(info0_page_index) => self
                .configure_access_bank0_info0_memory_protection_region(
                    info0_page_index,
                    register_value,
                ),
            Info0MemoryProtectionRegionIndex::Bank1(info0_page_index) => self
                .configure_access_bank1_info0_memory_protection_region(
                    info0_page_index,
                    register_value,
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
        let register_value = Self::convert_info_memory_protection_region_to_register_value(
            info1_memory_protection_region,
        );

        match info1_memory_protection_region_index {
            Info1MemoryProtectionRegionIndex::Bank0(info1_page_index) => self
                .configure_access_bank0_info1_memory_protection_region(
                    info1_page_index,
                    register_value,
                ),
            Info1MemoryProtectionRegionIndex::Bank1(info1_page_index) => self
                .configure_access_bank1_info1_memory_protection_region(
                    info1_page_index,
                    register_value,
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
        let register_value = Self::convert_info_memory_protection_region_to_register_value(
            info2_memory_protection_region,
        );

        match info2_memory_protection_region_index {
            Info2MemoryProtectionRegionIndex::Bank0(info2_page_index) => self
                .configure_access_bank0_info2_memory_protection_region(
                    info2_page_index,
                    register_value,
                ),
            Info2MemoryProtectionRegionIndex::Bank1(info2_page_index) => self
                .configure_access_bank1_info2_memory_protection_region(
                    info2_page_index,
                    register_value,
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
            .modify(REGION_CFG_REGWEN::REGION::REGION_LOCKED);
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

        register.modify(INFO_REGWEN::REGION::PAGE_LOCKED);
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

        register.modify(INFO_REGWEN::REGION::PAGE_LOCKED);
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

        register.modify(INFO_REGWEN::REGION::PAGE_LOCKED);
    }

    fn lock_bank1_info1_memory_protection_region(&self, info1_page_index: Info1PageIndex) {
        // PANIC: Info1PageIndex guarantees correct access to bank0_info1_page_cfg
        let register = self
            .registers
            .bank1_info1_regwen
            .get(info1_page_index.to_usize())
            .unwrap();

        register.modify(INFO_REGWEN::REGION::PAGE_LOCKED);
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

        register.modify(INFO_REGWEN::REGION::PAGE_LOCKED);
    }

    fn lock_bank1_info2_memory_protection_region(&self, info2_page_index: Info2PageIndex) {
        // PANIC: Info2PageIndex guarantees correct access to bank0_info2_page_cfg
        let register = self
            .registers
            .bank1_info2_regwen
            .get(info2_page_index.to_usize())
            .unwrap();

        register.modify(INFO_REGWEN::REGION::PAGE_LOCKED);
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
}
