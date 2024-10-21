// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.

use super::flash_address::{FlashAddress, InvalidHostAddressError};
use super::flash_ctrl::{BusyStatus, FlashCtrl, FLASH_HOST_STARTING_ADDRESS_OFFSET};
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

use crate::registers::flash_ctrl_regs::{
    region_enable_magic_value, CONTROL, ERR_CODE, INFO_PAGE_CFG, INTR, MP_REGION_CFG, OP_STATUS,
    STATUS,
};
use crate::uart::Uart;

use kernel::hil::flash::Client as FlashClientTrait;
use kernel::hil::flash::Flash as FlashTrait;
use kernel::hil::flash::HasClient;
use kernel::hil::flash::HasInfoClient;
use kernel::hil::flash::InfoClient as FlashInfoClientTrait;
use kernel::hil::flash::InfoFlash as InfoFlashTrait;
use kernel::utilities::cells::{OptionalCell, TakeCell};
use kernel::utilities::registers::{interfaces::Readable, ReadWrite};
use kernel::ErrorCode;

use core::cell::Cell;
use core::fmt::Write;
use core::ops::RangeInclusive;

const WRITE_MESSAGE: &str = "Tock is an awesome operating system!";
const READ_MESSAGE: &str = "Rust is a modern, memory safe programming language used for systems programming, embedded systems, command line applications, web servers and everything you might imagine.";
const WRITE_FILL_BYTE_VALUE: u8 = 0x00;
const ERASE_BYTE_VALUE: u8 = 0xFF;
// The position of an info2 page which has read, write and erase enabled.
const VALID_INFO2_PAGE_POSITION: Info2PagePosition =
    Info2PagePosition::new(Bank::Bank1, Info2PageIndex::Index1);
pub const VALID_INFO2_MEMORY_PROTECTION_REGION_INDEX: Info2MemoryProtectionRegionIndex =
    VALID_INFO2_PAGE_POSITION;
// The position of an info page which has read, write and erase enabled.
const VALID_INFO_PAGE_POSITION: InfoPagePosition =
    InfoPagePosition::Type2(VALID_INFO2_PAGE_POSITION);
// The position of an info page which has read, write and erase disabled.
const INVALID_INFO_PAGE_POSITION: InfoPagePosition =
    InfoPagePosition::Type2(Info2PagePosition::new(Bank::Bank1, Info2PageIndex::Index0));

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

macro_rules! print_test_info {
    ($msg:expr) => ({
        println!("INFO: {}", $msg);
    });
    ($fmt:expr, $($arg:tt)+) => ({
        println!("INFO: {}", format_args!($fmt, $($arg)+));
    });
}

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

type FlashPage<'a> = <FlashCtrl<'a> as FlashTrait>::Page;

#[derive(Clone, Copy, Debug)]
enum TestState {
    InitialState,
    TestDataErase,
    TestDataWrite,
    TestDataRead,
    TestDataEraseFault,
    TestDataWriteFault,
    TestDataReadFault,
    TestDataEraseBusy,
    TestDataWriteBusy,
    TestDataReadBusy,
    TestInfoErase,
    TestInfoWrite,
    // There is no TestInfoRead because TestInfoErase and TestInfoWrite already use page reading
    // since it is impossible to read a flash page directly from the host system.
    TestInfoEraseFault,
    TestInfoWriteFault,
    TestInfoReadFault,
    TestInfoEraseBusy,
    TestInfoWriteBusy,
    TestInfoReadBusy,
}

fn check_page_content(page_content: &[u8; EARLGREY_PAGE_SIZE.get()], message: &str) {
    let message_length = message.len();
    // split_at() can panic only for large messages, which is not the case for these test cases
    let (page_content_message_slice, page_content_fill_slice) =
        page_content.split_at(message_length);
    assert!(
        page_content_message_slice == message.as_bytes(),
        "Expected message {:?}, found {:?}",
        message.as_bytes(),
        page_content_message_slice
    );

    assert!(
        is_slice_filled_with(page_content_fill_slice, WRITE_FILL_BYTE_VALUE),
        "Page read should read all bytes past the message as {:#x}",
        ERASE_BYTE_VALUE
    );
}

// SAFETY: The caller must ensure that a previous mutable reference pointing to the same host page
// does not already exist. This function returns the same mutable reference only for the same data
// page position
unsafe fn convert_data_page_position_to_host_array<'a>(
    data_page_position: DataPagePosition,
) -> &'a mut [u8; EARLGREY_PAGE_SIZE.get()] {
    let page_number = convert_data_page_position_to_page_number(data_page_position);
    let host_ptr = (FLASH_HOST_STARTING_ADDRESS_OFFSET.get()
        + page_number * EARLGREY_PAGE_SIZE.get()) as *mut u8;

    &mut *host_ptr.cast::<[u8; EARLGREY_PAGE_SIZE.get()]>()
}

fn decompose_info_page_position(
    info_page_position: InfoPagePosition,
) -> (InfoPartitionType, Bank, usize) {
    match info_page_position {
        InfoPagePosition::Type0(info0_page_position) => match info0_page_position {
            Info0PagePosition::Bank0(page_index) => {
                (InfoPartitionType::Type0, Bank::Bank0, page_index.to_usize())
            }
            Info0PagePosition::Bank1(page_index) => {
                (InfoPartitionType::Type0, Bank::Bank1, page_index.to_usize())
            }
        },
        InfoPagePosition::Type1(info1_page_position) => match info1_page_position {
            Info1PagePosition::Bank0(page_index) => {
                (InfoPartitionType::Type1, Bank::Bank0, page_index.to_usize())
            }
            Info1PagePosition::Bank1(page_index) => {
                (InfoPartitionType::Type1, Bank::Bank1, page_index.to_usize())
            }
        },
        InfoPagePosition::Type2(info2_page_position) => match info2_page_position {
            Info2PagePosition::Bank0(page_index) => {
                (InfoPartitionType::Type2, Bank::Bank0, page_index.to_usize())
            }
            Info2PagePosition::Bank1(page_index) => {
                (InfoPartitionType::Type2, Bank::Bank1, page_index.to_usize())
            }
        },
    }
}

fn copy_message_and_fill_to_page<'a>(page: &'a mut FlashPage<'a>, message: &str) {
    // copy_from_slice cannot panic since the subslice has the length message.len()
    let page_array: &mut [u8; EARLGREY_PAGE_SIZE.get()] = page.as_mut();
    page_array[..message.len()].copy_from_slice(message.as_bytes());
    page_array[message.len()..].fill(WRITE_FILL_BYTE_VALUE);
}

#[derive(Clone, Copy, Debug)]
enum InfoMemoryProtectionRegionIndex {
    Type0(Info0MemoryProtectionRegionIndex),
    Type1(Info1MemoryProtectionRegionIndex),
    Type2(Info2MemoryProtectionRegionIndex),
}

impl FlashCtrl<'_> {
    fn get_info0_memory_protection_region_register(
        &self,
        info0_memory_protection_region_index: Info0MemoryProtectionRegionIndex,
    ) -> &ReadWrite<u32, INFO_PAGE_CFG::Register> {
        let registers = self.get_registers();
        match info0_memory_protection_region_index {
            Info0MemoryProtectionRegionIndex::Bank0(info0_page_index) =>
            // PANIC: Info0PageIndex guarantees safe access to bank0_info0_page_cfg
            {
                registers
                    .bank0_info0_page_cfg
                    .get(info0_page_index.to_usize())
                    .unwrap()
            }
            Info0MemoryProtectionRegionIndex::Bank1(info0_page_index) =>
            // PANIC: Info0PageIndex guarantees safe access to bank1_info0_page_cfg
            {
                registers
                    .bank1_info0_page_cfg
                    .get(info0_page_index.to_usize())
                    .unwrap()
            }
        }
    }

    fn get_info1_memory_protection_region_register(
        &self,
        info1_memory_protection_region_index: Info1MemoryProtectionRegionIndex,
    ) -> &ReadWrite<u32, INFO_PAGE_CFG::Register> {
        let registers = self.get_registers();
        match info1_memory_protection_region_index {
            Info1MemoryProtectionRegionIndex::Bank0(info1_page_index) =>
            // PANIC: Info1PageIndex guarantees safe access to bank0_info1_page_cfg
            {
                registers
                    .bank0_info1_page_cfg
                    .get(info1_page_index.to_usize())
                    .unwrap()
            }
            Info1MemoryProtectionRegionIndex::Bank1(info1_page_index) =>
            // PANIC: Info1PageIndex guarantees safe access to bank1_info1_page_cfg
            {
                registers
                    .bank1_info1_page_cfg
                    .get(info1_page_index.to_usize())
                    .unwrap()
            }
        }
    }

    fn get_info2_memory_protection_region_register(
        &self,
        info2_memory_protection_region_index: Info2MemoryProtectionRegionIndex,
    ) -> &ReadWrite<u32, INFO_PAGE_CFG::Register> {
        let registers = self.get_registers();
        match info2_memory_protection_region_index {
            Info2MemoryProtectionRegionIndex::Bank0(info2_page_index) =>
            // PANIC: Info2PageIndex guarantees safe access to bank0_info2_page_cfg
            {
                registers
                    .bank0_info2_page_cfg
                    .get(info2_page_index.to_usize())
                    .unwrap()
            }
            Info2MemoryProtectionRegionIndex::Bank1(info2_page_index) =>
            // PANIC: Info2PageIndex guarantees safe access to bank1_info2_page_cfg
            {
                registers
                    .bank1_info2_page_cfg
                    .get(info2_page_index.to_usize())
                    .unwrap()
            }
        }
    }

    fn get_info_memory_protection_region_register(
        &self,
        info_memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> &ReadWrite<u32, INFO_PAGE_CFG::Register> {
        match info_memory_protection_region_index {
            InfoMemoryProtectionRegionIndex::Type0(info0_memory_protection_region_index) => self
                .get_info0_memory_protection_region_register(info0_memory_protection_region_index),
            InfoMemoryProtectionRegionIndex::Type1(info1_memory_protection_region_index) => self
                .get_info1_memory_protection_region_register(info1_memory_protection_region_index),
            InfoMemoryProtectionRegionIndex::Type2(info2_memory_protection_region_index) => self
                .get_info2_memory_protection_region_register(info2_memory_protection_region_index),
        }
    }

    fn configure_info_memory_protection_region(
        &self,
        info_memory_protection_region_index: InfoMemoryProtectionRegionIndex,
        info_memory_protection_region: &InfoMemoryProtectionRegion,
    ) {
        match info_memory_protection_region_index {
            InfoMemoryProtectionRegionIndex::Type0(info0_memory_protection_region_index) => self
                .configure_info0_memory_protection_region(
                    info0_memory_protection_region_index,
                    info_memory_protection_region,
                ),
            InfoMemoryProtectionRegionIndex::Type1(info1_memory_protection_region_index) => self
                .configure_info1_memory_protection_region(
                    info1_memory_protection_region_index,
                    info_memory_protection_region,
                ),
            InfoMemoryProtectionRegionIndex::Type2(info2_memory_protection_region_index) => self
                .configure_info2_memory_protection_region(
                    info2_memory_protection_region_index,
                    info_memory_protection_region,
                ),
        }
    }

    /* CONTROL */
    fn is_control_start_set(&self) -> bool {
        self.get_registers().control.is_set(CONTROL::START)
    }

    /* MEMORY PROTECTION */
    fn is_data_region_read_enabled(
        &self,
        memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> bool {
        let registers = self.get_registers();
        // PANIC: DataMemoryProtectionRegionIndex is a type that guarantees safe accesses to all
        // memory protection region arrays
        let memory_protection_region_register = registers
            .mp_region_cfg
            .get(memory_protection_region_index.inner())
            .unwrap();
        memory_protection_region_register.read(MP_REGION_CFG::RD_EN) == region_enable_magic_value!()
    }

    fn is_data_region_write_enabled(
        &self,
        memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> bool {
        let registers = self.get_registers();
        // PANIC: DataMemoryProtectionRegionIndex is a type that guarantees safe accesses to all
        // memory protection region arrays
        let memory_protection_region_register = registers
            .mp_region_cfg
            .get(memory_protection_region_index.inner())
            .unwrap();
        memory_protection_region_register.read(MP_REGION_CFG::PROG_EN)
            == region_enable_magic_value!()
    }

    fn is_data_region_erase_enabled(
        &self,
        memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> bool {
        let registers = self.get_registers();
        // PANIC: DataMemoryProtectionRegionIndex is a type that guarantees safe accesses to all
        // memory protection region arrays
        let memory_protection_region_register = registers
            .mp_region_cfg
            .get(memory_protection_region_index.inner())
            .unwrap();
        memory_protection_region_register.read(MP_REGION_CFG::ERASE_EN)
            == region_enable_magic_value!()
    }

    fn is_data_region_scramble_enabled(
        &self,
        memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> bool {
        let registers = self.get_registers();
        // SAFETY: DataMemoryProtectionRegionIndex is a type that guarantees safe accesses to all
        // memory protection region arrays
        let memory_protection_region_register = registers
            .mp_region_cfg
            .get(memory_protection_region_index.inner())
            .unwrap();
        memory_protection_region_register.read(MP_REGION_CFG::SCRAMBLE_EN)
            == region_enable_magic_value!()
    }

    fn is_data_region_ecc_enabled(
        &self,
        memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> bool {
        let registers = self.get_registers();
        // SAFETY: DataMemoryProtectionRegionIndex is a type that guarantees safe accesses to all
        // memory protection region arrays
        let memory_protection_region_register = registers
            .mp_region_cfg
            .get(memory_protection_region_index.inner())
            .unwrap();
        memory_protection_region_register.read(MP_REGION_CFG::ECC_EN)
            == region_enable_magic_value!()
    }

    fn is_data_region_high_endurance_enabled(
        &self,
        memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> bool {
        let registers = self.get_registers();
        // SAFETY: DataMemoryProtectionRegionIndex is a type that guarantees safe accesses to all
        // memory protection region arrays
        let memory_protection_region_register = registers
            .mp_region_cfg
            .get(memory_protection_region_index.inner())
            .unwrap();
        memory_protection_region_register.read(MP_REGION_CFG::HE_EN) == region_enable_magic_value!()
    }

    fn is_info_region_read_enabled(
        &self,
        memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> bool {
        let info_memory_protection_region_register =
            self.get_info_memory_protection_region_register(memory_protection_region_index);
        info_memory_protection_region_register.read(INFO_PAGE_CFG::RD_EN)
            == region_enable_magic_value!()
    }

    fn is_info_region_write_enabled(
        &self,
        memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> bool {
        let info_memory_protection_region_register =
            self.get_info_memory_protection_region_register(memory_protection_region_index);
        info_memory_protection_region_register.read(INFO_PAGE_CFG::PROG_EN)
            == region_enable_magic_value!()
    }

    fn is_info_region_erase_enabled(
        &self,
        memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> bool {
        let info_memory_protection_region_register =
            self.get_info_memory_protection_region_register(memory_protection_region_index);
        info_memory_protection_region_register.read(INFO_PAGE_CFG::ERASE_EN)
            == region_enable_magic_value!()
    }

    fn _is_info_region_scramble_enabled(
        &self,
        memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> bool {
        let info_memory_protection_region_register =
            self.get_info_memory_protection_region_register(memory_protection_region_index);
        info_memory_protection_region_register.read(INFO_PAGE_CFG::SCRAMBLE_EN)
            == region_enable_magic_value!()
    }

    fn _is_info_region_ecc_enabled(
        &self,
        memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> bool {
        let info_memory_protection_region_register =
            self.get_info_memory_protection_region_register(memory_protection_region_index);
        info_memory_protection_region_register.read(INFO_PAGE_CFG::ECC_EN)
            == region_enable_magic_value!()
    }

    fn is_info_region_high_endurance_enabled(
        &self,
        memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> bool {
        let info_memory_protection_region_register =
            self.get_info_memory_protection_region_register(memory_protection_region_index);
        info_memory_protection_region_register.read(INFO_PAGE_CFG::HE_EN)
            == region_enable_magic_value!()
    }

    /* OP_STATUS */
    fn is_op_status_done_set(&self) -> bool {
        self.get_registers().op_status.is_set(OP_STATUS::DONE)
    }

    fn is_op_status_err_set(&self) -> bool {
        self.get_registers().op_status.is_set(OP_STATUS::ERR)
    }

    /* ERR_CODE */
    fn is_op_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::OP_ERR)
    }

    fn is_mp_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::MP_ERR)
    }

    fn is_rd_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::RD_ERR)
    }

    fn is_prog_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::PROG_ERR)
    }

    fn is_prog_win_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::PROG_WIN_ERR)
    }

    fn is_prog_type_err_set(&self) -> bool {
        self.get_registers()
            .err_code
            .is_set(ERR_CODE::PROG_TYPE_ERR)
    }

    fn is_update_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::UPDATE_ERR)
    }

    fn is_macro_err_set(&self) -> bool {
        self.get_registers().err_code.is_set(ERR_CODE::MACRO_ERR)
    }

    /* STATUS */
    fn is_status_rd_full_set(&self) -> bool {
        self.get_registers().status.is_set(STATUS::RD_FULL)
    }

    fn is_status_prog_full_set(&self) -> bool {
        self.get_registers().status.is_set(STATUS::PROG_FULL)
    }

    fn is_status_prog_empty_set(&self) -> bool {
        self.get_registers().status.is_set(STATUS::PROG_EMPTY)
    }

    fn is_status_init_wip_set(&self) -> bool {
        self.get_registers().status.is_set(STATUS::INIT_WIP)
    }

    fn is_status_initialized_set(&self) -> bool {
        self.get_registers().status.is_set(STATUS::INITIALIZED)
    }

    /* INTR_STATE */
    fn is_interrupt_corr_err_set(&self) -> bool {
        self.get_registers().intr_state.is_set(INTR::CORR_ERR)
    }

    fn is_interrupt_op_done_set(&self) -> bool {
        self.get_registers().intr_state.is_set(INTR::OP_DONE)
    }

    fn is_interrupt_rd_lvl_set(&self) -> bool {
        self.get_registers().intr_state.is_set(INTR::RD_LVL)
    }

    fn is_interrupt_rd_full_set(&self) -> bool {
        self.get_registers().intr_state.is_set(INTR::RD_FULL)
    }

    fn is_interrupt_prog_lvl_set(&self) -> bool {
        self.get_registers().intr_state.is_set(INTR::PROG_LVL)
    }

    fn is_interrupt_prog_empty_set(&self) -> bool {
        self.get_registers().intr_state.is_set(INTR::PROG_EMPTY)
    }

    /* INTR_ENABLE */
    fn is_interrupt_corr_err_enabled(&self) -> bool {
        self.get_registers().intr_enable.is_set(INTR::CORR_ERR)
    }

    fn is_interrupt_op_done_enabled(&self) -> bool {
        self.get_registers().intr_enable.is_set(INTR::OP_DONE)
    }

    fn is_interrupt_rd_lvl_enabled(&self) -> bool {
        self.get_registers().intr_enable.is_set(INTR::RD_LVL)
    }

    fn is_interrupt_rd_full_enabled(&self) -> bool {
        self.get_registers().intr_enable.is_set(INTR::RD_FULL)
    }

    fn is_interrupt_prog_lvl_enabled(&self) -> bool {
        self.get_registers().intr_enable.is_set(INTR::PROG_LVL)
    }

    fn is_interrupt_prog_empty_enabled(&self) -> bool {
        self.get_registers().intr_enable.is_set(INTR::PROG_EMPTY)
    }
}

fn convert_data_page_position_to_page_number(data_page_position: DataPagePosition) -> usize {
    match data_page_position {
        DataPagePosition::Bank0(page_index) => page_index.to_usize(),
        // Bank1 starts one past MAX_DATA_PAGE_INDEX
        DataPagePosition::Bank1(page_index) => {
            MAX_DATA_PAGE_INDEX.get() as usize + 1 + page_index.to_usize()
        }
    }
}

pub struct TestClient<'a> {
    flash: &'a FlashCtrl<'a>,
    page: TakeCell<'static, FlashPage<'a>>,
    placeholder_page: TakeCell<'static, FlashPage<'a>>,
    state: Cell<TestState>,
    page_position_range: RangeInclusive<DataPagePosition>,
    current_data_test_page_position: Cell<DataPagePosition>,
    current_info_test_page_position: Cell<InfoPagePosition>,
    current_test_message: OptionalCell<&'a str>,
}

impl<'a> TestClient<'a> {
    pub fn new(
        flash: &'a FlashCtrl<'a>,
        flash_page: &'static mut FlashPage<'a>,
        placeholder_flash_page: &'static mut FlashPage<'a>,
        page_position_range: RangeInclusive<DataPagePosition>,
    ) -> Self {
        Self {
            flash: flash,
            page: TakeCell::new(flash_page),
            placeholder_page: TakeCell::new(placeholder_flash_page),
            state: Cell::new(TestState::InitialState),
            page_position_range,
            current_data_test_page_position: Cell::new(DataPagePosition::Bank0(
                DataPageIndex::new(0),
            )),
            current_info_test_page_position: Cell::new(InfoPagePosition::Type0(
                Info0PagePosition::new(Bank::Bank0, Info0PageIndex::Index0),
            )),
            current_test_message: OptionalCell::empty(),
        }
    }
}

type FlashReturnType<'a> = Result<(), (ErrorCode, &'static mut FlashPage<'a>)>;

impl<'a> TestClient<'a> {
    fn print_info_result_read_write(result: &FlashReturnType<'a>) {
        match result {
            Ok(()) => print_test_info!("Peripheral returned OK"),
            Err((error_code, _page)) => print_test_info!("Peripheral returned {:?}", error_code),
        }
    }

    fn print_info_result_erase(result: &Result<(), ErrorCode>) {
        match result {
            Ok(()) => print_test_info!("Peripheral returned OK"),
            Err(error_code) => print_test_info!("Peripheral returned {:?}", error_code),
        }
    }

    fn raw_read_page(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'a>,
    ) -> FlashReturnType {
        print_test_info!("Attempting to read page number {}", page_number);
        let result = self.flash.read_page(page_number, page);
        Self::print_info_result_read_write(&result);

        result
    }

    fn raw_write_page(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'a>,
    ) -> FlashReturnType {
        print_test_info!("Attempting to write page number {}", page_number);
        let result = self.flash.write_page(page_number, page);
        Self::print_info_result_read_write(&result);

        result
    }

    fn raw_erase_data_page(&self, page_number: usize) -> Result<(), ErrorCode> {
        print_test_info!("Attempting to erase page number {}", page_number);
        let result = self.flash.erase_page(page_number);
        Self::print_info_result_erase(&result);

        result
    }

    fn raw_read_info_page(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        page: &'static mut FlashPage<'a>,
    ) -> Result<(), (ErrorCode, &'static mut FlashPage<'a>)> {
        print_test_info!(
            "Attempting to read info page: {:?}, {:?}, page number {}",
            info_partition_type,
            bank,
            page_number
        );
        let result = self
            .flash
            .read_info_page(info_partition_type, bank, page_number, page);
        Self::print_info_result_read_write(&result);

        result
    }

    fn read_info_page(
        &self,
        info_page_position: InfoPagePosition,
        page: &'static mut FlashPage<'a>,
    ) {
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        let result = self.raw_read_info_page(info_partition_type, bank, page_number, page);

        if let Err((error_code, _page)) = result {
            panic!(
                "Reading page {}, {:?}, {:?} must succeed, but instead failed with error {:?}",
                page_number, bank, info_partition_type, error_code
            )
        }
    }

    fn raw_write_info_page(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        page: &'static mut FlashPage<'a>,
    ) -> Result<(), (ErrorCode, &'static mut FlashPage<'a>)> {
        print_test_info!(
            "Attempting to write info page: {:?}, {:?}, page number {}",
            info_partition_type,
            bank,
            page_number,
        );

        let result = self
            .flash
            .write_info_page(info_partition_type, bank, page_number, page);
        Self::print_info_result_read_write(&result);

        result
    }

    fn raw_erase_info_page(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
    ) -> Result<(), ErrorCode> {
        print_test_info!(
            "Attempting to erase info page: {:?}, {:?}, page number {}",
            info_partition_type,
            bank,
            page_number,
        );
        let result = self
            .flash
            .erase_info_page(info_partition_type, bank, page_number);
        Self::print_info_result_erase(&result);

        result
    }

    fn get_configured_data_memory_protection_region(
        &self,
        data_memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) -> DataMemoryProtectionRegion {
        // TODO: Add base and size
        let mut data_memory_protection_region = DataMemoryProtectionRegion::new();

        if self
            .flash
            .is_data_region_read_enabled(data_memory_protection_region_index)
        {
            data_memory_protection_region.enable_read();
        }

        if self
            .flash
            .is_data_region_write_enabled(data_memory_protection_region_index)
        {
            data_memory_protection_region.enable_write();
        }

        if self
            .flash
            .is_data_region_erase_enabled(data_memory_protection_region_index)
        {
            data_memory_protection_region.enable_erase();
        }

        if self
            .flash
            .is_data_region_high_endurance_enabled(data_memory_protection_region_index)
        {
            data_memory_protection_region.enable_high_endurance();
        }

        data_memory_protection_region
    }

    fn get_data_memory_protection_region_complement(
        &self,
        data_memory_protection_region: &DataMemoryProtectionRegion,
    ) -> DataMemoryProtectionRegion {
        let mut data_memory_protection_region_complement = DataMemoryProtectionRegion::new();

        if ReadEnabledStatus::Disabled == data_memory_protection_region.is_read_enabled() {
            data_memory_protection_region_complement.enable_read();
        }

        if WriteEnabledStatus::Disabled == data_memory_protection_region.is_write_enabled() {
            data_memory_protection_region_complement.enable_write();
        }

        if EraseEnabledStatus::Disabled == data_memory_protection_region.is_erase_enabled() {
            data_memory_protection_region_complement.enable_erase();
        }

        if HighEnduranceEnabledStatus::Disabled
            == data_memory_protection_region.is_high_endurance_enabled()
        {
            data_memory_protection_region_complement.enable_high_endurance();
        }

        data_memory_protection_region_complement
    }

    fn test_data_memory_protection_lock(
        &self,
        data_memory_protection_region_index: DataMemoryProtectionRegionIndex,
    ) {
        // Read the existing configuration
        let data_memory_protection_region =
            self.get_configured_data_memory_protection_region(data_memory_protection_region_index);
        // Get the complement of the existing configuration
        let data_memory_protection_region_complement =
            self.get_data_memory_protection_region_complement(&data_memory_protection_region);

        // Try to configure the data memory protection region
        self.flash.configure_data_memory_protection_region(
            data_memory_protection_region_index,
            &data_memory_protection_region_complement,
        );

        let new_data_memory_protection_region =
            self.get_configured_data_memory_protection_region(data_memory_protection_region_index);

        assert_eq!(
            data_memory_protection_region,
            new_data_memory_protection_region,
            "The data memory protection configuration for region {:?} must not be mutable after memory protection locking.",
            data_memory_protection_region_index
        );
    }

    fn get_configured_info_memory_protection_region(
        &self,
        info_memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) -> InfoMemoryProtectionRegion {
        let mut info_memory_protection_region = InfoMemoryProtectionRegion::new();

        if self
            .flash
            .is_info_region_read_enabled(info_memory_protection_region_index)
        {
            info_memory_protection_region.enable_read();
        }

        if self
            .flash
            .is_info_region_write_enabled(info_memory_protection_region_index)
        {
            info_memory_protection_region.enable_write();
        }

        if self
            .flash
            .is_info_region_erase_enabled(info_memory_protection_region_index)
        {
            info_memory_protection_region.enable_erase();
        }

        if self
            .flash
            .is_info_region_high_endurance_enabled(info_memory_protection_region_index)
        {
            info_memory_protection_region.enable_high_endurance();
        }

        info_memory_protection_region
    }

    fn get_info_memory_protection_region_complement(
        &self,
        info_memory_protection_region: &InfoMemoryProtectionRegion,
    ) -> InfoMemoryProtectionRegion {
        let mut info_memory_protection_region_complement = InfoMemoryProtectionRegion::new();

        if ReadEnabledStatus::Disabled == info_memory_protection_region.is_read_enabled() {
            info_memory_protection_region_complement.enable_read();
        }

        if WriteEnabledStatus::Disabled == info_memory_protection_region.is_write_enabled() {
            info_memory_protection_region_complement.enable_write();
        }

        if EraseEnabledStatus::Disabled == info_memory_protection_region.is_erase_enabled() {
            info_memory_protection_region_complement.enable_erase();
        }

        if HighEnduranceEnabledStatus::Disabled
            == info_memory_protection_region.is_high_endurance_enabled()
        {
            info_memory_protection_region_complement.enable_high_endurance();
        }

        info_memory_protection_region_complement
    }

    fn test_info_memory_protection_lock(
        &self,
        info_memory_protection_region_index: InfoMemoryProtectionRegionIndex,
    ) {
        // Read the existing configuration
        let info_memory_protection_region =
            self.get_configured_info_memory_protection_region(info_memory_protection_region_index);
        // Get the complement of the existing configuration
        let info_memory_protection_region_complement =
            self.get_info_memory_protection_region_complement(&info_memory_protection_region);

        // Try to configure the data memory protection region
        self.flash.configure_info_memory_protection_region(
            info_memory_protection_region_index,
            &info_memory_protection_region_complement,
        );

        let new_info_memory_protection_region =
            self.get_configured_info_memory_protection_region(info_memory_protection_region_index);

        assert_eq!(
            info_memory_protection_region,
            new_info_memory_protection_region,
            "The data memory protection configuration for region {:?} must not be mutable after memory protection locking.",
            info_memory_protection_region_index
        );
    }

    fn test_invalid_read(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
        expected_error_code: ErrorCode,
    ) -> &'static mut FlashPage<'static> {
        let result = self.raw_read_page(page_number, page);

        let (error_code, page) = match result {
            Ok(()) => panic!(
                "Attempting to read the page number {:?} must fail",
                page_number
            ),
            Err(result) => result,
        };

        assert_eq!(
            expected_error_code, error_code,
            "flash.read_page() must return {:?} as error code",
            expected_error_code,
        );

        page
    }

    fn test_invalid_read_page_number(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        self.test_invalid_read(page_number, page, ErrorCode::INVAL)
    }

    fn test_invalid_read_busy(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        self.test_invalid_read(page_number, page, ErrorCode::BUSY)
    }

    fn test_invalid_info_read(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
        expected_error_code: ErrorCode,
    ) -> &'static mut FlashPage<'static> {
        let result = self.raw_read_info_page(info_partition_type, bank, page_number, page);

        let (actual_error_code, page) = match result {
            Ok(()) => panic!(
                "Reading info page {:?}, {:?}, page number {:?} must fail with error {:?}, but instead succeeded",
                info_partition_type, bank, page_number, expected_error_code,
            ),
            Err((error_code, page)) => (error_code, page),
        };

        assert_eq!(
            actual_error_code,
            expected_error_code,
            "Reading info page {:?}, {:?}, page number {:?} must fail with error {:?}, but instead failed with error {:?}",
            info_partition_type, bank, page_number, expected_error_code, actual_error_code,
        );

        page
    }

    fn test_invalid_info_read_page_number(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        self.test_invalid_info_read(
            info_partition_type,
            bank,
            page_number,
            page,
            ErrorCode::INVAL,
        )
    }

    fn test_invalid_info_read_busy(
        &self,
        info_page_position: InfoPagePosition,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        const EXPECTED_ERROR: ErrorCode = ErrorCode::BUSY;
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        self.test_invalid_info_read(info_partition_type, bank, page_number, page, EXPECTED_ERROR)
    }

    fn test_invalid_write(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
        expected_error_code: ErrorCode,
    ) -> &'static mut FlashPage<'static> {
        let result = self.raw_write_page(page_number, page);

        let (error_code, page) = match result {
            Ok(()) => panic!(
                "Attempting to write the page number {:?} must fail",
                page_number
            ),
            Err(result) => result,
        };

        assert_eq!(
            expected_error_code, error_code,
            "flash.write_page() must return {:?} as error code",
            expected_error_code,
        );

        page
    }

    fn test_invalid_write_page_number(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        self.test_invalid_write(page_number, page, ErrorCode::INVAL)
    }

    fn test_invalid_write_busy(
        &self,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        self.test_invalid_write(page_number, page, ErrorCode::BUSY)
    }

    fn test_invalid_info_write(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
        expected_error_code: ErrorCode,
    ) -> &'static mut FlashPage<'static> {
        let result = self.raw_write_info_page(info_partition_type, bank, page_number, page);

        let (actual_error_code, page) = match result {
            Ok(()) => panic!(
                "Writing info page {:?}, {:?}, page number {:?} must fail with error {:?}, but instead succeeded",
                info_partition_type, bank, page_number, expected_error_code,
            ),
            Err((error_code, page)) => (error_code, page),
        };

        assert_eq!(
            actual_error_code,
            expected_error_code,
            "Writing info page {:?}, {:?}, page number {:?} must fail with error {:?}, but instead failed with error {:?}",
            info_partition_type, bank, page_number, expected_error_code, actual_error_code,
        );

        page
    }

    fn test_invalid_info_write_page_number(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        self.test_invalid_info_write(
            info_partition_type,
            bank,
            page_number,
            page,
            ErrorCode::INVAL,
        )
    }

    fn test_invalid_info_write_busy(
        &self,
        info_page_position: InfoPagePosition,
        page: &'static mut FlashPage<'static>,
    ) -> &'static mut FlashPage<'static> {
        const EXPECTED_ERROR: ErrorCode = ErrorCode::BUSY;
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        self.test_invalid_info_write(info_partition_type, bank, page_number, page, EXPECTED_ERROR)
    }

    fn test_invalid_erase(&self, page_number: usize, expected_error_code: ErrorCode) {
        let result = self.raw_erase_data_page(page_number);

        let error_code = match result {
            Ok(()) => panic!(
                "Attempting to erase the page number {:?} must fail",
                page_number
            ),
            Err(error_code) => error_code,
        };

        assert_eq!(
            expected_error_code, error_code,
            "flash.erase_page() must return {:?} as error code",
            expected_error_code
        );
    }

    fn test_invalid_info_erase(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
        expected_error_code: ErrorCode,
    ) {
        let result = self.raw_erase_info_page(info_partition_type, bank, page_number);

        let actual_error_code = match result {
            Ok(()) => panic!(
                "Attempting to erase info page {:?}, {:?}, page number {:?} must fail with error code {:?}, but instead succeeded",
                info_partition_type, bank, page_number, expected_error_code,
            ),
            Err(error_code) => error_code,
        };

        assert_eq!(
            expected_error_code, actual_error_code,
            "Attempting to erase info page {:?}, {:?}, page number {:?} must fail with error code {:?}, but instead failed with error {:?}",
            info_partition_type, bank, page_number, expected_error_code, actual_error_code,
        );
    }

    fn test_invalid_info_erase_page_number(
        &self,
        info_partition_type: InfoPartitionType,
        bank: Bank,
        page_number: usize,
    ) {
        self.test_invalid_info_erase(info_partition_type, bank, page_number, ErrorCode::INVAL);
    }

    fn test_invalid_erase_page_number(&self, page_number: usize) {
        self.test_invalid_erase(page_number, ErrorCode::INVAL);
    }

    fn test_invalid_erase_busy(&self, page_position: DataPagePosition) {
        let page_number = convert_data_page_position_to_page_number(page_position);

        self.test_invalid_erase(page_number, ErrorCode::BUSY);
    }

    fn test_invalid_info_erase_busy(&self, info_page_position: InfoPagePosition) {
        const EXPECTED_ERROR: ErrorCode = ErrorCode::BUSY;
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        self.test_invalid_info_erase(info_partition_type, bank, page_number, EXPECTED_ERROR);
    }

    fn set_current_data_test_page_position(&self, page_position: DataPagePosition) {
        self.current_data_test_page_position.set(page_position);
    }

    fn get_current_data_test_page_position(&self) -> DataPagePosition {
        self.current_data_test_page_position.get()
    }

    fn set_current_info_test_page_position(&self, page_position: InfoPagePosition) {
        self.current_info_test_page_position.set(page_position);
    }

    fn get_current_info_test_page_position(&self) -> InfoPagePosition {
        self.current_info_test_page_position.get()
    }

    fn write_message(&self, page_position: DataPagePosition, message: &'a str) {
        print_test_info!(
            "writing page with index {:?} and message \"{}\"",
            page_position,
            message
        );

        self.set_current_test_message(message);
        let page_number = convert_data_page_position_to_page_number(page_position);
        let page = self.take_page();
        copy_message_and_fill_to_page(page, message);
        let result = self.raw_write_page(page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Writing page number {} must succeed, but instead failed with error {:?}",
                page_number, error_code
            );
        }
    }

    fn write_message_info_page(&self, info_page_position: InfoPagePosition, message: &'a str) {
        print_test_info!(
            "writing info page {:?} and message \"{}\"",
            info_page_position,
            message
        );

        let page = self.take_page();
        copy_message_and_fill_to_page(page, message);
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        let result = self.raw_write_info_page(info_partition_type, bank, page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Writing info page {:?} must succeed, but instead failed with error {:?}",
                info_page_position, error_code
            );
        }
    }

    fn set_current_test_message(&self, message: &'a str) {
        self.current_test_message.set(message);
    }

    fn take_current_test_message(&self) -> &'a str {
        self.current_test_message.take().unwrap()
    }

    fn read_data_message(&self, page_position: DataPagePosition, message: &'a str) {
        self.set_current_test_message(message);
        let page_number = convert_data_page_position_to_page_number(page_position);
        let page = self.take_page();
        let result = self.raw_read_page(page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Reading page number {} must succeed, but instead failed with error {:?}",
                page_number, error_code
            );
        }
    }

    fn read_info_message(&self, info_page_position: InfoPagePosition, message: &'a str) {
        self.set_current_test_message(message);
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        let page = self.take_page();
        let result = self.raw_read_info_page(info_partition_type, bank, page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Reading page {:?}, {:?}, {:?} must succeed, but instead failed with error {:?}",
                page_number, bank, info_partition_type, error_code,
            );
        }
    }

    fn test_erase_info_page(&self, page_position: InfoPagePosition) {
        let (info_partition_type, bank, page_number) = decompose_info_page_position(page_position);
        self.set_current_info_test_page_position(page_position);
        let result = self.raw_erase_info_page(info_partition_type, bank, page_number);
        if let Err(error_code) = result {
            panic!(
                "Erasing page {:?} must succeed, but instead failed with error {:?}",
                page_position, error_code
            );
        }
    }

    fn test_write_page(&self, page_position: DataPagePosition) {
        self.write_message(page_position, WRITE_MESSAGE);
    }

    fn test_write_info_page(&self, page_position: InfoPagePosition) {
        self.write_message_info_page(page_position, WRITE_MESSAGE);
    }

    fn test_read_page(&self, page_position: DataPagePosition) {
        // First, a page needs to be erased before writing to it
        self.test_erase_data_page(page_position);
    }

    fn test_info_invalid_arguments(&self) {
        print_test_header("info page read with invalid page number");
        let info_partition_type = InfoPartitionType::Type0;
        let bank = Bank::Bank0;
        let invalid_page_number = Info0PageIndex::Index9 as usize + 1;
        let mut page = self.take_page();
        page = self.test_invalid_info_read_page_number(
            info_partition_type,
            bank,
            invalid_page_number,
            page,
        );
        print_test_footer("info page read with invalid page number");

        print_test_header("info page write with invalid page number");
        let info_partition_type = InfoPartitionType::Type1;
        let bank = Bank::Bank1;
        let invalid_page_number = Info1PageIndex::Index0 as usize + 1;
        page = self.test_invalid_info_write_page_number(
            info_partition_type,
            bank,
            invalid_page_number,
            page,
        );
        self.set_page(page);
        print_test_footer("info page write with invalid page number");

        print_test_header("info page erase with invalid page number");
        let info_partition_type = InfoPartitionType::Type2;
        let bank = Bank::Bank0;
        let invalid_page_number = Info2PageIndex::Index1 as usize + 1;
        self.test_invalid_info_erase_page_number(info_partition_type, bank, invalid_page_number);
        print_test_footer("info page erase with invalid page number");
    }

    fn test_erase_data_page(&self, page_position: DataPagePosition) {
        let page_number = convert_data_page_position_to_page_number(page_position);
        self.set_current_data_test_page_position(page_position);
        let result = self.raw_erase_data_page(page_number);
        if let Err(error_code) = result {
            panic!(
                "Erasing page number {} must succeed, but instead failed with error {:?}",
                page_number, error_code
            );
        }
    }

    fn test_erase_busy(&self, page_position: DataPagePosition) {
        let page_number = convert_data_page_position_to_page_number(page_position);
        self.set_current_data_test_page_position(page_position);
        let result = self.raw_erase_data_page(page_number);
        if let Err(error_code) = result {
            panic!(
                "Erasing page number {} must succeed, but instead failed with error {:?}",
                page_number, error_code
            );
        }
        self.test_invalid_erase_busy(page_position);
    }

    fn test_erase_info_page_busy(&self, info_page_position: InfoPagePosition) {
        self.set_current_info_test_page_position(info_page_position);
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        let page = self.take_page();
        let result = self.raw_write_info_page(info_partition_type, bank, page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Writing info page {:?} must succeed, but instead failed with error {:?}",
                info_page_position, error_code,
            );
        }
        self.test_invalid_info_erase_busy(info_page_position);
    }

    fn test_write_busy(&self, page_position: DataPagePosition) {
        let page_number = convert_data_page_position_to_page_number(page_position);
        self.set_current_data_test_page_position(page_position);
        let page = self.take_page();
        let result = self.raw_read_page(page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Writing page number {} must succeed, but instead failed with error {:?}",
                page_number, error_code
            );
        }
        let mut placeholder_page = self.take_placeholder_page();
        placeholder_page = self.test_invalid_write_busy(page_number, placeholder_page);
        self.set_placeholder_page(placeholder_page);
    }

    fn test_write_info_page_busy(&self, info_page_position: InfoPagePosition) {
        self.set_current_info_test_page_position(info_page_position);
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        let page = self.take_page();
        let result = self.raw_read_info_page(info_partition_type, bank, page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Reading info page {:?} must succeed, but instead failed with error {:?}",
                info_page_position, error_code,
            );
        }

        let mut placeholder_page = self.take_placeholder_page();
        placeholder_page = self.test_invalid_info_write_busy(info_page_position, placeholder_page);
        self.set_placeholder_page(placeholder_page);
    }

    fn test_read_busy(&self, page_position: DataPagePosition) {
        let page_number = convert_data_page_position_to_page_number(page_position);
        self.set_current_data_test_page_position(page_position);
        let page = self.take_page();
        let result = self.raw_write_page(page_number, page);
        if let Err((error_code, _page)) = result {
            panic!(
                "Reading page number {} must succeed, but instead failed with error {:?}",
                page_number, error_code
            );
        }
        let mut placeholder_page = self.take_placeholder_page();
        placeholder_page = self.test_invalid_read_busy(page_number, placeholder_page);
        self.set_placeholder_page(placeholder_page);
    }

    fn test_read_info_page_busy(&self, info_page_position: InfoPagePosition) {
        self.set_current_info_test_page_position(info_page_position);
        let (info_partition_type, bank, page_number) =
            decompose_info_page_position(info_page_position);
        let mut page = self.take_page();
        let result = self.raw_erase_info_page(info_partition_type, bank, page_number);
        if let Err(error_code) = result {
            panic!(
                "Writing info page {:?} must succeed, but instead failed with error {:?}",
                info_page_position, error_code,
            );
        }

        page = self.test_invalid_info_read_busy(info_page_position, page);
        self.set_page(page);
    }

    fn check_successful_data_write(
        &self,
        write_page: &'static mut FlashPage<'a>,
        result: Result<(), Error>,
    ) {
        assert!(
            result.is_ok(),
            "Write must succeed for test case {:?}",
            self.get_current_test_case()
        );

        let current_data_test_page_position = self.get_current_data_test_page_position();
        // SAFETY: The reference goes out of scope by the end of the current function and since
        // check_successful_data_erase() cannot be called twice at the same time because the flash
        // controller does not support concurrent operations, the call to
        // convert_data_page_position_to_host_array() is safe.
        let host_array =
            unsafe { convert_data_page_position_to_host_array(current_data_test_page_position) };
        let written_message = self.take_current_test_message();
        check_page_content(host_array, written_message);

        self.set_page(write_page);
    }

    fn check_unsuccessful_write(&self, result: Result<(), Error>, expected_error: Error) {
        let actual_error = match result {
            Ok(()) => panic!(
                "Write succeeded when it was expected to fail with error {:?}",
                expected_error,
            ),
            Err(error) => error,
        };

        assert_eq!(
            actual_error, expected_error,
            "Expected write to fail with {:?}, got {:?} instead",
            expected_error, actual_error
        );
    }

    fn check_successful_info_write(
        &self,
        write_page: &'static mut FlashPage<'a>,
        result: Result<(), Error>,
    ) {
        if let Err(error) = result {
            panic!(
                "Write must succeed for test case {:?}, but instead failed with error {:?}",
                self.get_current_test_case(),
                error
            );
        }

        let current_info_test_page_position = self.get_current_info_test_page_position();
        let current_test_message = self.take_current_test_message();
        let write_page_content: &mut [u8; EARLGREY_PAGE_SIZE.get()] = write_page.as_mut();
        // Fill the page with ERASE_BYTE_VALUE to prevent false positive tests in case the read
        // operation doesn't work as expected.
        write_page_content.fill(ERASE_BYTE_VALUE);
        self.set_page(write_page);

        self.read_info_message(current_info_test_page_position, current_test_message);
    }

    fn check_successful_data_erase(&self, result: Result<(), Error>) {
        assert!(
            result.is_ok(),
            "Erase failed with error {:?} for case {:?} when it was expected to succeed",
            // PANIC: result is an Err variant because of the assert condition
            result.unwrap_err(),
            self.get_current_test_case()
        );

        let current_data_test_page_position = self.current_data_test_page_position.get();

        // SAFETY: The reference goes out of scope by the end of the current function and since
        // check_successful_data_erase() cannot be called twice at the same time because the flash
        // controller does not support concurrent operations, the call to
        // convert_data_page_position_to_host_array() is safe.
        let host_array =
            unsafe { convert_data_page_position_to_host_array(current_data_test_page_position) };

        assert!(
            is_slice_filled_with(host_array, ERASE_BYTE_VALUE),
            "Erase must fill all bytes with {:#x}",
            ERASE_BYTE_VALUE
        );
    }

    fn check_unsuccessful_erase(&self, result: Result<(), Error>, expected_error: Error) {
        let actual_error = match result {
            Ok(()) => panic!(
                "Erase succeeded when it was expected to fail with error {:?}",
                expected_error,
            ),
            Err(error) => error,
        };

        assert_eq!(
            expected_error, actual_error,
            "Expected error {:?}, got {:?}",
            expected_error, actual_error
        );
    }

    fn check_successful_info_erase(
        read_erased_page: &FlashPage<'a>,
        read_result: Result<(), Error>,
    ) {
        if let Err(error) = read_result {
            panic!(
                "Read failed with error {:?} when it was expected to succeed",
                error
            );
        }

        assert!(
            is_slice_filled_with(read_erased_page.as_ref(), ERASE_BYTE_VALUE),
            "Erase must fill all bytes with {:#x}",
            ERASE_BYTE_VALUE,
        );
    }

    fn set_page(&self, page: &'static mut FlashPage<'a>) {
        self.page.put(Some(page));
    }

    fn take_page(&self) -> &'static mut FlashPage<'a> {
        self.page.take().unwrap()
    }

    fn set_placeholder_page(&self, placeholder_page: &'static mut FlashPage<'a>) {
        self.placeholder_page.put(Some(placeholder_page));
    }

    fn take_placeholder_page(&self) -> &'static mut FlashPage<'a> {
        self.placeholder_page.take().unwrap()
    }

    fn get_current_test_case(&self) -> TestState {
        self.state.get()
    }

    fn check_successful_read(
        &self,
        read_page: &'static mut FlashPage<'a>,
        result: Result<(), Error>,
    ) {
        assert!(
            result.is_ok(),
            "Read must succeed for test case {:?}",
            self.get_current_test_case()
        );

        let read_page_array: &[u8; EARLGREY_PAGE_SIZE.get()] = read_page.as_mut();
        let written_message = self.take_current_test_message();
        check_page_content(read_page_array, written_message);

        self.set_page(read_page);
    }

    fn check_unsuccessful_read(&self, result: Result<(), Error>, expected_error: Error) {
        let actual_error = match result {
            Ok(()) => panic!(
                "Read succeeded when expected to fail with error {:?}",
                expected_error,
            ),
            Err(error) => error,
        };

        assert_eq!(
            actual_error, expected_error,
            "Expected read to fail with error {:?}, got {:?} instead",
            expected_error, actual_error
        );
    }

    fn check_op_status_clean_value(&self) {
        assert!(
            !self.flash.is_op_status_done_set(),
            "Bitfield DONE in register OP_STATUS must not be set upon operation completion",
        );

        assert!(
            !self.flash.is_op_status_err_set(),
            "Bitfield ERR in register OP_STATUS must not be set upon operation completion",
        );
    }

    fn check_err_code_clean_value(&self) {
        assert!(
            !self.flash.is_op_err_set(),
            "Bitfield OP_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_mp_err_set(),
            "Bitfield MP_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_rd_err_set(),
            "Bitfield RD_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_prog_err_set(),
            "Bitfield PROG_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_prog_win_err_set(),
            "Bitfield PROG_WIN_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_prog_type_err_set(),
            "Bitfield PROG_TYPE_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_update_err_set(),
            "Bitfield UPDATE_ERR in register ERR_CODE must not be set after operation completion",
        );

        assert!(
            !self.flash.is_macro_err_set(),
            "Bitfield MACRO_ERR in register ERR_CODE must not be set after operation completion",
        );
    }

    fn check_status_clean_value(&self) {
        assert!(
            !self.flash.is_status_rd_full_set(),
            "Bitfield RD_FULL in register STATUS must not be set after operation completion",
        );

        assert!(
            self.flash.is_status_rd_empty_set(),
            "Bitfield RD_FULL in register STATUS must not be set after operation completion",
        );

        assert!(
            !self.flash.is_status_prog_full_set(),
            "Bitfield PROG_FULL in register STATUS must not be set after operation completion",
        );

        assert!(
            self.flash.is_status_prog_empty_set(),
            "Bitfield PROG_EMPTY in register STATUS must not be set after operation completion",
        );

        assert!(
            !self.flash.is_status_init_wip_set(),
            "Bitfield INIT_WIP in register STATUS must not be set after operation completion",
        );

        assert!(
            self.flash.is_status_initialized_set(),
            "Bitfield INITIALIZED in register STATUS must be set after operation completion",
        );
    }

    fn check_control_clean_value(&self) {
        assert!(
            !self.flash.is_control_start_set(),
            "Bitfield START in register CONTROL must be cleared after operation completion",
        );
    }

    fn check_interrupt_state_clean_value(&self) {
        assert!(
            !self.flash.is_interrupt_corr_err_set(),
            "Bitfield CORR_ERR in register INTR_STATE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_op_done_set(),
            "Bitfield OP_DONE in register INTR_STATE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_rd_lvl_set(),
            "Bitfield RD_LVL in register INTR_STATE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_rd_full_set(),
            "Bitfield RD_FULL in register INTR_STATE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_prog_lvl_set(),
            "Bitfield PROG_LVL in register INTR_STATE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_prog_empty_set(),
            "Bitfield PROG_EMPTY in register INTR_STATE must be cleared after operation completion",
        );
    }

    fn check_interrupt_enable_clean_value(&self) {
        assert!(
            self.flash.is_interrupt_corr_err_enabled(),
            "Bitfield CORR_ERR in register INTR_ENABLE must be set after operation completion",
        );

        assert!(
            self.flash.is_interrupt_op_done_enabled(),
            "Bitfield OP_DONE in register INTR_ENABLE must be set after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_rd_lvl_enabled(),
            "Bitfield RD_LVL in register INTR_ENABLE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_rd_full_enabled(),
            "Bitfield RD_FULL in register INTR_ENABLE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_prog_lvl_enabled(),
            "Bitfield PROG_LVL in register INTR_ENABLE must be cleared after operation completion",
        );

        assert!(
            !self.flash.is_interrupt_prog_empty_enabled(),
            "Bitfield PROG_EMPTY in register INTR_ENABLE must be cleared after operation completion",
        );
    }

    fn check_interrupt_clean_value(&self) {
        self.check_interrupt_state_clean_value();
        self.check_interrupt_enable_clean_value();
    }

    fn check_registers_clean_value(&self) {
        self.check_op_status_clean_value();
        self.check_err_code_clean_value();
        self.check_status_clean_value();
        self.check_control_clean_value();
        self.check_interrupt_clean_value();
    }

    fn check_clean_state(&self) {
        self.check_registers_clean_value();

        assert_eq!(
            BusyStatus::NotBusy,
            self.flash.is_busy(),
            "The flash peripheral must not be busy after operation completion"
        );
    }

    pub(self) fn execute_next_test(&self) {
        match self.get_current_test_case() {
            TestState::InitialState => {
                print_test_header("data memory protection lock");
                self.test_data_memory_protection_lock(DataMemoryProtectionRegionIndex::Index0);
                print_test_footer("data memory protection lock");

                print_test_header("info0 memory protection lock");
                self.test_info_memory_protection_lock(InfoMemoryProtectionRegionIndex::Type0(
                    Info0MemoryProtectionRegionIndex::Bank1(Info0PageIndex::Index9),
                ));
                print_test_footer("info0 memory protection lock");

                print_test_header("info1 memory protection lock");
                self.test_info_memory_protection_lock(InfoMemoryProtectionRegionIndex::Type1(
                    Info1MemoryProtectionRegionIndex::Bank0(Info1PageIndex::Index0),
                ));
                print_test_footer("info1 memory protection lock");

                print_test_header("info2 memory protection lock");
                self.test_info_memory_protection_lock(InfoMemoryProtectionRegionIndex::Type2(
                    Info2MemoryProtectionRegionIndex::Bank1(Info2PageIndex::Index1),
                ));
                print_test_footer("info2 memory protection lock");

                let page_number = MAX_DATA_PAGE_INDEX.get() as usize * 3;
                let mut page = self.take_page();

                print_test_header("page read with invalid page number");
                page = self.test_invalid_read_page_number(page_number, page);
                print_test_footer("page read with invalid page number");

                print_test_header("page write with invalid page number");
                page = self.test_invalid_write_page_number(page_number, page);
                print_test_footer("page write with invalid page number");

                self.set_page(page);

                print_test_header("page erase with invalid page number");
                self.test_invalid_erase_page_number(page_number);
                print_test_footer("page erase with invalid page number");

                print_test_header("valid page erase");
                self.state.set(TestState::TestDataErase);
                let last_page_position = *self.page_position_range.end();
                self.test_erase_data_page(last_page_position);
            }
            TestState::TestDataErase => {
                print_test_header("valid page write");

                self.state.set(TestState::TestDataWrite);
                // From the last test, current_data_test_page_position is END
                let current_data_test_page_position = self.current_data_test_page_position.get();
                self.test_write_page(current_data_test_page_position);
            }
            TestState::TestDataWrite => {
                print_test_header("valid page read");
                self.state.set(TestState::TestDataRead);
                // From the last test, current_data_test_page_position is END
                let current_data_test_page_position = self.current_data_test_page_position.get();
                self.test_read_page(current_data_test_page_position);
            }
            TestState::TestDataRead => {
                if self.page_position_range.start() == self.page_position_range.end() {
                    println!("There is only one available page for testing. Memory protection tests are skipped");

                    self.state.set(TestState::TestDataReadFault);
                    self.execute_next_test();
                } else {
                    print_test_header("fault page erase");
                    self.state.set(TestState::TestDataEraseFault);
                    let page_position = *self.page_position_range.start();

                    self.test_erase_data_page(page_position);
                }
            }
            TestState::TestDataEraseFault => {
                print_test_header("fault page write");
                self.state.set(TestState::TestDataWriteFault);
                // From the last test, current_data_test_page_position is END - 1
                let current_data_test_page_position = self.get_current_data_test_page_position();
                self.test_write_page(current_data_test_page_position);
            }
            TestState::TestDataWriteFault => {
                print_test_header("fault page read");
                self.state.set(TestState::TestDataReadFault);
                // From the last test, current_data_test_page_position is END - 1
                let current_data_test_page_position = self.get_current_data_test_page_position();
                let page = self.take_page();

                let page_number =
                    convert_data_page_position_to_page_number(current_data_test_page_position);
                let result_attempt_result = self.raw_read_page(page_number, page);

                assert!(
                    result_attempt_result.is_ok(),
                    "Attempting to read page {:?} must succeed",
                    current_data_test_page_position
                );
            }
            TestState::TestDataReadFault => {
                print_test_header("page erase busy");
                self.state.set(TestState::TestDataEraseBusy);
                let page_position = *self.page_position_range.end();
                self.test_erase_busy(page_position);
            }
            TestState::TestDataEraseBusy => {
                print_test_header("page write busy");
                self.state.set(TestState::TestDataWriteBusy);
                let page_position = *self.page_position_range.end();
                self.test_write_busy(page_position);
            }
            TestState::TestDataWriteBusy => {
                print_test_header("page read busy");
                self.state.set(TestState::TestDataReadBusy);
                let page_position = *self.page_position_range.end();
                self.test_read_busy(page_position);
            }
            TestState::TestDataReadBusy => {
                // First test calls with invalid arguments
                self.test_info_invalid_arguments();
                print_test_header("valid info page erase");
                self.state.set(TestState::TestInfoErase);
                self.test_erase_info_page(VALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoErase => {
                print_test_header("valid info page write");
                self.state.set(TestState::TestInfoWrite);
                self.test_write_info_page(VALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoWrite => {
                print_test_header("fault info page erase");
                self.state.set(TestState::TestInfoEraseFault);
                self.test_erase_info_page(INVALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoEraseFault => {
                print_test_header("fault info page write");
                self.state.set(TestState::TestInfoWriteFault);
                self.test_write_info_page(INVALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoWriteFault => {
                print_test_header("fault info page read");
                self.state.set(TestState::TestInfoReadFault);
                let page = self.take_page();
                self.read_info_page(INVALID_INFO_PAGE_POSITION, page);
            }
            TestState::TestInfoReadFault => {
                print_test_header("info page erase busy");
                self.state.set(TestState::TestInfoEraseBusy);
                self.test_erase_info_page_busy(VALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoEraseBusy => {
                print_test_header("info page write busy");
                self.state.set(TestState::TestInfoWriteBusy);
                self.test_write_info_page_busy(VALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoWriteBusy => {
                print_test_header("info page read busy");
                self.state.set(TestState::TestInfoReadBusy);
                self.test_read_info_page_busy(VALID_INFO_PAGE_POSITION);
            }
            TestState::TestInfoReadBusy => {
                println!("\r\nFinished all tests. Everything is alright!\r\n");
            }
        }
    }
}

fn is_slice_filled_with(slice: &[u8], filling_byte: u8) -> bool {
    for &byte in slice {
        if byte != filling_byte {
            return false;
        }
    }

    true
}

use kernel::hil::flash::Error;

impl<'a> FlashClientTrait<FlashCtrl<'a>> for TestClient<'a> {
    fn read_complete(&self, read_page: &'static mut FlashPage<'a>, result: Result<(), Error>) {
        self.check_clean_state();

        match &result {
            ok @ Ok(()) => print_test_info!("Read completed: {:?}", ok),
            error => print_test_info!("Read completed: {:?}", error),
        }

        match self.get_current_test_case() {
            TestState::TestDataRead => {
                self.check_successful_read(read_page, result);
                print_test_footer("valid page read");
                self.execute_next_test();
            }
            TestState::TestDataReadFault => {
                const EXPECTED_ERROR: Error = Error::FlashMemoryProtectionError;
                self.set_page(read_page);
                self.check_unsuccessful_read(result, EXPECTED_ERROR);
                print_test_footer("fault page read");
                self.execute_next_test();
            }
            TestState::TestDataWriteBusy => {
                // The result of read is not important, so it is ignored.
                self.set_page(read_page);
                print_test_footer("page write busy");
                self.execute_next_test();
            }
            state => panic!("read_complete must not be trigerred for state {:?}", state),
        }
    }

    fn write_complete(&self, write_page: &'static mut FlashPage<'a>, result: Result<(), Error>) {
        self.check_clean_state();

        match &result {
            ok @ Ok(()) => print_test_info!("Write completed: {:?}", ok),
            error => print_test_info!("Write completed: {:?}", error),
        }

        match self.get_current_test_case() {
            TestState::TestDataWrite => {
                self.check_successful_data_write(write_page, result);
                print_test_footer("valid page write");
                self.execute_next_test();
            }
            TestState::TestDataRead => {
                // Now that a message has been written, check if reading the same page returns the
                // exact same message. Let's fill the page with ERASE_BYTE_VALUE. After the read,
                // it shall contain the exact same content as before the fill.
                let write_page_array: &mut [u8; EARLGREY_PAGE_SIZE.get()] = write_page.as_mut();
                write_page_array.fill(ERASE_BYTE_VALUE);
                self.set_page(write_page);
                self.read_data_message(self.current_data_test_page_position.get(), READ_MESSAGE);
            }
            TestState::TestDataWriteFault => {
                const EXPECTED_ERROR: Error = Error::FlashMemoryProtectionError;
                self.set_page(write_page);
                self.check_unsuccessful_write(result, EXPECTED_ERROR);
                print_test_footer("fault page write");
                self.execute_next_test();
            }
            TestState::TestDataReadBusy => {
                // The result of write is not important, so it is ignored.
                self.set_page(write_page);
                print_test_footer("page read busy");
                self.execute_next_test();
            }
            state => panic!(
                "write_complete() must not be triggered for state {:?}",
                state
            ),
        }
    }

    fn erase_complete(&self, result: Result<(), Error>) {
        self.check_clean_state();

        match &result {
            ok @ Ok(()) => print_test_info!("Erase completed: {:?}", ok),
            error => print_test_info!("Erase completed: {:?}", error),
        }

        match self.get_current_test_case() {
            TestState::TestDataErase => {
                self.check_successful_data_erase(result);
                print_test_footer("valid page erase");
                self.execute_next_test();
            }
            TestState::TestDataRead => {
                // After the page has been erased, it will be written with a specific message
                let current_data_test_page_position = self.current_data_test_page_position.get();
                self.write_message(current_data_test_page_position, READ_MESSAGE);
            }
            TestState::TestDataEraseFault => {
                const EXPECTED_ERROR: Error = Error::FlashMemoryProtectionError;
                self.check_unsuccessful_erase(result, EXPECTED_ERROR);
                print_test_footer("fault page erase");
                self.execute_next_test();
            }
            TestState::TestDataEraseBusy => {
                self.check_successful_data_erase(result);
                print_test_footer("page erase busy");
                self.execute_next_test();
            }
            state => panic!(
                "info_erase_complete() must not be triggered for state {:?}",
                state
            ),
        }
    }
}

impl<'a> FlashInfoClientTrait<FlashCtrl<'a>> for TestClient<'a> {
    fn info_read_complete(&self, read_page: &'static mut FlashPage<'a>, result: Result<(), Error>) {
        self.check_clean_state();

        match &result {
            ok @ Ok(()) => print_test_info!("Read completed: {:?}", ok),
            error => print_test_info!("Read completed: {:?}", error),
        }

        match self.get_current_test_case() {
            TestState::TestInfoErase => {
                Self::check_successful_info_erase(read_page, result);
                self.set_page(read_page);
                print_test_footer("valid info page erase");
                self.execute_next_test();
            }
            TestState::TestInfoWrite => {
                let current_test_message = self.take_current_test_message();
                let page_content: &[u8; EARLGREY_PAGE_SIZE.get()] = read_page.as_ref();

                check_page_content(page_content, current_test_message);
                self.set_page(read_page);
                print_test_footer("valid info page write");
                self.execute_next_test();
            }
            TestState::TestInfoReadFault => {
                const EXPECTED_ERROR: Error = Error::FlashMemoryProtectionError;
                self.set_page(read_page);
                self.check_unsuccessful_read(result, EXPECTED_ERROR);
                print_test_footer("fault info page read");
                self.execute_next_test();
            }
            TestState::TestInfoWriteBusy => {
                // This is a dummy read, so its result is discarded
                self.set_page(read_page);
                print_test_footer("info page write busy");
                self.execute_next_test();
            }
            state => panic!(
                "info_read_complete must not be trigerred for state {:?}",
                state
            ),
        }
    }

    fn info_write_complete(
        &self,
        write_page: &'static mut FlashPage<'a>,
        result: Result<(), Error>,
    ) {
        self.check_clean_state();

        match &result {
            ok @ Ok(()) => print_test_info!("Write completed: {:?}", ok),
            error => print_test_info!("Write completed: {:?}", error),
        }

        match self.get_current_test_case() {
            TestState::TestInfoWrite => {
                self.check_successful_info_write(write_page, result);
            }
            TestState::TestInfoWriteFault => {
                const EXPECTED_ERROR: Error = Error::FlashMemoryProtectionError;
                self.set_page(write_page);
                self.check_unsuccessful_write(result, EXPECTED_ERROR);
                print_test_footer("fault info page write");
                self.execute_next_test();
            }
            TestState::TestInfoEraseBusy => {
                // This is a dummy write, so the result is discarded.
                self.set_page(write_page);
                print_test_footer("info page erase busy");
                self.execute_next_test();
            }
            state => panic!(
                "info_write_complete() must not be triggered for state {:?}",
                state
            ),
        }
    }

    fn info_erase_complete(&self, result: Result<(), Error>) {
        self.check_clean_state();

        match &result {
            ok @ Ok(()) => print_test_info!("Erase completed: {:?}", ok),
            error => print_test_info!("Erase completed: {:?}", error),
        }

        match self.get_current_test_case() {
            TestState::TestInfoErase => {
                self.check_successful_data_erase(result);
                let current_info_test_page_position = self.get_current_info_test_page_position();
                let page = self.take_page();
                self.read_info_page(current_info_test_page_position, page);
            }
            TestState::TestInfoEraseFault => {
                const EXPECTED_ERROR: Error = Error::FlashMemoryProtectionError;
                self.check_unsuccessful_erase(result, EXPECTED_ERROR);
                print_test_footer("fault info page erase");
                self.execute_next_test();
            }
            TestState::TestInfoReadBusy => {
                // This is a dummy erase, so its result is discarded
                print_test_footer("info page read busy");
                self.execute_next_test();
            }
            state => panic!(
                "info_erase_complete() must not be triggered for state {:?}",
                state
            ),
        }
    }
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

fn test_memory_protection_region0(
    flash: &FlashCtrl<'_>,
    memory_protection_region_index: DataMemoryProtectionRegionIndex,
) {
    print_test_header("Memory protection region 0 configuration");

    assert!(
        flash.is_data_region_read_enabled(memory_protection_region_index),
        "Memory protection region 0 must have read enabled. This can be either an implementation bug or wrong memory protection configuration in board file."
    );

    assert!(
        flash.is_data_region_write_enabled(memory_protection_region_index),
        "Memory protection region 0 must have write enabled. This can be either an implementation bug or wrong memory protection configuration in board file."
    );

    assert!(
        flash.is_data_region_erase_enabled(memory_protection_region_index),
        "Memory protection region 0 must have erase enabled. This can be either an implementation bug or wrong memory protection configuration in board file."
    );

    assert!(
        !flash.is_data_region_scramble_enabled(memory_protection_region_index),
        "Memory protection region 0 must have scramble disabled. This can be either an implementation bug or wrong memory protection configuration in board file."
    );

    assert!(
        !flash.is_data_region_ecc_enabled(memory_protection_region_index),
        "Memory protection region 0 must have ecc disabled. This can be either an implementation bug or wrong memory protection configuration in board file."
    );

    assert!(
        flash.is_data_region_high_endurance_enabled(memory_protection_region_index),
        "Memory protection region 0 must have high_endurance enabled. This can be either an implementation bug or wrong memory protection configuration in board file."
    );

    print_test_footer("Memory protection region 0 configuration");
}

pub fn run_all(
    flash: &'static FlashCtrl<'static>,
    test_client: &'static TestClient<'static>,
    uart: &'static Uart<'static>,
) {
    // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
    unsafe { TEST_WRITER.set_uart(uart) };

    super::page_index::tests::run_all();
    super::page_position::tests::run_all();
    super::page::tests::run_all();
    super::memory_protection::tests::run_all();
    super::flash_address::tests::run_all();
    super::chunk::tests::run_all();

    test_memory_protection_region0(flash, DataMemoryProtectionRegionIndex::Index0);
    flash.set_client(test_client);
    flash.set_info_client(test_client);
    test_client.execute_next_test();

    println!("FLASH_CTRL TESTS PASSED");
}
