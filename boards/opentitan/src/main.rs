// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Board file for LowRISC OpenTitan RISC-V development platform.
//!
//! - <https://opentitan.org/>

#![no_std]
// Disable this attribute when documenting, as a workaround for
// https://github.com/rust-lang/rust/issues/62184.
#![cfg_attr(not(doc), no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::ptr::{addr_of, addr_of_mut};

use crate::hil::symmetric_encryption::AES128_BLOCK_SIZE;
use crate::otbn::OtbnComponent;
use crate::pinmux_layout::BoardPinmuxLayout;
use capsules_aes_gcm::aes_gcm;
use capsules_core::driver;
use capsules_core::reset_manager::ResetManager;
use capsules_core::virtualizers::virtual_aes_ccm;
use capsules_core::virtualizers::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
use capsules_extra::opentitan_alerthandler::AlertHandlerCapsule;
use capsules_extra::opentitan_sysrst::SystemReset;
use core::num::NonZeroU16;
use earlgrey::alert_handler;
use earlgrey::chip::EarlGreyDefaultPeripherals;
use earlgrey::chip_config::EarlGreyConfig;
use earlgrey::flash_ctrl;
use earlgrey::pinmux_config::EarlGreyPinmuxConfig;
use earlgrey::timer::RvTimer;
use lowrisc::aon_timer;

use kernel::capabilities;
use kernel::component::Component;
use kernel::hil;
use kernel::hil::entropy::Entropy32;
use kernel::hil::flash::Flash as FlashHIL;
use kernel::hil::flash::HasInfoClient;
use kernel::hil::hasher::Hasher;
use kernel::hil::i2c::I2CMaster;
use kernel::hil::led::LedHigh;
use kernel::hil::pattgen::PattGen;
use kernel::hil::reset_managment::ResetManagment;
use kernel::hil::rng::Rng;
use kernel::hil::symmetric_encryption::AES128;
use kernel::hil::usb::UsbController;
use kernel::platform::scheduler_timer::VirtualSchedulerTimer;
use kernel::platform::{KernelResources, SyscallDriverLookup, TbfHeaderFilterDefaultAllow};
use kernel::scheduler::priority::PrioritySched;
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::{create_capability, debug, static_init};
use lowrisc::flash_ctrl::FlashMPConfig;
use lowrisc::sysrst_ctrl::SysRstCtrl;
use rv32i::csr;

pub mod io;
mod otbn;
pub mod pinmux_layout;
#[cfg(test)]
mod tests;

/// The `earlgrey` chip crate supports multiple targets with slightly different
/// configurations, which are encoded through implementations of the
/// `earlgrey::chip_config::EarlGreyConfig` trait. This type provides different
/// implementations of the `EarlGreyConfig` trait, depending on Cargo's
/// conditional compilation feature flags. If no feature is selected,
/// compilation will error.
pub enum ChipConfig {}

#[cfg(feature = "fpga_cw310")]
impl EarlGreyConfig for ChipConfig {
    const NAME: &'static str = "fpga_cw310";

    // Clock frequencies as of https://github.com/lowRISC/opentitan/pull/19479
    const CPU_FREQ: u32 = 24_000_000;
    const PERIPHERAL_FREQ: u32 = 6_000_000;
    const AON_TIMER_FREQ: u32 = 250_000;
    const UART_BAUDRATE: u32 = 115200;
}

#[cfg(feature = "sim_verilator")]
impl EarlGreyConfig for ChipConfig {
    const NAME: &'static str = "sim_verilator";

    // Clock frequencies as of https://github.com/lowRISC/opentitan/pull/19368
    const CPU_FREQ: u32 = 500_000;
    const PERIPHERAL_FREQ: u32 = 125_000;
    const AON_TIMER_FREQ: u32 = 125_000;
    const UART_BAUDRATE: u32 = 7200;
}

// Whether to check for a proper ePMP handover configuration prior to ePMP
// initialization:
pub const EPMP_HANDOVER_CONFIG_CHECK: bool = false;

// EarlGrey ePMP debug mode
//
// This type determines whether JTAG access shall be enabled. When JTAG access
// is enabled, one less MPU region is available for use by userspace.
//
// Either
// - `earlgrey::epmp::EPMPDebugEnable`, or
// - `earlgrey::epmp::EPMPDebugDisable`.
pub type EPMPDebugConfig = earlgrey::epmp::EPMPDebugEnable;

// EarlGrey Chip type signature, including generic PMP argument and peripherals
// type:
pub type EarlGreyChip = earlgrey::chip::EarlGrey<
    'static,
    { <EPMPDebugConfig as earlgrey::epmp::EPMPDebugConfig>::TOR_USER_REGIONS },
    EarlGreyDefaultPeripherals<'static, ChipConfig, BoardPinmuxLayout>,
    ChipConfig,
    BoardPinmuxLayout,
    earlgrey::epmp::EarlGreyEPMP<{ EPMP_HANDOVER_CONFIG_CHECK }, EPMPDebugConfig>,
>;

const NUM_PROCS: usize = 4;

//
// Actual memory for holding the active process structures. Need an empty list
// at least.
static mut PROCESSES: [Option<&'static dyn kernel::process::Process>; 4] = [None; NUM_PROCS];

// Test access to the peripherals
#[cfg(test)]
static mut PERIPHERALS: Option<&'static EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>> =
    None;
// Test access to board
#[cfg(test)]
static mut BOARD: Option<&'static kernel::Kernel> = None;
// Test access to platform
#[cfg(test)]
static mut PLATFORM: Option<&'static EarlGrey> = None;
// Test access to main loop capability
#[cfg(test)]
static mut MAIN_CAP: Option<&dyn kernel::capabilities::MainLoopCapability> = None;
// Test access to alarm
static mut ALARM: Option<
    &'static MuxAlarm<'static, earlgrey::timer::RvTimer<'static, ChipConfig>>,
> = None;
// Test access to TicKV
static mut TICKV: Option<
    &capsules_extra::tickv::TicKVSystem<
        'static,
        capsules_core::virtualizers::virtual_flash::FlashUser<
            'static,
            earlgrey::flash_ctrl::FlashCtrl<'static>,
        >,
        capsules_extra::sip_hash::SipHasher24<'static>,
        2048,
    >,
> = None;
// Test access to AES
static mut AES: Option<
    &aes_gcm::Aes128Gcm<
        'static,
        virtual_aes_ccm::VirtualAES128CCM<'static, earlgrey::aes::Aes<'static>>,
    >,
> = None;
// Test access to SipHash
static mut SIPHASH: Option<&capsules_extra::sip_hash::SipHasher24<'static>> = None;
// Test access to RSA
static mut RSA_HARDWARE: Option<&lowrisc::rsa::OtbnRsa<'static>> = None;

// Test access to a software SHA256
#[cfg(test)]
static mut SHA256SOFT: Option<&capsules_extra::sha256::Sha256Software<'static>> = None;

static mut CHIP: Option<&'static EarlGreyChip> = None;
static mut PROCESS_PRINTER: Option<&'static capsules_system::process_printer::ProcessPrinterText> =
    None;

// How should the kernel respond when a process faults.
const FAULT_RESPONSE: capsules_system::process_policies::PanicFaultPolicy =
    capsules_system::process_policies::PanicFaultPolicy {};

/// Dummy buffer that causes the linker to reserve enough space for the stack.
#[no_mangle]
#[link_section = ".stack_buffer"]
pub static mut STACK_MEMORY: [u8; 0x1400] = [0; 0x1400];

/// A structure representing this platform that holds references to all
/// capsules for this platform. We've included an alarm and console.
struct EarlGrey {
    led: &'static capsules_core::led::LedDriver<
        'static,
        LedHigh<'static, earlgrey::gpio::GpioPin<'static, earlgrey::pinmux::PadConfig>>,
        8,
    >,
    gpio: &'static capsules_core::gpio::GPIO<
        'static,
        earlgrey::gpio::GpioPin<'static, earlgrey::pinmux::PadConfig>,
    >,
    console: &'static capsules_core::console::Console<'static>,
    alarm: &'static capsules_core::alarm::AlarmDriver<
        'static,
        VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<'static, ChipConfig>>,
    >,
    hmac: &'static capsules_extra::hmac::HmacDriver<'static, lowrisc::hmac::Hmac<'static>, 32>,
    info_flash: Option<
        &'static capsules_extra::info_flash::InfoFlash<
            'static,
            earlgrey::flash_ctrl::FlashCtrl<'static>,
        >,
    >,
    lldb: &'static capsules_core::low_level_debug::LowLevelDebug<
        'static,
        capsules_core::virtualizers::virtual_uart::UartDevice<'static>,
    >,
    i2c_master:
        &'static capsules_core::i2c_master::I2CMasterDriver<'static, lowrisc::i2c::I2c<'static>>,
    spi_controller: &'static capsules_core::spi_controller::Spi<
        'static,
        capsules_core::virtualizers::virtual_spi::VirtualSpiMasterDevice<
            'static,
            lowrisc::spi_host::SpiHost<'static>,
        >,
    >,
    rng: &'static capsules_core::rng::RngDriver<
        'static,
        capsules_core::rng::Entropy32ToRandom<'static, lowrisc::csrng::CsRng<'static>>,
    >,
    aes: &'static capsules_extra::symmetric_encryption::aes::AesDriver<
        'static,
        aes_gcm::Aes128Gcm<
            'static,
            virtual_aes_ccm::VirtualAES128CCM<'static, earlgrey::aes::Aes<'static>>,
        >,
    >,
    pattgen: &'static capsules_extra::pattgen::PattGen<'static, lowrisc::pattgen::PattGen<'static>>,
    kv_driver: &'static capsules_extra::kv_driver::KVStoreDriver<
        'static,
        capsules_extra::virtual_kv::VirtualKVPermissions<
            'static,
            capsules_extra::kv_store_permissions::KVStorePermissions<
                'static,
                capsules_extra::tickv_kv_store::TicKVKVStore<
                    'static,
                    capsules_extra::tickv::TicKVSystem<
                        'static,
                        capsules_core::virtualizers::virtual_flash::FlashUser<
                            'static,
                            earlgrey::flash_ctrl::FlashCtrl<'static>,
                        >,
                        capsules_extra::sip_hash::SipHasher24<'static>,
                        2048,
                    >,
                    [u8; 8],
                >,
            >,
        >,
    >,
    usb: &'static capsules_extra::usb::usb_user2::UsbSyscallDriver<
        'static,
        lowrisc::usb::Usb<'static>,
        { lowrisc::usb::MAXIMUM_PACKET_SIZE.get() },
    >,
    opentitan_sysrst: &'static SystemReset<'static, SysRstCtrl<'static>>,
    syscall_filter: &'static TbfHeaderFilterDefaultAllow,
    scheduler: &'static PrioritySched,
    scheduler_timer: &'static VirtualSchedulerTimer<
        VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<'static, ChipConfig>>,
    >,
    watchdog: &'static lowrisc::aon_timer::AonTimer<'static>,
    opentitan_alerthandler: &'static AlertHandlerCapsule,
    reset_manager: &'static ResetManager<'static, earlgrey::rstmgr::RstMgr>,
}

/// Mapping of integer syscalls to objects that implement syscalls.
impl SyscallDriverLookup for EarlGrey {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::syscall::SyscallDriver>) -> R,
    {
        match driver_num {
            capsules_core::led::DRIVER_NUM => f(Some(self.led)),
            capsules_extra::hmac::DRIVER_NUM => f(Some(self.hmac)),
            capsules_core::gpio::DRIVER_NUM => f(Some(self.gpio)),
            capsules_core::console::DRIVER_NUM => f(Some(self.console)),
            capsules_core::alarm::DRIVER_NUM => f(Some(self.alarm)),
            capsules_core::low_level_debug::DRIVER_NUM => f(Some(self.lldb)),
            capsules_core::i2c_master::DRIVER_NUM => f(Some(self.i2c_master)),
            capsules_core::spi_controller::DRIVER_NUM => f(Some(self.spi_controller)),
            capsules_core::rng::DRIVER_NUM => f(Some(self.rng)),
            capsules_extra::symmetric_encryption::aes::DRIVER_NUM => f(Some(self.aes)),
            capsules_extra::kv_driver::DRIVER_NUM => f(Some(self.kv_driver)),
            capsules_extra::info_flash::DRIVER_NUMBER => match self.info_flash {
                Some(info_flash) => f(Some(info_flash)),
                None => f(None),
            },
            capsules_extra::usb::usb_user2::DRIVER_NUM => f(Some(self.usb)),
            capsules_extra::pattgen::DRIVER_NUM => f(Some(self.pattgen)),
            capsules_extra::opentitan_alerthandler::DRIVER_NUM => {
                f(Some(self.opentitan_alerthandler))
            }
            capsules_core::reset_manager::DRIVER_NUM => f(Some(self.reset_manager)),
            capsules_extra::opentitan_sysrst::DRIVER_NUM => f(Some(self.opentitan_sysrst)),
            _ => f(None),
        }
    }
}

impl KernelResources<EarlGreyChip> for EarlGrey {
    type SyscallDriverLookup = Self;
    type SyscallFilter = TbfHeaderFilterDefaultAllow;
    type ProcessFault = ();
    type Scheduler = PrioritySched;
    type SchedulerTimer =
        VirtualSchedulerTimer<VirtualMuxAlarm<'static, RvTimer<'static, ChipConfig>>>;
    type WatchDog = lowrisc::aon_timer::AonTimer<'static>;
    type ContextSwitchCallback = ();

    fn syscall_driver_lookup(&self) -> &Self::SyscallDriverLookup {
        self
    }
    fn syscall_filter(&self) -> &Self::SyscallFilter {
        self.syscall_filter
    }
    fn process_fault(&self) -> &Self::ProcessFault {
        &()
    }
    fn scheduler(&self) -> &Self::Scheduler {
        self.scheduler
    }
    fn scheduler_timer(&self) -> &Self::SchedulerTimer {
        self.scheduler_timer
    }
    fn watchdog(&self) -> &Self::WatchDog {
        self.watchdog
    }
    fn context_switch_callback(&self) -> &Self::ContextSwitchCallback {
        &()
    }
}

// These symbols are defined in the linker script.
extern "C" {
    /// Beginning of the ROM region containing app images.
    static _sapps: u8;
    /// End of the ROM region containing app images.
    static _eapps: u8;
    /// Beginning of the RAM region for app memory.
    static mut _sappmem: u8;
    /// End of the RAM region for app memory.
    static _eappmem: u8;
    /// The start of the kernel text (Included only for kernel PMP)
    static _stext: u8;
    /// The end of the kernel text (Included only for kernel PMP)
    static _etext: u8;
    /// The start of the kernel / app / storage flash (Included only for kernel PMP)
    static _sflash: u8;
    /// The end of the kernel / app / storage flash (Included only for kernel PMP)
    static _eflash: u8;
    /// The start of the kernel / app RAM (Included only for kernel PMP)
    static _ssram: u8;
    /// The end of the kernel / app RAM (Included only for kernel PMP)
    static _esram: u8;
    /// The start of the OpenTitan manifest
    static _manifest: u8;
}

// Set this variable to true if tests are needed to be run
const FLASH_TESTS_ENABLED: bool = false;

fn get_flash_default_memory_protection_region() -> flash_ctrl::DefaultMemoryProtectionRegion {
    flash_ctrl::DefaultMemoryProtectionRegion::new()
}

fn get_flash_memory_protection_configuration() -> flash_ctrl::MemoryProtectionConfiguration {
    let flash_default_memory_protection_region = get_flash_default_memory_protection_region();

    if FLASH_TESTS_ENABLED {
        let page_index_range =
            earlgrey::flash_ctrl::tests::convert_flash_slice_to_page_position_range(unsafe {
                core::slice::from_raw_parts(
                    &_sapps as *const u8,
                    &_eapps as *const u8 as usize - &_sapps as *const u8 as usize,
                )
            })
            .unwrap();

        use earlgrey::flash_ctrl::DataMemoryProtectionRegionBase;
        use earlgrey::flash_ctrl::DataMemoryProtectionRegionSize;

        let memory_protection_page0_base =
            DataMemoryProtectionRegionBase::new(*page_index_range.end());

        const RAW_MEMORY_PROTECTION_PAGE0_SIZE: NonZeroU16 = match NonZeroU16::new(1) {
            Some(non_zero_u16) => non_zero_u16,
            None => unreachable!(),
        };

        const MEMORY_PROTECTION_PAGE0_SIZE: DataMemoryProtectionRegionSize =
            match DataMemoryProtectionRegionSize::new(RAW_MEMORY_PROTECTION_PAGE0_SIZE) {
                Ok(memory_protection_page0_size) => memory_protection_page0_size,
                Err(()) => unreachable!(),
            };

        flash_ctrl::MemoryProtectionConfiguration::new(flash_default_memory_protection_region)
            .enable_and_configure_data_region(
                flash_ctrl::DataMemoryProtectionRegionIndex::Index0,
                memory_protection_page0_base,
                MEMORY_PROTECTION_PAGE0_SIZE,
            )
            .enable_erase()
            .enable_write()
            .enable_read()
            .enable_high_endurance()
            .finalize_region()
            .enable_and_configure_info2_region(
                flash_ctrl::tests::VALID_INFO2_MEMORY_PROTECTION_REGION_INDEX,
            )
            .enable_erase()
            .enable_write()
            .enable_read()
            .enable_high_endurance()
            .finalize_region()
    } else {
        // SAFETY: &_stext represents a valid flash address in the host address space.
        let starting_address =
            flash_ctrl::FlashAddress::new_from_host_address(unsafe { &_stext as *const u8 })
                .unwrap();
        // SAFETY: &_etext represents a valid flash address in the host address space.
        let ending_address =
            flash_ctrl::FlashAddress::new_from_host_address(unsafe { &_etext as *const u8 })
                .unwrap();

        // Setup flash memory protection for the kernel
        // PANIC: the unwrap panics only if Flash(_stext) < FlashAddress(_etext), which occurs
        // only due to a linker script bug.
        flash_ctrl::MemoryProtectionConfiguration::new(flash_default_memory_protection_region)
            .enable_and_configure_data_region_from_pointers(
                flash_ctrl::DataMemoryProtectionRegionIndex::Index0,
                starting_address,
                ending_address,
            )
            .unwrap()
            .enable_read()
            .finalize_region()
            .enable_and_configure_info2_region(flash_ctrl::Info2MemoryProtectionRegionIndex::Bank1(
                flash_ctrl::Info2PageIndex::Index1,
            ))
            .enable_read()
            .enable_write()
            .enable_erase()
            .finalize_region()
    }
}

unsafe fn setup() -> (
    &'static kernel::Kernel,
    &'static EarlGrey,
    &'static EarlGreyChip,
    &'static EarlGreyDefaultPeripherals<'static, ChipConfig, BoardPinmuxLayout>,
) {
    // Ibex-specific handler
    earlgrey::chip::configure_trap_handler();

    // Set up memory protection immediately after setting the trap handler, to
    // ensure that much of the board initialization routine runs with ePMP
    // protection.
    let earlgrey_epmp = earlgrey::epmp::EarlGreyEPMP::new_debug(
        earlgrey::epmp::FlashRegion(
            rv32i::pmp::NAPOTRegionSpec::new(
                core::ptr::addr_of!(_sflash),
                core::ptr::addr_of!(_eflash) as usize - core::ptr::addr_of!(_sflash) as usize,
            )
            .unwrap(),
        ),
        earlgrey::epmp::RAMRegion(
            rv32i::pmp::NAPOTRegionSpec::new(
                core::ptr::addr_of!(_ssram),
                core::ptr::addr_of!(_esram) as usize - core::ptr::addr_of!(_ssram) as usize,
            )
            .unwrap(),
        ),
        earlgrey::epmp::MMIORegion(
            rv32i::pmp::NAPOTRegionSpec::new(
                0x40000000 as *const u8, // start
                0x10000000,              // size
            )
            .unwrap(),
        ),
        earlgrey::epmp::KernelTextRegion(
            rv32i::pmp::TORRegionSpec::new(
                core::ptr::addr_of!(_stext),
                core::ptr::addr_of!(_etext),
            )
            .unwrap(),
        ),
        // RV Debug Manager memory region (required for JTAG debugging).
        // This access can be disabled by changing the EarlGreyEPMP type
        // parameter `EPMPDebugConfig` to `EPMPDebugDisable`, in which case
        // this expects to be passed a unit (`()`) type.
        earlgrey::epmp::RVDMRegion(
            rv32i::pmp::NAPOTRegionSpec::new(
                0x00010000 as *const u8, // start
                0x00001000,              // size
            )
            .unwrap(),
        ),
    )
    .unwrap();

    // Configure board layout in pinmux
    BoardPinmuxLayout::setup();

    // initialize capabilities
    let process_mgmt_cap = create_capability!(capabilities::ProcessManagementCapability);
    let memory_allocation_cap = create_capability!(capabilities::MemoryAllocationCapability);

    let board_kernel = static_init!(kernel::Kernel, kernel::Kernel::new(&*addr_of!(PROCESSES)));

    let flash_memory_protection_configuration = get_flash_memory_protection_configuration();

    let peripherals = static_init!(
        EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>,
        EarlGreyDefaultPeripherals::new(flash_memory_protection_configuration)
    );
    peripherals.init();

    // retrieve reset reason
    // RSTMGR::reset_reason might get cleared by ROM code that runs before Tock and cached in RetentionRAM
    // if reset_reason in HW peripheral is cleared, attempt to read it from RRAM
    // TODO replace unsafe fn `get_rr_from_rram()` with RRAM function call when RRAM is ready
    let reset_reason = peripherals
        .rst_mgmt
        .reset_reason()
        .or(earlgrey::rstmgr::RstMgr::get_rr_from_rram());

    let reset_manager = kernel::static_init!(
        capsules_core::reset_manager::ResetManager<'static, earlgrey::rstmgr::RstMgr>,
        ResetManager::new(
            &peripherals.rst_mgmt,
            board_kernel.create_grant(
                capsules_core::reset_manager::DRIVER_NUM,
                &memory_allocation_cap
            )
        )
    );

    // Configure kernel debug gpios as early as possible
    kernel::debug::assign_gpios(
        Some(&peripherals.gpio_port[7]), // First LED
        None,
        None,
    );

    // Create a shared UART channel for the console and for kernel debug.
    let uart_mux =
        components::console::UartMuxComponent::new(&peripherals.uart0, ChipConfig::UART_BAUDRATE)
            .finalize(components::uart_mux_component_static!());

    // LEDs
    // Start with half on and half off
    let led = components::led::LedsComponent::new().finalize(components::led_component_static!(
        LedHigh<'static, earlgrey::gpio::GpioPin<earlgrey::pinmux::PadConfig>>,
        LedHigh::new(&peripherals.gpio_port[8]),
        LedHigh::new(&peripherals.gpio_port[9]),
        LedHigh::new(&peripherals.gpio_port[10]),
        LedHigh::new(&peripherals.gpio_port[11]),
        LedHigh::new(&peripherals.gpio_port[12]),
        LedHigh::new(&peripherals.gpio_port[13]),
        LedHigh::new(&peripherals.gpio_port[14]),
        LedHigh::new(&peripherals.gpio_port[15]),
    ));

    let gpio = components::gpio::GpioComponent::new(
        board_kernel,
        capsules_core::gpio::DRIVER_NUM,
        components::gpio_component_helper!(
            earlgrey::gpio::GpioPin<earlgrey::pinmux::PadConfig>,
            0 => &peripherals.gpio_port[0],
            1 => &peripherals.gpio_port[1],
            2 => &peripherals.gpio_port[2],
            3 => &peripherals.gpio_port[3],
            4 => &peripherals.gpio_port[4],
            5 => &peripherals.gpio_port[5],
            6 => &peripherals.gpio_port[6],
            7 => &peripherals.gpio_port[15],
            8 => &peripherals.gpio_port[7],
            9 => &peripherals.gpio_port[20],
        ),
    )
    .finalize(components::gpio_component_static!(
        earlgrey::gpio::GpioPin<earlgrey::pinmux::PadConfig>
    ));

    peripherals.timer.setup();

    // Create a shared virtualization mux layer on top of a single hardware
    // alarm.
    let mux_alarm = static_init!(
        MuxAlarm<'static, earlgrey::timer::RvTimer<ChipConfig>>,
        MuxAlarm::new(&peripherals.timer)
    );
    hil::time::Alarm::set_alarm_client(&peripherals.timer, mux_alarm);

    ALARM = Some(mux_alarm);

    // Alarm
    let virtual_alarm_user = static_init!(
        VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<ChipConfig>>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_user.setup();

    let scheduler_timer_virtual_alarm = static_init!(
        VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<ChipConfig>>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    scheduler_timer_virtual_alarm.setup();

    let alarm = static_init!(
        capsules_core::alarm::AlarmDriver<
            'static,
            VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<ChipConfig>>,
        >,
        capsules_core::alarm::AlarmDriver::new(
            virtual_alarm_user,
            board_kernel.create_grant(capsules_core::alarm::DRIVER_NUM, &memory_allocation_cap)
        )
    );
    hil::time::Alarm::set_alarm_client(virtual_alarm_user, alarm);

    let scheduler_timer = static_init!(
        VirtualSchedulerTimer<
            VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<'static, ChipConfig>>,
        >,
        VirtualSchedulerTimer::new(scheduler_timer_virtual_alarm)
    );

    let chip = static_init!(
        EarlGreyChip,
        earlgrey::chip::EarlGrey::new(peripherals, earlgrey_epmp)
    );
    CHIP = Some(chip);

    // Need to enable all interrupts for Tock Kernel
    chip.enable_plic_interrupts();
    // enable interrupts globally
    csr::CSR.mie.modify(
        csr::mie::mie::msoft::SET + csr::mie::mie::mtimer::CLEAR + csr::mie::mie::mext::SET,
    );
    csr::CSR.mstatus.modify(csr::mstatus::mstatus::mie::SET);

    // Setup the console.
    let console = components::console::ConsoleComponent::new(
        board_kernel,
        capsules_core::console::DRIVER_NUM,
        uart_mux,
    )
    .finalize(components::console_component_static!());
    // Create the debugger object that handles calls to `debug!()`.
    components::debug_writer::DebugWriterComponent::new(uart_mux)
        .finalize(components::debug_writer_component_static!());

    let lldb = components::lldb::LowLevelDebugComponent::new(
        board_kernel,
        capsules_core::low_level_debug::DRIVER_NUM,
        uart_mux,
    )
    .finalize(components::low_level_debug_component_static!());

    let hmac = components::hmac::HmacComponent::new(
        board_kernel,
        capsules_extra::hmac::DRIVER_NUM,
        &peripherals.hmac,
    )
    .finalize(components::hmac_component_static!(lowrisc::hmac::Hmac, 32));

    let i2c_master_buffer = static_init!(
        [u8; capsules_core::i2c_master::BUFFER_LENGTH],
        [0; capsules_core::i2c_master::BUFFER_LENGTH]
    );
    let i2c_master = static_init!(
        capsules_core::i2c_master::I2CMasterDriver<'static, lowrisc::i2c::I2c<'static>>,
        capsules_core::i2c_master::I2CMasterDriver::new(
            &peripherals.i2c0,
            i2c_master_buffer,
            board_kernel.create_grant(
                capsules_core::i2c_master::DRIVER_NUM,
                &memory_allocation_cap
            )
        )
    );

    peripherals.i2c0.set_master_client(i2c_master);

    //SPI
    let mux_spi = components::spi::SpiMuxComponent::new(&peripherals.spi_host0).finalize(
        components::spi_mux_component_static!(lowrisc::spi_host::SpiHost),
    );

    let spi_controller = components::spi::SpiSyscallComponent::new(
        board_kernel,
        mux_spi,
        0,
        capsules_core::spi_controller::DRIVER_NUM,
    )
    .finalize(components::spi_syscall_component_static!(
        lowrisc::spi_host::SpiHost
    ));

    let process_printer = components::process_printer::ProcessPrinterTextComponent::new()
        .finalize(components::process_printer_text_component_static!());
    PROCESS_PRINTER = Some(process_printer);

    // USB support is currently broken in the OpenTitan hardware
    // See https://github.com/lowRISC/opentitan/issues/2598 for more details
    // let usb = components::usb::UsbComponent::new(
    //     board_kernel,
    //     capsules_extra::usb::usb_user::DRIVER_NUM,
    //     &peripherals.usb,
    // )
    // .finalize(components::usb_component_static!(earlgrey::usbdev::Usb));

    // Uncomment if you want to test the USB client at the kernel level. Don't forget to uncomment
    // the other USB client a few lines below.
    /*
    use kernel::hil::usb::Client;
    let usb_client = static_init!(
        capsules_extra::usb::usbc_client::Client<lowrisc::usb::Usb>,
        capsules_extra::usb::usbc_client::Client::new(&peripherals.usb, 64),
    );

    use kernel::hil::usb::UsbController;
    peripherals.usb.set_client(usb_client);
    usb_client.enable();
    usb_client.attach();
    */

    let usb_client = static_init!(
        capsules_extra::usb::usb_user2::UsbClient<
            'static,
            lowrisc::usb::Usb,
            { lowrisc::usb::MAXIMUM_PACKET_SIZE.get() },
        >,
        capsules_extra::usb::usb_user2::UsbClient::new(&peripherals.usb)
    );

    peripherals.usb.set_client(usb_client);

    let usb = static_init!(
        capsules_extra::usb::usb_user2::UsbSyscallDriver<
            'static,
            lowrisc::usb::Usb,
            { lowrisc::usb::MAXIMUM_PACKET_SIZE.get() },
        >,
        capsules_extra::usb::usb_user2::UsbSyscallDriver::new(
            usb_client,
            board_kernel.create_grant(
                capsules_extra::usb::usb_user2::DRIVER_NUM,
                &memory_allocation_cap
            )
        ),
    );
    usb.init();

    // Kernel storage region, allocated with the storage_volume!
    // macro in common/utils.rs
    extern "C" {
        /// Beginning on the ROM region containing app images.
        static _sstorage: u8;
        static _estorage: u8;
    }

    /*
    // Flash setup memory protection for the ROM/Kernel
    // Only allow reads for this region, any other ops will cause an MP fault
    let mp_cfg = FlashMPConfig {
        read_en: true,
        write_en: false,
        erase_en: false,
        scramble_en: false,
        ecc_en: false,
        he_en: false,
    };

    // Allocate a flash protection region (associated cfg number: 0), for the code section.
    if let Err(e) = peripherals.flash_ctrl.mp_set_region_perms(
        core::ptr::addr_of!(_manifest) as usize,
        core::ptr::addr_of!(_etext) as usize,
        0,
        &mp_cfg,
    ) {
        debug!("Failed to set flash memory protection: {:?}", e);
    } else {
        // Lock region 0, until next system reset.
        if let Err(e) = peripherals.flash_ctrl.mp_lock_region_cfg(0) {
            debug!("Failed to lock memory protection config: {:?}", e);
        }
    }
    */

    // Flash
    let flash_ctrl_read_buf = static_init!(
        [u8; lowrisc::flash_ctrl::PAGE_SIZE],
        [0; lowrisc::flash_ctrl::PAGE_SIZE]
    );
    let page_buffer = static_init!(
        earlgrey::flash_ctrl::RawFlashCtrlPage,
        earlgrey::flash_ctrl::RawFlashCtrlPage::default()
    );

    let mux_flash = components::flash::FlashMuxComponent::new(&peripherals.flash_ctrl).finalize(
        components::flash_mux_component_static!(earlgrey::flash_ctrl::FlashCtrl),
    );

    // SipHash
    let sip_hash = static_init!(
        capsules_extra::sip_hash::SipHasher24,
        capsules_extra::sip_hash::SipHasher24::new()
    );
    kernel::deferred_call::DeferredCallClient::register(sip_hash);
    SIPHASH = Some(sip_hash);

    // TicKV
    let tickv = components::tickv::TicKVComponent::new(
        sip_hash,
        mux_flash,                                     // Flash controller
        lowrisc::flash_ctrl::FLASH_PAGES_PER_BANK - 1, // Region offset (End of Bank0/Use Bank1)
        // Region Size
        lowrisc::flash_ctrl::FLASH_PAGES_PER_BANK * lowrisc::flash_ctrl::PAGE_SIZE,
        flash_ctrl_read_buf, // Buffer used internally in TicKV
        page_buffer,         // Buffer used with the flash controller
    )
    .finalize(components::tickv_component_static!(
        earlgrey::flash_ctrl::FlashCtrl,
        capsules_extra::sip_hash::SipHasher24,
        2048
    ));
    hil::flash::HasClient::set_client(&peripherals.flash_ctrl, mux_flash);
    sip_hash.set_client(tickv);
    TICKV = Some(tickv);

    let kv_store = components::kv::TicKVKVStoreComponent::new(tickv).finalize(
        components::tickv_kv_store_component_static!(
            capsules_extra::tickv::TicKVSystem<
                capsules_core::virtualizers::virtual_flash::FlashUser<
                    earlgrey::flash_ctrl::FlashCtrl,
                >,
                capsules_extra::sip_hash::SipHasher24<'static>,
                2048,
            >,
            capsules_extra::tickv::TicKVKeyType,
        ),
    );

    let kv_store_permissions = components::kv::KVStorePermissionsComponent::new(kv_store).finalize(
        components::kv_store_permissions_component_static!(
            capsules_extra::tickv_kv_store::TicKVKVStore<
                capsules_extra::tickv::TicKVSystem<
                    capsules_core::virtualizers::virtual_flash::FlashUser<
                        earlgrey::flash_ctrl::FlashCtrl,
                    >,
                    capsules_extra::sip_hash::SipHasher24<'static>,
                    2048,
                >,
                capsules_extra::tickv::TicKVKeyType,
            >
        ),
    );

    let mux_kv = components::kv::KVPermissionsMuxComponent::new(kv_store_permissions).finalize(
        components::kv_permissions_mux_component_static!(
            capsules_extra::kv_store_permissions::KVStorePermissions<
                capsules_extra::tickv_kv_store::TicKVKVStore<
                    capsules_extra::tickv::TicKVSystem<
                        capsules_core::virtualizers::virtual_flash::FlashUser<
                            earlgrey::flash_ctrl::FlashCtrl,
                        >,
                        capsules_extra::sip_hash::SipHasher24<'static>,
                        2048,
                    >,
                    capsules_extra::tickv::TicKVKeyType,
                >,
            >
        ),
    );

    let virtual_kv_driver = components::kv::VirtualKVPermissionsComponent::new(mux_kv).finalize(
        components::virtual_kv_permissions_component_static!(
            capsules_extra::kv_store_permissions::KVStorePermissions<
                capsules_extra::tickv_kv_store::TicKVKVStore<
                    capsules_extra::tickv::TicKVSystem<
                        capsules_core::virtualizers::virtual_flash::FlashUser<
                            earlgrey::flash_ctrl::FlashCtrl,
                        >,
                        capsules_extra::sip_hash::SipHasher24<'static>,
                        2048,
                    >,
                    capsules_extra::tickv::TicKVKeyType,
                >,
            >
        ),
    );

    let kv_driver = components::kv::KVDriverComponent::new(
        virtual_kv_driver,
        board_kernel,
        capsules_extra::kv_driver::DRIVER_NUM,
    )
    .finalize(components::kv_driver_component_static!(
        capsules_extra::virtual_kv::VirtualKVPermissions<
            capsules_extra::kv_store_permissions::KVStorePermissions<
                capsules_extra::tickv_kv_store::TicKVKVStore<
                    capsules_extra::tickv::TicKVSystem<
                        capsules_core::virtualizers::virtual_flash::FlashUser<
                            earlgrey::flash_ctrl::FlashCtrl,
                        >,
                        capsules_extra::sip_hash::SipHasher24<'static>,
                        2048,
                    >,
                    capsules_extra::tickv::TicKVKeyType,
                >,
            >,
        >
    ));

    let info_flash = if !FLASH_TESTS_ENABLED {
        use capsules_extra::info_flash::InfoFlash;
        let raw_flash_ctrl_page = static_init!(
            earlgrey::flash_ctrl::RawFlashCtrlPage,
            earlgrey::flash_ctrl::RawFlashCtrlPage::default()
        );

        let info_flash: &'static InfoFlash<earlgrey::flash_ctrl::FlashCtrl> = static_init!(
            InfoFlash<earlgrey::flash_ctrl::FlashCtrl>,
            InfoFlash::new(
                &peripherals.flash_ctrl,
                board_kernel.create_grant(
                    capsules_extra::info_flash::DRIVER_NUMBER,
                    &memory_allocation_cap
                ),
                raw_flash_ctrl_page,
            ),
        );

        peripherals.flash_ctrl.set_info_client(info_flash);

        Some(info_flash)
    } else {
        // Don't instantiate the info flash capsule when testing the flash peripheral. It may
        // interfere with the tests.
        None
    };

    let mux_otbn = crate::otbn::AccelMuxComponent::new(&peripherals.otbn)
        .finalize(otbn_mux_component_static!());

    let otbn = OtbnComponent::new(mux_otbn).finalize(crate::otbn_component_static!());

    let otbn_rsa_internal_buf = static_init!([u8; 512], [0; 512]);

    // Use the OTBN to create an RSA engine
    if let Ok((rsa_imem_start, rsa_imem_length, rsa_dmem_start, rsa_dmem_length)) =
        crate::otbn::find_app(
            "otbn-rsa",
            core::slice::from_raw_parts(
                core::ptr::addr_of!(_sapps),
                core::ptr::addr_of!(_eapps) as usize - core::ptr::addr_of!(_sapps) as usize,
            ),
        )
    {
        let rsa_hardware = static_init!(
            lowrisc::rsa::OtbnRsa<'static>,
            lowrisc::rsa::OtbnRsa::new(
                otbn,
                lowrisc::rsa::AppAddresses {
                    imem_start: rsa_imem_start,
                    imem_size: rsa_imem_length,
                    dmem_start: rsa_dmem_start,
                    dmem_size: rsa_dmem_length
                },
                otbn_rsa_internal_buf,
            )
        );
        peripherals.otbn.set_client(rsa_hardware);
        RSA_HARDWARE = Some(rsa_hardware);
    } else {
        debug!("Unable to find otbn-rsa, disabling RSA support");
    }

    // Convert hardware RNG to the Random interface.
    let entropy_to_random = static_init!(
        capsules_core::rng::Entropy32ToRandom<'static, lowrisc::csrng::CsRng<'static>>,
        capsules_core::rng::Entropy32ToRandom::new(&peripherals.rng)
    );
    peripherals.rng.set_client(entropy_to_random);
    // Setup RNG for userspace
    let rng = static_init!(
        capsules_core::rng::RngDriver<
            'static,
            capsules_core::rng::Entropy32ToRandom<'static, lowrisc::csrng::CsRng<'static>>,
        >,
        capsules_core::rng::RngDriver::new(
            entropy_to_random,
            board_kernel.create_grant(capsules_core::rng::DRIVER_NUM, &memory_allocation_cap)
        )
    );
    entropy_to_random.set_client(rng);

    const CRYPT_SIZE: usize = 7 * AES128_BLOCK_SIZE;

    let ccm_mux = static_init!(
        virtual_aes_ccm::MuxAES128CCM<'static, earlgrey::aes::Aes<'static>>,
        virtual_aes_ccm::MuxAES128CCM::new(&peripherals.aes)
    );
    kernel::deferred_call::DeferredCallClient::register(ccm_mux);
    peripherals.aes.set_client(ccm_mux);

    let ccm_client = components::aes::AesVirtualComponent::new(ccm_mux).finalize(
        components::aes_virtual_component_static!(earlgrey::aes::Aes<'static>),
    );

    let crypt_buf2 = static_init!([u8; CRYPT_SIZE], [0x00; CRYPT_SIZE]);
    let gcm_client = static_init!(
        aes_gcm::Aes128Gcm<
            'static,
            virtual_aes_ccm::VirtualAES128CCM<'static, earlgrey::aes::Aes<'static>>,
        >,
        aes_gcm::Aes128Gcm::new(ccm_client, crypt_buf2)
    );
    ccm_client.set_client(gcm_client);

    let aes = components::aes::AesDriverComponent::new(
        board_kernel,
        capsules_extra::symmetric_encryption::aes::DRIVER_NUM,
        gcm_client,
    )
    .finalize(components::aes_driver_component_static!(
        aes_gcm::Aes128Gcm<
            'static,
            virtual_aes_ccm::VirtualAES128CCM<'static, earlgrey::aes::Aes<'static>>,
        >,
    ));

    AES = Some(gcm_client);

    #[cfg(test)]
    {
        use capsules_extra::sha256::Sha256Software;

        let sha_soft = static_init!(Sha256Software<'static>, Sha256Software::new());
        kernel::deferred_call::DeferredCallClient::register(sha_soft);

        SHA256SOFT = Some(sha_soft);
    }

    hil::symmetric_encryption::AES128GCM::set_client(gcm_client, aes);
    hil::symmetric_encryption::AES128::set_client(gcm_client, ccm_client);

    let pattgen = static_init!(
        capsules_extra::pattgen::PattGen<lowrisc::pattgen::PattGen<'static>>,
        capsules_extra::pattgen::PattGen::new(
            &peripherals.pattgen,
            board_kernel.create_grant(capsules_extra::pattgen::DRIVER_NUM, &memory_allocation_cap)
        )
    );
    peripherals.pattgen.set_client(pattgen);

    let syscall_filter = static_init!(TbfHeaderFilterDefaultAllow, TbfHeaderFilterDefaultAllow {});
    let scheduler = components::sched::priority::PriorityComponent::new(board_kernel)
        .finalize(components::priority_component_static!());
    let watchdog = &peripherals.watchdog;

    let alert_handler_capsule = static_init!(
        AlertHandlerCapsule,
        AlertHandlerCapsule::new(board_kernel.create_grant(
            capsules_extra::opentitan_alerthandler::DRIVER_NUM,
            &memory_allocation_cap
        ))
    );
    peripherals.alert_handler.set_client(alert_handler_capsule);

    let opentitan_sysrst = static_init!(
        SystemReset<'static, SysRstCtrl>,
        SystemReset::new(
            &peripherals.sysreset,
            board_kernel.create_grant(
                driver::NUM::OpenTitanSysRst as usize,
                &memory_allocation_cap
            ),
        )
    );
    peripherals.sysreset.set_client(Some(opentitan_sysrst));

    peripherals.sysreset.enable_interrupts();

    let earlgrey = static_init!(
        EarlGrey,
        EarlGrey {
            gpio,
            led,
            console,
            alarm,
            hmac,
            info_flash,
            rng,
            lldb: lldb,
            i2c_master,
            spi_controller,
            aes,
            usb,
            kv_driver,
            pattgen,
            syscall_filter,
            scheduler,
            scheduler_timer,
            opentitan_sysrst,
            watchdog,
            reset_manager,
            opentitan_alerthandler: alert_handler_capsule,
        }
    );

    // Pattern generation tests
    /*
    let pattgen_test = static_init!(
        lowrisc::pattgen::tests::PattGenTest,
        lowrisc::pattgen::tests::PattGenTest::new(&peripherals.pattgen),
    );
    lowrisc::pattgen::tests::run_all(pattgen_test);
    */

    earlgrey.reset_manager.startup();
    earlgrey.reset_manager.populate_reset_reason(reset_reason);

    /* TESTs */

    #[cfg(feature = "test_alerthandler")]
    {
        test_alerthandler(peripherals, mux_alarm);
    }

    #[cfg(feature = "test_sysrst_ctrl")]
    {
        test_sysrst_ctrl(peripherals);
    }

    kernel::process::load_processes(
        board_kernel,
        chip,
        core::slice::from_raw_parts(
            core::ptr::addr_of!(_sapps),
            core::ptr::addr_of!(_eapps) as usize - core::ptr::addr_of!(_sapps) as usize,
        ),
        core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(_sappmem),
            core::ptr::addr_of!(_eappmem) as usize - core::ptr::addr_of!(_sappmem) as usize,
        ),
        &mut *addr_of_mut!(PROCESSES),
        &FAULT_RESPONSE,
        &process_mgmt_cap,
    )
    .unwrap_or_else(|err| {
        debug!("Error loading processes!");
        debug!("{:?}", err);
    });

    #[cfg(feature = "test_sram_ret")]
    peripherals
        .sram_ret
        .test(&peripherals.rst_mgmt, &peripherals.uart0);

    #[cfg(feature = "test_aon_timer")]
    {
        peripherals.watchdog.test(
            &peripherals.uart0,
            &peripherals.sram_ret,
            &peripherals.sram_ret,
        );
        test_aon_timer(peripherals, mux_alarm);
    }

    debug!("OpenTitan initialisation complete. Entering main loop");

    (board_kernel, earlgrey, chip, peripherals)
}

#[cfg(feature = "test_sysrst_ctrl")]
fn test_sysrst_ctrl(peripherals: &EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>) {
    pinmux_layout::prepare_wiring_sysrst_ctrl_tests();
    lowrisc::sysrst_ctrl::tests::test_all(
        &peripherals.sysreset,
        &peripherals.gpio_port[7],
        &peripherals.gpio_port[2],
        &peripherals.gpio_port[20],
    );
}

fn test_flash(
    flash_ctrl: &'static earlgrey::flash_ctrl::FlashCtrl,
    uart: &'static earlgrey::uart::Uart<'static>,
) {
    let flash_page = unsafe {
        static_init!(
            <earlgrey::flash_ctrl::FlashCtrl as FlashHIL>::Page,
            <earlgrey::flash_ctrl::FlashCtrl as FlashHIL>::Page::default()
        )
    };

    let placeholder_flash_page = unsafe {
        static_init!(
            <earlgrey::flash_ctrl::FlashCtrl as FlashHIL>::Page,
            <earlgrey::flash_ctrl::FlashCtrl as FlashHIL>::Page::default()
        )
    };

    let page_index_range =
        earlgrey::flash_ctrl::tests::convert_flash_slice_to_page_position_range(unsafe {
            core::slice::from_raw_parts(
                &_sapps as *const u8,
                &_eapps as *const u8 as usize - &_sapps as *const u8 as usize,
            )
        })
        .unwrap();

    let test_client = unsafe {
        static_init!(
            earlgrey::flash_ctrl::tests::TestClient,
            earlgrey::flash_ctrl::tests::TestClient::new(
                flash_ctrl,
                flash_page,
                placeholder_flash_page,
                page_index_range
            ),
        )
    };

    earlgrey::flash_ctrl::tests::run_all(flash_ctrl, test_client, uart);
}

#[cfg(feature = "test_alerthandler")]
unsafe fn test_alerthandler(
    peripherals: &'static EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>,
    mux_alarm: &'static MuxAlarm<'static, RvTimer<ChipConfig>>,
) {
    // an Alarm is needed for some of the tests as alert handling works using interrupts
    let virtual_alarm_tests = static_init!(
        VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<ChipConfig>>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_tests.setup();

    let alert_handler_tests = static_init!(
        alert_handler::tests::Tests<VirtualMuxAlarm<'static, RvTimer<ChipConfig>>>,
        alert_handler::tests::Tests::new(
            &peripherals.alert_handler,
            virtual_alarm_tests,
            &peripherals.uart0
        )
    );

    hil::time::Alarm::set_alarm_client(virtual_alarm_tests, alert_handler_tests);

    alert_handler_tests.run_tests();
}

#[cfg(feature = "test_aon_timer")]
unsafe fn test_aon_timer(
    peripherals: &'static EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>,
    mux_alarm: &'static MuxAlarm<'static, RvTimer<ChipConfig>>,
) {
    use kernel::hil::time::Alarm;
    use kernel::hil::time::ConvertTicks;
    use kernel::hil::time::Time;

    debug!("Start aon_timer kernel runtime tests!");

    // an Alarm is needed for some of the tests as alert handling works using interrupts
    let virtual_alarm_tests = static_init!(
        VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<ChipConfig>>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_tests.setup();

    let aon_timer_tests = static_init!(
        aon_timer::tests::Tests<VirtualMuxAlarm<'static, RvTimer<ChipConfig>>>,
        aon_timer::tests::Tests::new(
            &peripherals.watchdog,
            &peripherals.rst_mgmt,
            &peripherals.uart0,
            &peripherals.sram_ret,
            virtual_alarm_tests,
        )
    );

    hil::time::Alarm::set_alarm_client(virtual_alarm_tests, aon_timer_tests);

    aon_timer_tests.start_alarm(1000);
}

/// Main function.
///
/// This function is called from the arch crate after some very basic RISC-V
/// setup and RAM initialization.
#[no_mangle]
pub unsafe fn main() {
    #[cfg(test)]
    test_main();

    #[cfg(not(test))]
    {
        let (board_kernel, earlgrey, chip, peripherals) = setup();

        if FLASH_TESTS_ENABLED {
            test_flash(&peripherals.flash_ctrl, &peripherals.uart0);
        }

        let main_loop_cap = create_capability!(capabilities::MainLoopCapability);

        board_kernel.kernel_loop(earlgrey, chip, None::<&kernel::ipc::IPC<0>>, &main_loop_cap);
    }
}

#[cfg(test)]
use kernel::platform::watchdog::WatchDog;

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    unsafe {
        let (board_kernel, earlgrey, _chip, peripherals) = setup();

        BOARD = Some(board_kernel);
        PLATFORM = Some(&earlgrey);
        PERIPHERALS = Some(peripherals);
        MAIN_CAP = Some(&create_capability!(capabilities::MainLoopCapability));

        PLATFORM.map(|p| {
            p.watchdog().setup();
        });

        for test in tests {
            test();
        }
    }

    // Exit QEMU with a return code of 0
    crate::tests::semihost_command_exit_success()
}
