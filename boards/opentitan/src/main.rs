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

use crate::hil::symmetric_encryption::AES128_BLOCK_SIZE;
use crate::otbn::OtbnComponent;
use crate::pinmux_layout::BoardPinmuxLayout;
use capsules_aes_gcm::aes_gcm;
use capsules_core::driver;
use capsules_core::virtualizers::virtual_aes_ccm;
use capsules_core::virtualizers::virtual_alarm::{MuxAlarm, VirtualMuxAlarm};
use capsules_core::virtualizers::virtual_flash::InfoFlashUser;
use capsules_extra::info_flash::InfoFlash;
use capsules_extra::opentitan_alerthandler::AlertHandlerCapsule;
use capsules_extra::opentitan_attestation::Attestation;
#[cfg(not(feature = "qemu"))]
use capsules_extra::opentitan_sysrst::SystemReset;
use capsules_extra::reset_manager::ResetManager;
use core::ptr::{addr_of, from_ref};
#[cfg(feature = "test_alerthandler")]
use earlgrey::alert_handler;
use earlgrey::attestation::Attestation as EarlgreyAttestation;
use earlgrey::chip::EarlGreyDefaultPeripherals;
use earlgrey::chip_config::EarlGreyConfig;
use earlgrey::flash_ctrl;
use earlgrey::pinmux_config::EarlGreyPinmuxConfig;
use lowrisc::timer::RvTimer;

#[cfg(feature = "ffi")]
use capsules_core::virtualizers::timeout_mux::TimeoutMux;
use capsules_core::virtualizers::virtual_timer::{MuxTimer, VirtualTimer};
use kernel::capabilities;
use kernel::component::Component;
use kernel::hil;
use kernel::hil::entropy::Entropy32;
use kernel::hil::flash::HasInfoClient;
#[cfg(not(feature = "test_flash_ctrl"))]
use kernel::hil::hasher::Hasher;
use kernel::hil::i2c::I2CMaster;
use kernel::hil::led::LedHigh;
use kernel::hil::opentitan_attestation::CertificateReader;
use kernel::hil::pattgen::PattGen;
#[cfg(feature = "ffi")]
use kernel::hil::public_key_crypto::ecc::{EllipticCurve, P256, P384};
use kernel::hil::rng::Rng;
use kernel::hil::symmetric_encryption::AES128;
use kernel::hil::usb::UsbController;
use kernel::platform::scheduler_timer::VirtualSchedulerTimer;
use kernel::platform::{KernelResources, SyscallDriverLookup, TbfHeaderFilterDefaultAllow};
use kernel::scheduler::priority::PrioritySched;
use kernel::utilities::registers::interfaces::ReadWriteable;
use kernel::{create_capability, debug, static_init};
#[cfg(feature = "ffi")]
use lowrisc::ffi::cryptolib::{
    ecc::ecdsa::OtCryptoEcdsaP256,
    ecc::ecdsa::OtCryptoEcdsaP384,
    mux::{CryptolibMux, OtbnOperation, OTBN_TIMEOUT_MUX_CHECK_FREQ},
    timeouts::ECDSA_P256_VERIFY_TIMEOUT,
    timeouts::ECDSA_P384_VERIFY_TIMEOUT,
};
#[cfg(not(feature = "qemu"))]
use lowrisc::sysrst_ctrl::SysRstCtrl;
use rv32i::csr;

pub mod io;
mod otbn;
pub mod pinmux_layout;
#[cfg(feature = "ffi")]
pub mod polyfill;
#[cfg(test)]
mod tests;
/// The `earlgrey` chip crate supports multiple targets with slightly different
/// configurations, which are encoded through implementations of the
/// `earlgrey::chip_config::EarlGreyConfig` trait. This type provides different
/// implementations of the `EarlGreyConfig` trait, depending on Cargo's
/// conditional compilation feature flags. If no feature is selected,
/// compilation will error.
enum ChipConfig {}

#[cfg(feature = "qemu")]
impl EarlGreyConfig for ChipConfig {
    const NAME: &'static str = "qemu";

    // Clock frequencies as of https://github.com/lowRISC/opentitan/pull/19479
    const CPU_FREQ: u32 = 24_000_000;
    const PERIPHERAL_FREQ: u32 = 6_000_000;
    const AON_TIMER_FREQ: u32 = 250_000;
    const UART_BAUDRATE: u32 = 115200;
}

#[cfg(feature = "fpga")]
impl EarlGreyConfig for ChipConfig {
    const NAME: &'static str = "fpga";

    // Clock frequencies as of https://github.com/lowRISC/opentitan/pull/19479
    const CPU_FREQ: u32 = 24_000_000;
    const PERIPHERAL_FREQ: u32 = 6_000_000;
    const AON_TIMER_FREQ: u32 = 250_000;
    const UART_BAUDRATE: u32 = 115200;
}

#[cfg(feature = "silicon")]
impl EarlGreyConfig for ChipConfig {
    const NAME: &'static str = "silicon";
    const CPU_FREQ: u32 = 100_000_000;
    const PERIPHERAL_FREQ: u32 = 24_000_000;
    const AON_TIMER_FREQ: u32 = 200_000;
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
#[cfg(feature = "sival")]
pub type EPMPDebugConfig = earlgrey::epmp::EPMPDebugDisable;
#[cfg(not(feature = "sival"))]
pub type EPMPDebugConfig = earlgrey::epmp::EPMPDebugEnable;

// EarlGrey Chip type signature, including generic PMP argument and peripherals
// type:
type EarlGreyChip = earlgrey::chip::EarlGrey<
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
static mut ALARM: Option<&'static MuxAlarm<'static, RvTimer<'static>>> = None;
// Test access to TicKV
#[cfg(not(feature = "test_flash_ctrl"))]
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
        virtual_aes_ccm::VirtualAES128CCM<'static, lowrisc::aes::Aes<'static>>,
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
pub static mut STACK_MEMORY: [u8; 0x6000] = [0; 0x6000];

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
        VirtualMuxAlarm<'static, RvTimer<'static>>,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha256: &'static capsules_extra::oneshot_digest::hash::OneshotSha256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha384: &'static capsules_extra::oneshot_digest::hash::OneshotSha384<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha512: &'static capsules_extra::oneshot_digest::hash::OneshotSha512<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha3_224: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_224<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha3_256: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha3_384: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_384<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_sha3_512: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_512<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_shake128: &'static capsules_extra::oneshot_digest::shake::OneshotShake128<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_shake256: &'static capsules_extra::oneshot_digest::shake::OneshotShake256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_cshake128: &'static capsules_extra::oneshot_digest::cshake::OneshotCshake128<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_cshake256: &'static capsules_extra::oneshot_digest::cshake::OneshotCshake256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_hmac_sha256: &'static capsules_extra::oneshot_digest::hmac::OneshotHmacSha256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_hmac_sha384: &'static capsules_extra::oneshot_digest::hmac::OneshotHmacSha384<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_hmac_sha512: &'static capsules_extra::oneshot_digest::hmac::OneshotHmacSha512<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_kmac128: &'static capsules_extra::oneshot_digest::kmac::OneshotKmac128<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    #[cfg(feature = "ffi")]
    oneshot_kmac256: &'static capsules_extra::oneshot_digest::kmac::OneshotKmac256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    >,
    info_flash: &'static capsules_extra::info_flash::InfoFlash<
        'static,
        InfoFlashUser<'static, earlgrey::flash_ctrl::FlashCtrl<'static>>,
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
            virtual_aes_ccm::VirtualAES128CCM<'static, lowrisc::aes::Aes<'static>>,
        >,
    >,
    pattgen: &'static capsules_extra::pattgen::PattGen<'static, lowrisc::pattgen::PattGen<'static>>,
    usb: &'static capsules_extra::usb::usb_user2::UsbSyscallDriver<
        'static,
        lowrisc::usb::Usb<'static>,
        { lowrisc::usb::MAXIMUM_PACKET_SIZE.get() },
    >,
    #[cfg(not(feature = "qemu"))]
    opentitan_sysrst: &'static SystemReset<'static, SysRstCtrl<'static>>,
    syscall_filter: &'static TbfHeaderFilterDefaultAllow,
    scheduler: &'static PrioritySched,
    scheduler_timer: &'static VirtualSchedulerTimer<VirtualMuxAlarm<'static, RvTimer<'static>>>,
    watchdog: &'static lowrisc::aon_timer::AonTimer<'static>,
    opentitan_alerthandler: &'static AlertHandlerCapsule,
    reset_manager: &'static ResetManager<'static, earlgrey::rstmgr::RstMgr>,
    ipc: kernel::ipc::IPC<{ NUM_PROCS as u8 }>,
    #[allow(dead_code)]
    attestation: &'static Attestation<
        'static,
        EarlgreyAttestation<
            'static,
            InfoFlashUser<'static, earlgrey::flash_ctrl::FlashCtrl<'static>>,
        >,
    >,
    #[cfg(feature = "ffi")]
    ecdsa_p256: &'static capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<
        'static,
        { P256::HASH_LEN },
        { P256::SIG_LEN },
        OtCryptoEcdsaP256<'static, RvTimer<'static>>,
    >,
    #[cfg(feature = "ffi")]
    ecdsa_p384: &'static capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<
        'static,
        { P384::HASH_LEN },
        { P384::SIG_LEN },
        OtCryptoEcdsaP384<'static, RvTimer<'static>>,
    >,
}

/// Mapping of integer syscalls to objects that implement syscalls.
impl SyscallDriverLookup for EarlGrey {
    fn with_driver<F, R>(&self, driver_num: usize, f: F) -> R
    where
        F: FnOnce(Option<&dyn kernel::syscall::SyscallDriver>) -> R,
    {
        match driver_num {
            capsules_core::led::DRIVER_NUM => f(Some(self.led)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA256 => f(Some(self.oneshot_sha256)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA384 => f(Some(self.oneshot_sha384)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA512 => f(Some(self.oneshot_sha512)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_224 => f(Some(self.oneshot_sha3_224)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_256 => f(Some(self.oneshot_sha3_256)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_384 => f(Some(self.oneshot_sha3_384)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_512 => f(Some(self.oneshot_sha3_512)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHAKE128 => f(Some(self.oneshot_shake128)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_SHAKE256 => f(Some(self.oneshot_shake256)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_CSHAKE128 => f(Some(self.oneshot_cshake128)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_CSHAKE256 => f(Some(self.oneshot_cshake256)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_HMAC_SHA256 => {
                f(Some(self.oneshot_hmac_sha256))
            }
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_HMAC_SHA384 => {
                f(Some(self.oneshot_hmac_sha384))
            }
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_HMAC_SHA512 => {
                f(Some(self.oneshot_hmac_sha512))
            }
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_KMAC128 => f(Some(self.oneshot_kmac128)),
            #[cfg(feature = "ffi")]
            capsules_extra::oneshot_digest::DRIVER_NUM_KMAC256 => f(Some(self.oneshot_kmac256)),
            capsules_core::gpio::DRIVER_NUM => f(Some(self.gpio)),
            capsules_core::console::DRIVER_NUM => f(Some(self.console)),
            capsules_core::alarm::DRIVER_NUM => f(Some(self.alarm)),
            capsules_core::low_level_debug::DRIVER_NUM => f(Some(self.lldb)),
            capsules_core::i2c_master::DRIVER_NUM => f(Some(self.i2c_master)),
            capsules_core::spi_controller::DRIVER_NUM => f(Some(self.spi_controller)),
            capsules_core::rng::DRIVER_NUM => f(Some(self.rng)),
            capsules_extra::symmetric_encryption::aes::DRIVER_NUM => f(Some(self.aes)),
            capsules_extra::info_flash::DRIVER_NUMBER => f(Some(self.info_flash)),
            capsules_extra::usb::usb_user2::DRIVER_NUM => f(Some(self.usb)),
            capsules_extra::pattgen::DRIVER_NUM => f(Some(self.pattgen)),
            capsules_extra::opentitan_alerthandler::DRIVER_NUM => {
                f(Some(self.opentitan_alerthandler))
            }
            capsules_extra::reset_manager::DRIVER_NUM => f(Some(self.reset_manager)),
            #[cfg(not(feature = "qemu"))]
            capsules_extra::opentitan_sysrst::DRIVER_NUM => f(Some(self.opentitan_sysrst)),
            capsules_extra::opentitan_attestation::DRIVER_NUM => f(Some(self.attestation)),
            #[cfg(feature = "ffi")]
            capsules_extra::public_key_crypto::asymmetric_crypto::DRIVER_NUM_P256 => {
                f(Some(self.ecdsa_p256))
            }
            #[cfg(feature = "ffi")]
            capsules_extra::public_key_crypto::asymmetric_crypto::DRIVER_NUM_P384 => {
                f(Some(self.ecdsa_p384))
            }
            kernel::ipc::DRIVER_NUM => f(Some(&self.ipc)),
            _ => f(None),
        }
    }
}

impl KernelResources<EarlGreyChip> for EarlGrey {
    type SyscallDriverLookup = Self;
    type SyscallFilter = TbfHeaderFilterDefaultAllow;
    type ProcessFault = ();
    type Scheduler = PrioritySched;
    type SchedulerTimer = VirtualSchedulerTimer<VirtualMuxAlarm<'static, RvTimer<'static>>>;
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

fn get_flash_default_memory_protection_region() -> flash_ctrl::DefaultMemoryProtectionRegion {
    if cfg!(feature = "sival") {
        flash_ctrl::DefaultMemoryProtectionRegion::new()
            .enable_ecc()
            .enable_scramble()
    } else {
        flash_ctrl::DefaultMemoryProtectionRegion::new()
    }
}

fn get_flash_memory_protection_configuration() -> flash_ctrl::MemoryProtectionConfiguration {
    let flash_default_memory_protection_region = get_flash_default_memory_protection_region();

    #[cfg(feature = "unlock_dice_info_pages")]
    let base_memory_protection_config =
        flash_ctrl::MemoryProtectionConfiguration::new(flash_default_memory_protection_region)
            .enable_and_configure_info0_region(flash_ctrl::Info0MemoryProtectionRegionIndex::Bank1(
                flash_ctrl::Info0PageIndex::Index6,
            ))
            .enable_erase()
            .enable_write()
            .enable_read()
            .finalize_region()
            .enable_and_configure_info0_region(flash_ctrl::Info0MemoryProtectionRegionIndex::Bank1(
                flash_ctrl::Info0PageIndex::Index8,
            ))
            .enable_erase()
            .enable_write()
            .enable_read()
            .finalize_region()
            .enable_and_configure_info0_region(flash_ctrl::Info0MemoryProtectionRegionIndex::Bank1(
                flash_ctrl::Info0PageIndex::Index9,
            ))
            .enable_erase()
            .enable_write()
            .enable_read()
            .finalize_region();
    #[cfg(not(feature = "unlock_dice_info_pages"))]
    let base_memory_protection_config =
        flash_ctrl::MemoryProtectionConfiguration::new(flash_default_memory_protection_region);

    #[cfg(feature = "test_flash_ctrl")]
    {
        let page_index_range =
            earlgrey::flash_ctrl::tests::convert_flash_slice_to_page_position_range(unsafe {
                core::slice::from_raw_parts(
                    from_ref(&_sapps),
                    from_ref(&_eapps) as usize - from_ref(&_sapps) as usize,
                )
            })
            .unwrap();

        use earlgrey::flash_ctrl::DataMemoryProtectionRegionBase;
        use earlgrey::flash_ctrl::DataMemoryProtectionRegionSize;

        let memory_protection_page0_base =
            DataMemoryProtectionRegionBase::new(*page_index_range.end());

        const RAW_MEMORY_PROTECTION_PAGE0_SIZE: core::num::NonZeroU16 =
            match core::num::NonZeroU16::new(1) {
                Some(non_zero_u16) => non_zero_u16,
                None => unreachable!(),
            };

        const MEMORY_PROTECTION_PAGE0_SIZE: DataMemoryProtectionRegionSize =
            match DataMemoryProtectionRegionSize::new(RAW_MEMORY_PROTECTION_PAGE0_SIZE) {
                Ok(memory_protection_page0_size) => memory_protection_page0_size,
                Err(()) => unreachable!(),
            };

        base_memory_protection_config
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
    }
    #[cfg(not(feature = "test_flash_ctrl"))]
    {
        // SAFETY: &_stext represents a valid flash address in the host address space.
        let starting_address =
            flash_ctrl::FlashAddress::new_from_host_address(unsafe { from_ref(&_stext) }).unwrap();
        // SAFETY: &_etext represents a valid flash address in the host address space.
        let ending_address =
            flash_ctrl::FlashAddress::new_from_host_address(unsafe { from_ref(&_etext) }).unwrap();

        // Setup flash memory protection for the kernel
        // PANIC: the unwrap panics only if Flash(_stext) < FlashAddress(_etext), which occurs
        // only due to a linker script bug.
        if cfg!(feature = "sival") {
            base_memory_protection_config
                .enable_and_configure_data_region_from_pointers(
                    flash_ctrl::DataMemoryProtectionRegionIndex::Index0,
                    starting_address,
                    ending_address,
                )
                .unwrap()
                .enable_read()
                .enable_scramble()
                .enable_ecc()
                .finalize_region()
                .enable_and_configure_info2_region(
                    flash_ctrl::Info2MemoryProtectionRegionIndex::Bank1(
                        flash_ctrl::Info2PageIndex::Index1,
                    ),
                )
                .enable_read()
                .enable_write()
                .enable_erase()
                .finalize_region()
        } else {
            base_memory_protection_config
                .enable_and_configure_data_region_from_pointers(
                    flash_ctrl::DataMemoryProtectionRegionIndex::Index0,
                    starting_address,
                    ending_address,
                )
                .unwrap()
                .enable_read()
                .finalize_region()
                .enable_and_configure_info2_region(
                    flash_ctrl::Info2MemoryProtectionRegionIndex::Bank1(
                        flash_ctrl::Info2PageIndex::Index1,
                    ),
                )
                .enable_read()
                .enable_write()
                .enable_erase()
                .finalize_region()
        }
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
    let flash_region = earlgrey::epmp::FlashRegion(
        rv32i::pmp::NAPOTRegionSpec::new(
            core::ptr::addr_of!(_sflash),
            core::ptr::addr_of!(_eflash) as usize - core::ptr::addr_of!(_sflash) as usize,
        )
        .unwrap(),
    );
    let ram_region = earlgrey::epmp::RAMRegion(
        rv32i::pmp::NAPOTRegionSpec::new(
            core::ptr::addr_of!(_ssram),
            core::ptr::addr_of!(_esram) as usize - core::ptr::addr_of!(_ssram) as usize,
        )
        .unwrap(),
    );
    let mmio_region = earlgrey::epmp::MMIORegion(
        rv32i::pmp::NAPOTRegionSpec::new(
            0x40000000 as *const u8, // start
            0x10000000,              // size
        )
        .unwrap(),
    );
    let kernel_text_region = earlgrey::epmp::KernelTextRegion(
        rv32i::pmp::TORRegionSpec::new(core::ptr::addr_of!(_stext), core::ptr::addr_of!(_etext))
            .unwrap(),
    );

    #[cfg(feature = "sival")]
    let earlgrey_epmp = earlgrey::epmp::EarlGreyEPMP::new(
        flash_region,
        ram_region,
        mmio_region,
        kernel_text_region,
    )
    .unwrap();
    #[cfg(not(feature = "sival"))]
    let earlgrey_epmp = {
        let debug_region = earlgrey::epmp::RVDMRegion(
            rv32i::pmp::NAPOTRegionSpec::new(
                0x00010000 as *const u8, // start
                0x00001000,              // size
            )
            .unwrap(),
        );
        earlgrey::epmp::EarlGreyEPMP::new_debug(
            flash_region,
            ram_region,
            mmio_region,
            kernel_text_region,
            debug_region,
        )
        .unwrap()
    };

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

    let reset_manager = kernel::static_init!(
        capsules_extra::reset_manager::ResetManager<'static, earlgrey::rstmgr::RstMgr>,
        ResetManager::new(
            &peripherals.rst_mgmt,
            board_kernel.create_grant(
                capsules_extra::reset_manager::DRIVER_NUM,
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
        MuxAlarm<'static, RvTimer>,
        MuxAlarm::new(&peripherals.timer)
    );
    hil::time::Alarm::set_alarm_client(&peripherals.timer, mux_alarm);

    ALARM = Some(mux_alarm);

    // Alarm
    let virtual_alarm_user = static_init!(
        VirtualMuxAlarm<'static, RvTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_user.setup();

    let scheduler_timer_virtual_alarm = static_init!(
        VirtualMuxAlarm<'static, RvTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    scheduler_timer_virtual_alarm.setup();

    let alarm = static_init!(
        capsules_core::alarm::AlarmDriver<'static, VirtualMuxAlarm<'static, RvTimer>>,
        capsules_core::alarm::AlarmDriver::new(
            virtual_alarm_user,
            board_kernel.create_grant(capsules_core::alarm::DRIVER_NUM, &memory_allocation_cap)
        )
    );
    hil::time::Alarm::set_alarm_client(virtual_alarm_user, alarm);

    let scheduler_timer = static_init!(
        VirtualSchedulerTimer<VirtualMuxAlarm<'static, RvTimer<'static>>>,
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

    #[cfg(feature = "ffi")]
    let oneshot_sha256: &'static capsules_extra::oneshot_digest::hash::OneshotSha256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha256<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha256::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA256,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_sha384: &'static capsules_extra::oneshot_digest::hash::OneshotSha384<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha384<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha384::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA384,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_sha512: &'static capsules_extra::oneshot_digest::hash::OneshotSha512<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha512<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha512::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA512,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_sha3_224: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_224<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha3_224<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha3_224::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_224,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_sha3_256: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha3_256<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha3_256::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_256,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_sha3_384: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_384<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha3_384<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha3_384::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_384,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_sha3_512: &'static capsules_extra::oneshot_digest::hash::OneshotSha3_512<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hash::OneshotSha3_512<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hash::OneshotSha3_512::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHA3_512,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_shake128: &'static capsules_extra::oneshot_digest::shake::OneshotShake128<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::shake::OneshotShake128<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::shake::OneshotShake128::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHAKE128,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_shake256: &'static capsules_extra::oneshot_digest::shake::OneshotShake256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::shake::OneshotShake256<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::shake::OneshotShake256::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_SHAKE256,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_cshake128: &'static capsules_extra::oneshot_digest::cshake::OneshotCshake128<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::cshake::OneshotCshake128<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::cshake::OneshotCshake128::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_CSHAKE128,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_cshake256: &'static capsules_extra::oneshot_digest::cshake::OneshotCshake256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::cshake::OneshotCshake256<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::cshake::OneshotCshake256::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_CSHAKE256,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_hmac_sha256: &'static capsules_extra::oneshot_digest::hmac::OneshotHmacSha256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hmac::OneshotHmacSha256<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hmac::OneshotHmacSha256::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_HMAC_SHA256,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_hmac_sha384: &'static capsules_extra::oneshot_digest::hmac::OneshotHmacSha384<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hmac::OneshotHmacSha384<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hmac::OneshotHmacSha384::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_HMAC_SHA384,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_hmac_sha512: &'static capsules_extra::oneshot_digest::hmac::OneshotHmacSha512<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::hmac::OneshotHmacSha512<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::hmac::OneshotHmacSha512::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_HMAC_SHA512,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_kmac128: &'static capsules_extra::oneshot_digest::kmac::OneshotKmac128<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::kmac::OneshotKmac128<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::kmac::OneshotKmac128::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_KMAC128,
                &memory_allocation_cap
            )
        )
    );
    #[cfg(feature = "ffi")]
    let oneshot_kmac256: &'static capsules_extra::oneshot_digest::kmac::OneshotKmac256<
        lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
    > = static_init!(
        capsules_extra::oneshot_digest::kmac::OneshotKmac256<
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
        >,
        capsules_extra::oneshot_digest::kmac::OneshotKmac256::new(
            lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest,
            board_kernel.create_grant(
                capsules_extra::oneshot_digest::DRIVER_NUM_KMAC256,
                &memory_allocation_cap
            )
        )
    );
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
    #[cfg(not(feature = "test_flash_ctrl"))]
    let flash_ctrl_read_buf = static_init!(
        [u8; lowrisc::flash_ctrl::PAGE_SIZE],
        [0; lowrisc::flash_ctrl::PAGE_SIZE]
    );

    #[cfg(not(feature = "test_flash_ctrl"))]
    let page_buffer = static_init!(
        earlgrey::flash_ctrl::RawFlashCtrlPage,
        earlgrey::flash_ctrl::RawFlashCtrlPage::default()
    );

    let mux_flash = components::flash::FlashMuxComponent::new(&peripherals.flash_ctrl).finalize(
        components::flash_mux_component_static!(earlgrey::flash_ctrl::FlashCtrl),
    );
    let mux_info_flash = components::flash::InfoFlashMuxComponent::new(&peripherals.flash_ctrl)
        .finalize(components::info_flash_mux_component_static!(
            earlgrey::flash_ctrl::FlashCtrl
        ));

    // SipHash
    let sip_hash = static_init!(
        capsules_extra::sip_hash::SipHasher24,
        capsules_extra::sip_hash::SipHasher24::new()
    );
    kernel::deferred_call::DeferredCallClient::register(sip_hash);
    SIPHASH = Some(sip_hash);

    #[cfg(not(feature = "test_flash_ctrl"))]
    {
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
        sip_hash.set_client(tickv);
        TICKV = Some(tickv);
    }

    // Info flash multiplexer user endpoint
    let virtual_info_flash = components::flash::InfoFlashUserComponent::new(mux_info_flash)
        .finalize(components::info_flash_user_component_static!(
            earlgrey::flash_ctrl::FlashCtrl
        ));

    // Raw page buffer for info flash driver
    let raw_flash_ctrl_page = static_init!(
        earlgrey::flash_ctrl::RawFlashCtrlPage,
        earlgrey::flash_ctrl::RawFlashCtrlPage::default()
    );
    // Info flash capsule
    let info_flash: &'static InfoFlash<InfoFlashUser<earlgrey::flash_ctrl::FlashCtrl>> = static_init!(
        InfoFlash<InfoFlashUser<'static, earlgrey::flash_ctrl::FlashCtrl>>,
        InfoFlash::new(
            virtual_info_flash,
            board_kernel.create_grant(
                capsules_extra::info_flash::DRIVER_NUMBER,
                &memory_allocation_cap
            ),
            raw_flash_ctrl_page,
        ),
    );
    virtual_info_flash.set_info_client(info_flash);

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
        virtual_aes_ccm::MuxAES128CCM<'static, lowrisc::aes::Aes<'static>>,
        virtual_aes_ccm::MuxAES128CCM::new(&peripherals.aes)
    );
    kernel::deferred_call::DeferredCallClient::register(ccm_mux);
    peripherals.aes.set_client(ccm_mux);

    let ccm_client = components::aes::AesVirtualComponent::new(ccm_mux).finalize(
        components::aes_virtual_component_static!(lowrisc::aes::Aes<'static>),
    );

    let crypt_buf2 = static_init!([u8; CRYPT_SIZE], [0x00; CRYPT_SIZE]);
    let gcm_client = static_init!(
        aes_gcm::Aes128Gcm<
            'static,
            virtual_aes_ccm::VirtualAES128CCM<'static, lowrisc::aes::Aes<'static>>,
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
            virtual_aes_ccm::VirtualAES128CCM<'static, lowrisc::aes::Aes<'static>>,
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

    #[cfg(not(feature = "qemu"))]
    let opentitan_sysrst = {
        let opentitan_sysrst: &'static SystemReset<'static, SysRstCtrl> = static_init!(
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
        opentitan_sysrst
    };

    let ipc = kernel::ipc::IPC::new(
        board_kernel,
        kernel::ipc::DRIVER_NUM,
        &memory_allocation_cap,
    );

    let attestation_virtual_info_flash =
        components::flash::InfoFlashUserComponent::new(mux_info_flash).finalize(
            components::info_flash_user_component_static!(earlgrey::flash_ctrl::FlashCtrl),
        );
    let raw_flash_ctrl_page = static_init!(
        earlgrey::flash_ctrl::RawFlashCtrlPage,
        earlgrey::flash_ctrl::RawFlashCtrlPage::default(),
    );
    let earlgrey_attestation: &'static EarlgreyAttestation<
        InfoFlashUser<earlgrey::flash_ctrl::FlashCtrl>,
    > = static_init!(
        EarlgreyAttestation<'static, InfoFlashUser<earlgrey::flash_ctrl::FlashCtrl>>,
        EarlgreyAttestation::new(attestation_virtual_info_flash, raw_flash_ctrl_page),
    );
    let attestation: &'static Attestation<
        EarlgreyAttestation<InfoFlashUser<earlgrey::flash_ctrl::FlashCtrl>>,
    > = static_init!(
        Attestation<EarlgreyAttestation<InfoFlashUser<earlgrey::flash_ctrl::FlashCtrl>>>,
        Attestation::new(
            earlgrey_attestation,
            board_kernel.create_grant(
                driver::NUM::OpenTitanAttestation as usize,
                &memory_allocation_cap
            ),
        )
    );
    attestation_virtual_info_flash.set_info_client(earlgrey_attestation);
    earlgrey_attestation.set_client(attestation);

    // Data structures for scheduling OTBN operations with timeouts
    let virtual_alarm_user: &'static VirtualMuxAlarm<'static, RvTimer> = static_init!(
        VirtualMuxAlarm<'static, RvTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_user.setup();
    let mux_timer: &'static MuxTimer<'static, RvTimer> = static_init!(
        MuxTimer<'static, RvTimer>,
        MuxTimer::new(virtual_alarm_user)
    );
    hil::time::Alarm::set_alarm_client(virtual_alarm_user, mux_timer);
    let otbn_timer: &'static VirtualTimer<'static, RvTimer> =
        static_init!(VirtualTimer<'static, RvTimer>, VirtualTimer::new(mux_timer),);
    otbn_timer.setup();

    // Asymmetric crypto
    #[cfg(feature = "ffi")]
    let (ecdsa_p256, ecdsa_p384) = {
        let timeout_mux = static_init!(
            TimeoutMux<'static, RvTimer, OtbnOperation<'static, RvTimer>>,
            TimeoutMux::new(otbn_timer, OTBN_TIMEOUT_MUX_CHECK_FREQ),
        );
        kernel::hil::time::Timer::set_timer_client(otbn_timer, timeout_mux);
        timeout_mux.setup();
        let cryptolib_mux = static_init!(
            CryptolibMux<'static, RvTimer>,
            CryptolibMux::new(earlgrey::otbn::OTBN_BASE, timeout_mux),
        );

        // ECDSA P-256
        let cryptolib_ecdsa_p256: &'static OtCryptoEcdsaP256<'static, RvTimer<'static>> = static_init!(
            OtCryptoEcdsaP256<'static, RvTimer<'static>>,
            OtCryptoEcdsaP256::new(cryptolib_mux, ECDSA_P256_VERIFY_TIMEOUT.into()),
        );
        cryptolib_ecdsa_p256.set_self_ref();
        let public_key_buf: &'static mut [u8; 2 * P256::COORD_LEN] =
            static_init!([u8; 2 * P256::COORD_LEN], [0u8; 2 * P256::COORD_LEN],);
        // Initialize the public key buffer for ECDSA P-256 driver.
        //
        // PANIC: The implementation of `import_public_key` for
        // `OtCryptoEcdsaP256` never returns `Err`.
        kernel::hil::public_key_crypto::keys::PubKeyMut::import_public_key(
            cryptolib_ecdsa_p256,
            public_key_buf,
        )
        .unwrap();
        let p256_hash_buf: &'static mut [u8; P256::HASH_LEN] =
            static_init!([u8; P256::HASH_LEN], [0u8; P256::HASH_LEN],);
        let p256_signature_buf: &'static mut [u8; P256::SIG_LEN] =
            static_init!([u8; P256::SIG_LEN], [0u8; P256::SIG_LEN],);
        let ecdsa_p256: &'static capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<
                    'static,
                { P256::HASH_LEN },
                { P256::SIG_LEN },
                OtCryptoEcdsaP256<'static, RvTimer>,
                > = static_init!(
            capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<
                'static,
            { P256::HASH_LEN },
            { P256::SIG_LEN },
                OtCryptoEcdsaP256<'static, RvTimer>,
            >,
            capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto::new(
                cryptolib_ecdsa_p256,
                p256_hash_buf,
                p256_signature_buf,
                board_kernel.create_grant(
                    capsules_extra::public_key_crypto::asymmetric_crypto::DRIVER_NUM_P256,
                    &memory_allocation_cap
                ),
            ),
        );
        // ECDSA P-384
        let cryptolib_ecdsa_p384: &'static OtCryptoEcdsaP384<'static, RvTimer<'static>> = static_init!(
            OtCryptoEcdsaP384<'static, RvTimer<'static>>,
            OtCryptoEcdsaP384::new(cryptolib_mux, ECDSA_P384_VERIFY_TIMEOUT.into()),
        );
        cryptolib_ecdsa_p384.set_self_ref();
        let public_key_buf: &'static mut [u8; 2 * P384::COORD_LEN] =
            static_init!([u8; 2 * P384::COORD_LEN], [0u8; 2 * P384::COORD_LEN],);
        // Initialize the public key buffer for ECDSA P-384 driver.
        //
        // PANIC: The implementation of `import_public_key` for
        // `OtCryptoEcdsaP384` never returns `Err`.
        kernel::hil::public_key_crypto::keys::PubKeyMut::import_public_key(
            cryptolib_ecdsa_p384,
            public_key_buf,
        )
        .unwrap();
        let p384_hash_buf: &'static mut [u8; P384::HASH_LEN] =
            static_init!([u8; P384::HASH_LEN], [0u8; P384::HASH_LEN],);
        let p384_signature_buf: &'static mut [u8; P384::SIG_LEN] =
            static_init!([u8; P384::SIG_LEN], [0u8; P384::SIG_LEN],);
        let ecdsa_p384: &'static capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<
                    'static,
                { P384::HASH_LEN },
                { P384::SIG_LEN },
                OtCryptoEcdsaP384<'static, RvTimer>,
                > = static_init!(
            capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto<
                'static,
            { P384::HASH_LEN },
            { P384::SIG_LEN },
                OtCryptoEcdsaP384<'static, RvTimer>,
            >,
            capsules_extra::public_key_crypto::asymmetric_crypto::AsymmetricCrypto::new(
                cryptolib_ecdsa_p384,
                p384_hash_buf,
                p384_signature_buf,
                board_kernel.create_grant(
                    capsules_extra::public_key_crypto::asymmetric_crypto::DRIVER_NUM_P384,
                    &memory_allocation_cap
                ),
            ),
        );
        // This must be set before the test block, not after, otherwise it
        // interferes with the test.
        kernel::hil::public_key_crypto::ecc::EcdsaP256::set_verify_client(
            cryptolib_ecdsa_p256,
            ecdsa_p256,
        );
        kernel::hil::public_key_crypto::ecc::EcdsaP384::set_verify_client(
            cryptolib_ecdsa_p384,
            ecdsa_p384,
        );
        #[cfg(feature = "test_cryptolib")]
        {
            let p256_hash_buf: &'static mut [u8; P256::HASH_LEN] =
                static_init!([u8; P256::HASH_LEN], [0u8; P256::HASH_LEN],);
            let p256_signature_buf: &'static mut [u8; P256::SIG_LEN] =
                static_init!([u8; P256::SIG_LEN], [0u8; P256::SIG_LEN],);
            let p256_hash_buf_2: &'static mut [u8; P256::HASH_LEN] =
                static_init!([u8; P256::HASH_LEN], [0u8; P256::HASH_LEN],);
            let p256_signature_buf_2: &'static mut [u8; P256::SIG_LEN] =
                static_init!([u8; P256::SIG_LEN], [0u8; P256::SIG_LEN],);
            let p256_test_client: &'static lowrisc::ffi::cryptolib::ecc::ecdsa::tests::EcdsaP256TestClient = static_init!(
                lowrisc::ffi::cryptolib::ecc::ecdsa::tests::EcdsaP256TestClient,
                lowrisc::ffi::cryptolib::ecc::ecdsa::tests::EcdsaP256TestClient::new(p256_hash_buf, p256_signature_buf, p256_hash_buf_2, p256_signature_buf_2),
            );
            kernel::hil::public_key_crypto::ecc::EcdsaP256::set_verify_client(
                cryptolib_ecdsa_p256,
                p256_test_client,
            );
            let p256_pub_key_buf: &'static mut [u8; 2 * P256::COORD_LEN] =
                static_init!([u8; 2 * P256::COORD_LEN], [0u8; 2 * P256::COORD_LEN],);

            let p384_hash_buf: &'static mut [u8; P384::HASH_LEN] =
                static_init!([u8; P384::HASH_LEN], [0u8; P384::HASH_LEN],);
            let p384_signature_buf: &'static mut [u8; P384::SIG_LEN] =
                static_init!([u8; P384::SIG_LEN], [0u8; P384::SIG_LEN],);
            let p384_hash_buf_2: &'static mut [u8; P384::HASH_LEN] =
                static_init!([u8; P384::HASH_LEN], [0u8; P384::HASH_LEN],);
            let p384_signature_buf_2: &'static mut [u8; P384::SIG_LEN] =
                static_init!([u8; P384::SIG_LEN], [0u8; P384::SIG_LEN],);
            let p384_test_client: &'static lowrisc::ffi::cryptolib::ecc::ecdsa::tests::EcdsaP384TestClient = static_init!(
                lowrisc::ffi::cryptolib::ecc::ecdsa::tests::EcdsaP384TestClient,
                lowrisc::ffi::cryptolib::ecc::ecdsa::tests::EcdsaP384TestClient::new(p384_hash_buf, p384_signature_buf, p384_hash_buf_2, p384_signature_buf_2),
            );
            kernel::hil::public_key_crypto::ecc::EcdsaP384::set_verify_client(
                cryptolib_ecdsa_p384,
                p384_test_client,
            );
            let p384_pub_key_buf: &'static mut [u8; 2 * P384::COORD_LEN] =
                static_init!([u8; 2 * P384::COORD_LEN], [0u8; 2 * P384::COORD_LEN],);

            // Test P-256 verify
            lowrisc::ffi::cryptolib::ecc::ecdsa::tests::test_ecdsa_p256_verify(
                cryptolib_ecdsa_p256,
                p256_test_client,
                p256_pub_key_buf,
            );
            // Test P-384 verify
            lowrisc::ffi::cryptolib::ecc::ecdsa::tests::test_ecdsa_p384_verify(
                cryptolib_ecdsa_p384,
                p384_test_client,
                p384_pub_key_buf,
            );
        }
        (ecdsa_p256, ecdsa_p384)
    };

    let earlgrey = static_init!(
        EarlGrey,
        EarlGrey {
            gpio,
            led,
            console,
            alarm,
            info_flash,
            rng,
            lldb,
            i2c_master,
            spi_controller,
            aes,
            usb,
            #[cfg(feature = "ffi")]
            oneshot_sha256,
            #[cfg(feature = "ffi")]
            oneshot_sha384,
            #[cfg(feature = "ffi")]
            oneshot_sha512,
            #[cfg(feature = "ffi")]
            oneshot_sha3_224,
            #[cfg(feature = "ffi")]
            oneshot_sha3_256,
            #[cfg(feature = "ffi")]
            oneshot_sha3_384,
            #[cfg(feature = "ffi")]
            oneshot_sha3_512,
            #[cfg(feature = "ffi")]
            oneshot_shake128,
            #[cfg(feature = "ffi")]
            oneshot_shake256,
            #[cfg(feature = "ffi")]
            oneshot_cshake128,
            #[cfg(feature = "ffi")]
            oneshot_cshake256,
            #[cfg(feature = "ffi")]
            oneshot_hmac_sha256,
            #[cfg(feature = "ffi")]
            oneshot_hmac_sha384,
            #[cfg(feature = "ffi")]
            oneshot_hmac_sha512,
            #[cfg(feature = "ffi")]
            oneshot_kmac128,
            #[cfg(feature = "ffi")]
            oneshot_kmac256,
            pattgen,
            syscall_filter,
            scheduler,
            scheduler_timer,
            #[cfg(not(feature = "qemu"))]
            opentitan_sysrst,
            watchdog,
            reset_manager,
            opentitan_alerthandler: alert_handler_capsule,
            ipc,
            attestation,
            #[cfg(feature = "ffi")]
            ecdsa_p256,
            #[cfg(feature = "ffi")]
            ecdsa_p384,
        }
    );

    // If the feature is selected, run flash tests before setting the flash clients to the
    // multiplexers.
    #[cfg(feature = "test_flash_ctrl")]
    test_flash(&peripherals.flash_ctrl, &peripherals.uart0);

    hil::flash::HasClient::set_client(&peripherals.flash_ctrl, mux_flash);
    hil::flash::HasInfoClient::set_info_client(&peripherals.flash_ctrl, mux_info_flash);

    // OTP tests (currently broken on sival)
    #[cfg(all(not(feature = "sival"), feature = "test_otp"))]
    {
        lowrisc::otp::tests::run_all(&peripherals.otp);
    }

    // Pattern generation tests
    #[cfg(feature = "test_pattgen")]
    {
        let pattgen_test = static_init!(
            lowrisc::pattgen::tests::PattGenTest,
            lowrisc::pattgen::tests::PattGenTest::new(&peripherals.pattgen),
        );
        lowrisc::pattgen::tests::run_all(pattgen_test);
    }

    // when running with ROM, reset reason is cleared from HW and stored inside RetentionRAM
    #[cfg(not(feature = "qemu"))]
    {
        let reset_reason = earlgrey::rstmgr::RstMgr::get_rr_from_rram(&peripherals.sram_ret);
        earlgrey.reset_manager.startup();
        earlgrey.reset_manager.populate_reset_reason(reset_reason);
    }

    /* TESTs */
    #[cfg(all(not(feature = "qemu"), feature = "test_resetmanager"))]
    capsules_extra::reset_manager::test::test_software_reset(
        &peripherals.sram_ret,
        earlgrey.reset_manager,
        core::ptr::addr_of!(_sflash) as usize,
        core::ptr::addr_of!(_eflash) as usize,
    );

    #[cfg(all(not(feature = "qemu"), feature = "test_sysrst_ctrl"))]
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
        &mut *core::ptr::addr_of_mut!(PROCESSES),
        &FAULT_RESPONSE,
        &process_mgmt_cap,
    )
    .unwrap_or_else(|err| {
        debug!("Error loading processes!");
        debug!("{:?}", err);
    });

    #[cfg(feature = "test_alerthandler")]
    {
        test_alerthandler(peripherals, mux_alarm);
    }

    #[cfg(all(not(feature = "qemu"), feature = "test_sram_ret"))]
    peripherals
        .sram_ret
        .test(&peripherals.rst_mgmt, &peripherals.uart0);

    #[cfg(all(not(feature = "qemu"), feature = "test_aon_timer"))]
    {
        peripherals.watchdog.test(
            &peripherals.uart0,
            &peripherals.sram_ret,
            &peripherals.sram_ret,
        );
        test_aon_timer(peripherals, mux_alarm);
    }

    #[cfg(all(not(feature = "qemu"), feature = "test_rv_timer"))]
    {
        peripherals.timer.test(
            &peripherals.uart0,
            &peripherals.sram_ret,
            &peripherals.sram_ret,
        );
        test_rv_timer(mux_alarm);
    }

    #[cfg(all(not(feature = "qemu"), feature = "test_clkmgr"))]
    {
        peripherals.clkmgr.run_tests();
    }

    debug!("OpenTitan initialisation complete. Entering main loop");

    (board_kernel, earlgrey, chip, peripherals)
}

#[cfg(all(not(feature = "qemu"), feature = "test_sysrst_ctrl"))]
fn test_sysrst_ctrl(peripherals: &EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>) {
    pinmux_layout::prepare_wiring_sysrst_ctrl_tests();
    lowrisc::sysrst_ctrl::tests::test_all(
        &peripherals.sysreset,
        &peripherals.gpio_port[7],
        &peripherals.gpio_port[2],
        &peripherals.gpio_port[20],
    );
}

#[cfg(feature = "test_flash_ctrl")]
fn test_flash(
    flash_ctrl: &'static earlgrey::flash_ctrl::FlashCtrl,
    uart: &'static earlgrey::uart::Uart<'static>,
) {
    use kernel::hil::flash::Flash as FlashHIL;
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
                from_ref(&_sapps),
                from_ref(&_eapps) as usize - from_ref(&_sapps) as usize,
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
    mux_alarm: &'static MuxAlarm<'static, RvTimer>,
) {
    debug!("Starting AlertHandler test...");
    // an Alarm is needed for some of the tests as alert handling works using interrupts
    let virtual_alarm_tests = static_init!(
        VirtualMuxAlarm<'static, RvTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_tests.setup();

    let alert_handler_tests = static_init!(
        alert_handler::tests::Tests<VirtualMuxAlarm<'static, RvTimer>>,
        alert_handler::tests::Tests::new(
            &peripherals.alert_handler,
            virtual_alarm_tests,
            &peripherals.uart0
        )
    );

    hil::time::Alarm::set_alarm_client(virtual_alarm_tests, alert_handler_tests);

    alert_handler_tests.run_tests();
    debug!("Finished AlertHandler tests. Everything is alright!");
}

#[cfg(feature = "test_aon_timer")]
unsafe fn test_aon_timer(
    peripherals: &'static EarlGreyDefaultPeripherals<ChipConfig, BoardPinmuxLayout>,
    mux_alarm: &'static MuxAlarm<'static, RvTimer>,
) {
    use lowrisc::aon_timer;

    debug!("Start aon_timer kernel runtime tests!");

    // an Alarm is needed for some of the tests as alert handling works using interrupts
    let virtual_alarm_tests = static_init!(
        VirtualMuxAlarm<'static, RvTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_tests.setup();

    let aon_timer_tests = static_init!(
        aon_timer::tests::Tests<VirtualMuxAlarm<'static, RvTimer>>,
        aon_timer::tests::Tests::new(&peripherals.watchdog, virtual_alarm_tests,)
    );

    hil::time::Alarm::set_alarm_client(virtual_alarm_tests, aon_timer_tests);

    aon_timer_tests.start_alarm(1000);
}

#[cfg(feature = "test_rv_timer")]
unsafe fn test_rv_timer(mux_alarm: &'static MuxAlarm<'static, RvTimer>) {
    use lowrisc::timer;

    debug!("Start rv_timer kernel runtime tests!");

    // an Alarm is needed for some of the tests as alert handling works using interrupts
    let virtual_alarm_tests = static_init!(
        VirtualMuxAlarm<'static, RvTimer>,
        VirtualMuxAlarm::new(mux_alarm)
    );
    virtual_alarm_tests.setup();

    let rv_timer_tests = static_init!(
        timer::tests::Tests<VirtualMuxAlarm<'static, RvTimer>>,
        timer::tests::Tests::new(virtual_alarm_tests)
    );

    hil::time::Alarm::set_alarm_client(virtual_alarm_tests, rv_timer_tests);

    rv_timer_tests.start_alarm(1000);
}

/// Main function.
///
/// This function is called from the arch crate after some very basic RISC-V
/// setup and RAM initialization.
#[no_mangle]
#[allow(clippy::missing_safety_doc)]
pub unsafe fn main() {
    #[cfg(test)]
    test_main();

    #[cfg(not(test))]
    {
        let (board_kernel, earlgrey, chip, _peripherals) = setup();

        let main_loop_cap = create_capability!(capabilities::MainLoopCapability);

        board_kernel.kernel_loop(earlgrey, chip, Some(&earlgrey.ipc), &main_loop_cap);
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
