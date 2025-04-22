// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! High-level setup and interrupt mapping for the chip.
use core::fmt::{Display, Write};
use core::marker::PhantomData;
use core::ptr::addr_of;
use kernel::platform::chip::{Chip, InterruptService};
use kernel::utilities::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::utilities::StaticRef;
use lowrisc::timer::RvTimer;
use rv32i::csr::{mcause, mie::mie, mtvec::mtvec, CSR};
use rv32i::pmp::{PMPUserMPU, TORUserPMP};
use rv32i::syscall::SysCall;
use {core::num::NonZeroU32, kernel::utilities::helpers::create_non_zero_u32};

use crate::alert_handler::{AlertClass, LocalAlertFlags};
use crate::alert_handler::{AlertFlags, AlertHandler};
use crate::aon_timer::AON_TIMER;
use crate::chip_config::EarlGreyConfig;
use crate::pinmux_config::EarlGreyPinmuxConfig;
use crate::plic::Plic;
use crate::plic::PLIC;
use crate::pwrmgr::PwrMgrInterrupt;
use crate::registers::top_earlgrey::AlertId;
use crate::registers::top_earlgrey::PlicIrqId;
use crate::registers::top_earlgrey::RV_TIMER_BASE_ADDR;
use crate::registers::top_earlgrey::SYSRST_CTRL_AON_BASE_ADDR;
use crate::rstmgr::RstMgr;
use crate::rv_core_ibex::{IBEX_EXTERNAL_NMI_MCAUSE, RV_CORE_IBEX};
use lowrisc::aon_timer::AonTimerInterrupt;

pub struct EarlGrey<
    'a,
    const MPU_REGIONS: usize,
    I: InterruptService + 'a,
    CFG: EarlGreyConfig + 'static,
    PINMUX: EarlGreyPinmuxConfig,
    PMP: TORUserPMP<{ MPU_REGIONS }> + Display + 'static,
> {
    userspace_kernel_boundary: SysCall,
    pub mpu: PMPUserMPU<MPU_REGIONS, PMP>,
    pub(super) plic: &'a Plic,
    pwrmgr: crate::pwrmgr::PwrMgr,
    plic_interrupt_service: &'a I,
    _cfg: PhantomData<CFG>,
    _pinmux: PhantomData<PINMUX>,
}

pub struct EarlGreyDefaultPeripherals<'a, CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig> {
    pub sram_ret: Option<crate::sram_ret::SramCtrl>,
    pub adc_ctrl: Option<crate::adc_ctrl::AdcCtrl<'a>>,
    pub aes: Option<lowrisc::aes::Aes<'a>>,
    pub csrng: Option<lowrisc::csrng::CsRng<'a>>,
    pub edn0: Option<lowrisc::edn::Edn<'a>>,
    pub edn1: Option<lowrisc::edn::Edn<'a>>,
    pub entropy_src: Option<lowrisc::entropy_src::EntropySrc<'a>>,
    pub hmac: Option<lowrisc::hmac::Hmac<'a>>,
    pub keymgr: Option<lowrisc::keymgr::Keymgr<'a>>,
    pub kmac: Option<lowrisc::kmac::Kmac<'a>>,
    pub clkmgr: Option<crate::clkmgr::Clkmgr>,
    pub usb: Option<lowrisc::usb::Usb<'a>>,
    pub uart0: Option<lowrisc::uart::Uart<'a>>,
    pub uart1: Option<lowrisc::uart::Uart<'a>>,
    pub uart2: Option<lowrisc::uart::Uart<'a>>,
    pub uart3: Option<lowrisc::uart::Uart<'a>>,
    pub otbn: Option<lowrisc::otbn::Otbn<'a>>,
    pub otp: Option<lowrisc::otp::Otp>,
    pub gpio_port: Option<crate::gpio::Port<'a>>,
    pub i2c0: Option<lowrisc::i2c::I2c<'a>>,
    pub i2c1: Option<lowrisc::i2c::I2c<'a>>,
    pub i2c2: Option<lowrisc::i2c::I2c<'a>>,
    pub spi_host0: Option<lowrisc::spi_host::SpiHost<'a>>,
    pub spi_host1: Option<lowrisc::spi_host::SpiHost<'a>>,
    pub spi_device: Option<lowrisc::spi_device::SpiDevice<'a>>,
    pub flash_ctrl: Option<crate::flash_ctrl::FlashCtrl<'a>>,
    pub watchdog: Option<&'a lowrisc::aon_timer::AonTimer<'static>>,
    pub sensor_ctrl: Option<crate::sensor_ctrl::SensorCtrl<'a>>,
    pub sysreset: Option<lowrisc::sysrst_ctrl::SysRstCtrl<'a>>,
    pub timer: Option<RvTimer<'static>>,
    pub alert_handler: Option<AlertHandler>,
    pub pattgen: Option<lowrisc::pattgen::PattGen<'a>>,
    pub rst_mgmt: Option<RstMgr>,
    _cfg: PhantomData<CFG>,
    _pinmux: PhantomData<PINMUX>,
}

pub enum PeripheralConfig {
    /// Enable peripheral-specific driver and interrupts.
    Enabled,
    /// Enable interrupts, but disable peripheral-specific driver. Useful when
    /// nonstandard drivers that rely on the peripheral interrupt, such as
    /// OpenTitan cryptolib APIs.
    InterruptsOnly,
    /// Disable peripheral-specific driver and interrupts.
    Disabled,
}

/// Peripheral configuration for GPIO, which enables interrupts on a per-pin
/// basis, but enabling the driver is a global decision.
pub struct GpioPeripheralConfig {
    pub driver_enabled: bool,
    pub interrupts_enabled: [bool; crate::gpio::GPIO_PINS],
}

/// Default GPIO peripheral configuration
pub const DEFAULT_GPIO_CONFIG: GpioPeripheralConfig = GpioPeripheralConfig {
    driver_enabled: true,
    interrupts_enabled: [true; crate::gpio::GPIO_PINS],
};

/// Type that defines which peripheral drivers should be included in the kernel
/// configuration.
pub struct EarlgreyPeripheralConfig {
    pub sram_ret: PeripheralConfig,
    pub adc_ctrl: PeripheralConfig,
    pub aes: PeripheralConfig,
    pub csrng: PeripheralConfig,
    pub edn0: PeripheralConfig,
    pub edn1: PeripheralConfig,
    pub entropy_src: PeripheralConfig,
    pub hmac: PeripheralConfig,
    pub keymgr: PeripheralConfig,
    pub kmac: PeripheralConfig,
    pub clkmgr: PeripheralConfig,
    pub usb: PeripheralConfig,
    pub uart0: PeripheralConfig,
    pub uart1: PeripheralConfig,
    pub uart2: PeripheralConfig,
    pub uart3: PeripheralConfig,
    pub otbn: PeripheralConfig,
    pub otp: PeripheralConfig,
    pub gpio_port: GpioPeripheralConfig,
    pub i2c0: PeripheralConfig,
    pub i2c1: PeripheralConfig,
    pub i2c2: PeripheralConfig,
    pub spi_host0: PeripheralConfig,
    pub spi_host1: PeripheralConfig,
    pub spi_device: PeripheralConfig,
    pub flash_ctrl: PeripheralConfig,
    pub rng: PeripheralConfig,
    pub watchdog: PeripheralConfig,
    pub sensor_ctrl: PeripheralConfig,
    pub sysreset: PeripheralConfig,
    pub timer: PeripheralConfig,
    pub alert_handler: PeripheralConfig,
    pub pattgen: PeripheralConfig,
    pub rst_mgmt: PeripheralConfig,
}

// Macro that:
// - If the `EarlgreyPeripheralConfig` says to include the driver, initializes the
//   driver according to `$init`.
// - If the `EarlgreyPeripheralConfig` says _not_ to include the driver, sets the
//   peripheral to `None`.
macro_rules! conditional_init {
    {$peripheral_config:expr, $peripheral:ident, $init:expr} => {
        match $peripheral_config.$peripheral {
            PeripheralConfig::Enabled => Some($init),
            _ => None,
        }
    }
}

impl<CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig>
    EarlGreyDefaultPeripherals<'_, CFG, PINMUX>
{
    #[allow(static_mut_refs)]
    pub unsafe fn new(
        flash_memory_protection_configuration: crate::flash_ctrl::MemoryProtectionConfiguration,
        peripheral_config: EarlgreyPeripheralConfig,
    ) -> Self {
        AON_TIMER.set_clk_freq(CFG::AON_TIMER_FREQ);
        Self {
            sram_ret: conditional_init!(
                &peripheral_config,
                sram_ret,
                crate::sram_ret::SramCtrl::new()
            ),
            adc_ctrl: conditional_init!(
                &peripheral_config,
                adc_ctrl,
                lowrisc::adc_ctrl::AdcCtrl::new(crate::adc_ctrl::ADC_CTRL_BASE)
            ),
            aes: conditional_init!(
                &peripheral_config,
                aes,
                lowrisc::aes::Aes::new(crate::aes::AES_BASE)
            ),
            csrng: conditional_init!(
                &peripheral_config,
                csrng,
                lowrisc::csrng::CsRng::new(crate::csrng::CSRNG_BASE)
            ),
            edn0: conditional_init!(
                &peripheral_config,
                edn0,
                lowrisc::edn::Edn::new(crate::edn::EDN0_BASE)
            ),
            edn1: conditional_init!(
                &peripheral_config,
                edn1,
                lowrisc::edn::Edn::new(crate::edn::EDN1_BASE)
            ),
            entropy_src: conditional_init!(
                &peripheral_config,
                entropy_src,
                lowrisc::entropy_src::EntropySrc::new(crate::entropy_src::ENTROPY_SRC_BASE,)
            ),
            hmac: conditional_init!(
                &peripheral_config,
                hmac,
                lowrisc::hmac::Hmac::new(crate::hmac::HMAC0_BASE)
            ),
            keymgr: conditional_init!(
                &peripheral_config,
                keymgr,
                lowrisc::keymgr::Keymgr::new(crate::keymgr::KEYMGR_BASE)
            ),
            kmac: conditional_init!(
                &peripheral_config,
                kmac,
                lowrisc::kmac::Kmac::new(crate::kmac::KMAC_BASE)
            ),
            clkmgr: conditional_init!(&peripheral_config, clkmgr, crate::clkmgr::Clkmgr::new()),
            usb: conditional_init!(
                &peripheral_config,
                usb,
                lowrisc::usb::Usb::new(crate::usbdev::USB0_BASE)
            ),
            uart0: conditional_init!(
                &peripheral_config,
                uart0,
                lowrisc::uart::Uart::new(crate::uart::UART0_BASE, CFG::PERIPHERAL_FREQ)
            ),
            uart1: conditional_init!(
                &peripheral_config,
                uart1,
                lowrisc::uart::Uart::new(crate::uart::UART1_BASE, CFG::PERIPHERAL_FREQ)
            ),
            uart2: conditional_init!(
                &peripheral_config,
                uart2,
                lowrisc::uart::Uart::new(crate::uart::UART2_BASE, CFG::PERIPHERAL_FREQ)
            ),
            uart3: conditional_init!(
                &peripheral_config,
                uart3,
                lowrisc::uart::Uart::new(crate::uart::UART3_BASE, CFG::PERIPHERAL_FREQ)
            ),
            otbn: conditional_init!(
                &peripheral_config,
                otbn,
                lowrisc::otbn::Otbn::new(crate::otbn::OTBN_BASE)
            ),
            otp: conditional_init!(
                &peripheral_config,
                otp,
                lowrisc::otp::Otp::new(crate::otp::OTP_BASE)
            ),
            gpio_port: if peripheral_config.gpio_port.driver_enabled {
                Some(crate::gpio::Port::new::<PINMUX>())
            } else {
                None
            },
            i2c0: conditional_init!(
                &peripheral_config,
                i2c0,
                lowrisc::i2c::I2c::new(crate::i2c::I2C0_BASE, (1 / CFG::CPU_FREQ) * 1000 * 1000)
            ),
            i2c1: conditional_init!(
                &peripheral_config,
                i2c1,
                lowrisc::i2c::I2c::new(crate::i2c::I2C1_BASE, (1 / CFG::CPU_FREQ) * 1000 * 1000)
            ),
            i2c2: conditional_init!(
                &peripheral_config,
                i2c2,
                lowrisc::i2c::I2c::new(crate::i2c::I2C2_BASE, (1 / CFG::CPU_FREQ) * 1000 * 1000)
            ),
            spi_host0: conditional_init!(
                &peripheral_config,
                spi_host0,
                lowrisc::spi_host::SpiHost::new(crate::spi_host::SPIHOST0_BASE, CFG::CPU_FREQ,)
            ),
            spi_host1: conditional_init!(
                &peripheral_config,
                spi_host1,
                lowrisc::spi_host::SpiHost::new(crate::spi_host::SPIHOST1_BASE, CFG::CPU_FREQ,)
            ),
            spi_device: conditional_init!(
                &peripheral_config,
                spi_device,
                lowrisc::spi_device::SpiDevice::new(crate::spi_device::SPIDEVICE_BASE)
            ),
            flash_ctrl: conditional_init!(
                &peripheral_config,
                flash_ctrl,
                crate::flash_ctrl::FlashCtrl::new(flash_memory_protection_configuration)
            ),
            watchdog: conditional_init!(&peripheral_config, watchdog, &*addr_of!(AON_TIMER)),
            sensor_ctrl: conditional_init!(
                &peripheral_config,
                sensor_ctrl,
                crate::sensor_ctrl::SensorCtrl::new()
            ),
            sysreset: conditional_init!(
                &peripheral_config,
                sysreset,
                lowrisc::sysrst_ctrl::SysRstCtrl::new(SYSRST_CTRL_AON_BASE_ADDR)
            ),
            timer: conditional_init!(
                &peripheral_config,
                timer,
                RvTimer::new(
                    unsafe {
                        StaticRef::new(
                            RV_TIMER_BASE_ADDR
                                as *const lowrisc::registers::rv_timer_regs::RvTimerRegisters,
                        )
                    },
                    CFG::PERIPHERAL_FREQ,
                )
            ),
            alert_handler: conditional_init!(
                &peripheral_config,
                alert_handler,
                AlertHandler::new()
            ),
            pattgen: conditional_init!(
                &peripheral_config,
                pattgen,
                lowrisc::pattgen::PattGen::new(crate::pattgen::PATTGEN_BASE)
            ),
            rst_mgmt: conditional_init!(&peripheral_config, rst_mgmt, RstMgr::new()),
            _cfg: PhantomData,
            _pinmux: PhantomData,
        }
    }

    pub fn init(&'static self) {
        self.aes
            .as_ref()
            .map(|aes| kernel::deferred_call::DeferredCallClient::register(aes));
        self.uart0
            .as_ref()
            .map(|uart0| kernel::deferred_call::DeferredCallClient::register(uart0));
        self.uart1
            .as_ref()
            .map(|uart1| kernel::deferred_call::DeferredCallClient::register(uart1));
        self.uart2
            .as_ref()
            .map(|uart2| kernel::deferred_call::DeferredCallClient::register(uart2));
        self.uart3
            .as_ref()
            .map(|uart3| kernel::deferred_call::DeferredCallClient::register(uart3));

        // Recommended value by documentation
        const INTEGRITY_CHECK_PERIOD: u32 = 0x3_FFFF;
        // Recommended value by documentation
        const CONSISTENCY_CHECK_PERIOD: u32 = 0x3_FFFF;
        // Recommended value by documentation is at least 100_000.
        const CHECK_TIMEOUT: NonZeroU32 = create_non_zero_u32(100_000);

        self.otp.as_ref().map(|otp| {
            otp.init(
                INTEGRITY_CHECK_PERIOD,
                CONSISTENCY_CHECK_PERIOD,
                Some(CHECK_TIMEOUT),
            )
            .expect("Failed to initialize OTP");
        });
    }

    #[inline]
    pub fn handle_alert_interrupt(&self, class: AlertClass) {
        if let Some(alert_handler) = &self.alert_handler {
            // retrieve alert state for this class and (try to) stop HW escalation
            let class_state = alert_handler.class_state(class);
            alert_handler.clear_esclation(class);

            // HANDLE LOCAL ALERTS
            // iterate multiple times through the local alerts, only handled once each alert (mark the alerts that have been handled in `handled_alerts` and don't reconsider them).
            let mut handled_alerts = LocalAlertFlags::empty();
            loop {
                // check which local alerts are still set
                let local_alerts = alert_handler.snapshot_local_alert_causes();

                // iterate through all of the set local alerts that have not been handled since the start of the interrupt
                let anything_new = handled_alerts.for_each_new(&local_alerts, |alert| {
                    // send each alert to `alert_handler`
                    let should_clear = alert_handler.handle_alert(alert, class_state);
                    if should_clear {
                        alert_handler.clear_local_alert_cause(alert);
                    }
                });

                // if no new alerts have been raised consider that all of the alert have been handled
                if !anything_new {
                    break;
                }
            }

            // HANDLE ALERTS FROM ALL PERIPHEREALS
            // alerts could be triggered while inside the interrupt handler,
            // alerts flags could remain set until the underlying issue is solved
            let mut handled_alerts = AlertFlags::empty();
            loop {
                // snapshot alert flags
                let alerts = alert_handler.snapshot_alert_causes();
                // iterate over current alert flags that have not previously been handled (and marked as such in `handled_alerts`)
                let anything_new =
                    handled_alerts.for_each_new(&alerts, |alert| self.handle_alert(alert));

                // break the loop when no new alert flags have been raised
                if !anything_new {
                    break;
                }
            }

            // clear interrupt flag
            alert_handler.clear_interrupt(class);
        }
    }

    fn handle_alert(&self, alert: AlertId) {
        let should_clear = match alert {
            // If UART is not present, no reason to hold onto the alert, so default to `true`.
            AlertId::Uart0FatalFault => self
                .uart0
                .as_ref()
                .map_or(true, |uart0| uart0.handle_alert()),
            _ => panic!("alert with no handle was triggered"),
        };
        self.alert_handler.as_ref().map(|alert_handler| {
            alert_handler.notify_userspace(alert);
            if should_clear {
                alert_handler.clear_alert_cause(alert);
            }
        });
    }
}

impl<
        'a,
        const MPU_REGIONS: usize,
        I: InterruptService + 'a,
        CFG: EarlGreyConfig,
        PINMUX: EarlGreyPinmuxConfig,
        PMP: TORUserPMP<{ MPU_REGIONS }> + Display + 'static,
    > EarlGrey<'a, MPU_REGIONS, I, CFG, PINMUX, PMP>
{
    pub unsafe fn new(plic_interrupt_service: &'a I, pmp: PMP) -> Self {
        Self {
            userspace_kernel_boundary: SysCall::new(),
            mpu: PMPUserMPU::new(pmp),
            plic: &*addr_of!(PLIC),
            pwrmgr: crate::pwrmgr::PwrMgr::new(crate::pwrmgr::PWRMGR_BASE),
            plic_interrupt_service,
            _cfg: PhantomData,
            _pinmux: PhantomData,
        }
    }

    unsafe fn handle_plic_interrupts(&self) {
        while let Some(interrupt) = self.plic.get_saved_interrupts() {
            // Handle pwrmgr interrupts
            if let Ok(PlicIrqId::PwrmgrAonWakeup) = PlicIrqId::try_from(interrupt) {
                self.pwrmgr.handle_interrupt(PwrMgrInterrupt::AonWakeup);
                self.check_until_true_or_interrupt(|| self.pwrmgr.check_clock_propagation(), None);
            }
            if !self.plic_interrupt_service.service_interrupt(interrupt) {
                // PANIC: Using the interrupt handler in
                // `crate:handle_interrupts.rs`, this panic should be
                // unreachable, because every interrupt is handled.
                panic!("Unknown interrupt: {}", interrupt);
            }

            self.atomic(|| {
                self.plic.complete(interrupt);
            });
        }
    }

    /// Run a function in an interruptable loop.
    ///
    /// The function will run until it returns true, an interrupt occurs or if
    /// `max_tries` is not `None` and that limit is reached.
    /// If the function returns true this call will also return true. If an
    /// interrupt occurs or `max_tries` is reached this call will return false.
    fn check_until_true_or_interrupt<F>(&self, f: F, max_tries: Option<usize>) -> bool
    where
        F: Fn() -> bool,
    {
        match max_tries {
            Some(t) => {
                for _i in 0..t {
                    if self.has_pending_interrupts() {
                        return false;
                    }
                    if f() {
                        return true;
                    }
                }
            }
            None => {
                while !self.has_pending_interrupts() {
                    if f() {
                        return true;
                    }
                }
            }
        }

        false
    }
}

impl<
        'a,
        const MPU_REGIONS: usize,
        I: InterruptService + 'a,
        CFG: EarlGreyConfig,
        PINMUX: EarlGreyPinmuxConfig,
        PMP: TORUserPMP<{ MPU_REGIONS }> + Display + 'static,
    > kernel::platform::chip::Chip for EarlGrey<'a, MPU_REGIONS, I, CFG, PINMUX, PMP>
{
    type MPU = PMPUserMPU<MPU_REGIONS, PMP>;
    type UserspaceKernelBoundary = SysCall;

    fn mpu(&self) -> &Self::MPU {
        &self.mpu
    }

    fn userspace_kernel_boundary(&self) -> &SysCall {
        &self.userspace_kernel_boundary
    }

    fn service_pending_interrupts(&self) {
        loop {
            if self.plic.get_saved_interrupts().is_some() {
                unsafe {
                    self.handle_plic_interrupts();
                }
            }

            if self.plic.get_saved_interrupts().is_none() {
                break;
            }
        }

        // Re-enable all MIE interrupts that we care about. Since we looped
        // until we handled them all, we can re-enable all of them.
        CSR.mie.modify(mie::mext::SET + mie::mtimer::CLEAR);
        self.plic.enable_all();
    }

    fn has_pending_interrupts(&self) -> bool {
        self.plic.get_saved_interrupts().is_some()
    }

    fn sleep(&self) {
        unsafe {
            self.pwrmgr.enable_low_power();
            self.check_until_true_or_interrupt(|| self.pwrmgr.check_clock_propagation(), None);
            rv32i::support::wfi();
        }
    }

    unsafe fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        rv32i::support::atomic(f)
    }

    unsafe fn print_state(&self, writer: &mut dyn Write) {
        let _ = writer.write_fmt(format_args!(
            "\r\n---| OpenTitan Earlgrey configuration for {} |---",
            CFG::NAME
        ));
        rv32i::print_riscv_state(writer);
        let _ = writer.write_fmt(format_args!("{}", self.mpu.pmp));
    }
}

fn handle_exception(exception: mcause::Exception) {
    match exception {
        mcause::Exception::UserEnvCall | mcause::Exception::SupervisorEnvCall => (),

        // Breakpoints occur from the tests running on hardware
        mcause::Exception::Breakpoint => loop {
            unsafe { rv32i::support::wfi() }
        },

        mcause::Exception::InstructionMisaligned
        | mcause::Exception::InstructionFault
        | mcause::Exception::IllegalInstruction
        | mcause::Exception::LoadMisaligned
        | mcause::Exception::LoadFault
        | mcause::Exception::StoreMisaligned
        | mcause::Exception::StoreFault
        | mcause::Exception::MachineEnvCall
        | mcause::Exception::InstructionPageFault
        | mcause::Exception::LoadPageFault
        | mcause::Exception::StorePageFault
        | mcause::Exception::Unknown => {
            panic!("fatal exception: {:?}: {:#x}", exception, CSR.mtval.get());
        }
    }
}

#[allow(static_mut_refs)]
unsafe fn handle_interrupt(intr: mcause::Interrupt) {
    match intr {
        mcause::Interrupt::UserSoft
        | mcause::Interrupt::UserTimer
        | mcause::Interrupt::UserExternal => {
            panic!("unexpected user-mode interrupt");
        }
        mcause::Interrupt::SupervisorExternal
        | mcause::Interrupt::SupervisorTimer
        | mcause::Interrupt::SupervisorSoft => {
            panic!("unexpected supervisor-mode interrupt");
        }

        mcause::Interrupt::MachineSoft => {
            CSR.mie.modify(mie::msoft::CLEAR);
        }
        mcause::Interrupt::MachineTimer => {
            CSR.mie.modify(mie::mtimer::CLEAR);
        }
        mcause::Interrupt::MachineExternal => {
            // We received an interrupt, disable interrupts while we handle them
            CSR.mie.modify(mie::mext::CLEAR);

            // Claim the interrupt, unwrap() as we know an interrupt exists
            // Once claimed this interrupt won't fire until it's completed
            // NOTE: The interrupt is no longer pending in the PLIC
            loop {
                let interrupt = (*addr_of!(PLIC)).next_pending();

                match interrupt {
                    Some(irq) => {
                        // Safe as interrupts are disabled
                        (*addr_of!(PLIC)).save_interrupt(irq);
                    }
                    None => {
                        // Enable generic interrupts
                        CSR.mie.modify(mie::mext::SET);
                        break;
                    }
                }
            }
        }

        mcause::Interrupt::Unknown => {
            match CSR.mcause.get() {
                // Both external NMI sourcess for Earl Grey are from the AON
                // timer, and in the production ROM in upstream OpenTitan's
                // rom.c, only the watchdog NMI is enabled.
                //
                // Update (Feb 2025): In earlgrey_1.0.0, this property still
                // holds, but the ROM code to set it is now at
                // opentitan:sw/device/silicon_creator/rom/rom_start.S:143.
                IBEX_EXTERNAL_NMI_MCAUSE => {
                    AON_TIMER.handle_interrupt(AonTimerInterrupt::AonWdogTimerBark);
                    RV_CORE_IBEX.clear_wdog_nmi();
                }
                _ => panic!("interrupt of unknown cause"),
            }
        }
    }
}

/// Trap handler for board/chip specific code.
///
/// For the Ibex this gets called when an interrupt occurs while the chip is
/// in kernel mode.
#[export_name = "_start_trap_rust_from_kernel"]
pub unsafe extern "C" fn start_trap_rust() {
    match mcause::Trap::from(CSR.mcause.extract()) {
        mcause::Trap::Interrupt(interrupt) => {
            handle_interrupt(interrupt);
        }
        mcause::Trap::Exception(exception) => {
            handle_exception(exception);
        }
    }
}

/// Function that gets called if an interrupt occurs while an app was running.
///
/// mcause is passed in, and this function should correctly handle disabling the
/// interrupt that fired so that it does not trigger again.
#[export_name = "_disable_interrupt_trap_rust_from_app"]
pub unsafe extern "C" fn disable_interrupt_trap_handler(mcause_val: u32) {
    match mcause::Trap::from(mcause_val as usize) {
        mcause::Trap::Interrupt(interrupt) => {
            handle_interrupt(interrupt);
        }
        _ => {
            panic!("unexpected non-interrupt\n");
        }
    }
}

#[cfg(feature = "separate_irq_stack")]
#[export_name = "_check_mcause_for_nmi"]
pub unsafe extern "C" fn check_mcause_for_nmi(mcause_val: u32) -> bool {
    match mcause_val as usize {
        IBEX_EXTERNAL_NMI_MCAUSE | IBEX_LOAD_INTEGRITY_ERROR_NMI_MCAUSE => true,
        _ => false,
    }
}

pub unsafe fn configure_trap_handler() {
    // The common _start_trap handler uses mscratch to determine
    // whether we are executing kernel or process code. Set to `0` to
    // indicate we're in the kernel right now.
    CSR.mscratch.set(0);

    // The Ibex CPU does not support non-vectored trap entries.
    CSR.mtvec.write(
        mtvec::trap_addr.val(_earlgrey_start_trap_vectored as usize >> 2) + mtvec::mode::Vectored,
    );
}

// Mock implementation for crate tests that does not include the section
// specifier, as the test will not use our linker script, and the host
// compilation environment may not allow the section name.
#[cfg(not(any(doc, all(target_arch = "riscv32", target_os = "none"))))]
pub extern "C" fn _earlgrey_start_trap_vectored() {
    use core::hint::unreachable_unchecked;
    unsafe {
        unreachable_unchecked();
    }
}

#[cfg(any(doc, all(target_arch = "riscv32", target_os = "none")))]
extern "C" {
    pub fn _earlgrey_start_trap_vectored();
}

#[cfg(any(doc, all(target_arch = "riscv32", target_os = "none")))]
// According to the Ibex user manual:
// [NMI] has interrupt ID 31, i.e., it has the highest priority of all
// interrupts and the core jumps to the trap-handler base address (in
// mtvec) plus 0x7C to handle the NMI.
//
// Below are 32 (non-compressed) jumps to cover the entire possible
// range of vectored traps.
core::arch::global_asm!(
    "
            .section .riscv.trap_vectored, \"ax\"
            .globl _start_trap_vectored
          _earlgrey_start_trap_vectored:

            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
            j {start_trap}
    ",
    start_trap = sym rv32i::_start_trap,
);
