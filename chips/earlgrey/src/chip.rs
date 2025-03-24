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
use crate::interrupts;
use crate::pinmux_config::EarlGreyPinmuxConfig;
use crate::plic::Plic;
use crate::plic::PLIC;
use crate::registers::top_earlgrey::AlertId;
use crate::registers::top_earlgrey::RV_TIMER_BASE_ADDR;
#[cfg(not(feature = "qemu"))]
use crate::registers::top_earlgrey::SYSRST_CTRL_AON_BASE_ADDR;
use crate::rstmgr::RstMgr;
use crate::rv_core_ibex::{IBEX_EXTERNAL_NMI_MCAUSE, RV_CORE_IBEX};

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
    plic: &'a Plic,
    pwrmgr: crate::pwrmgr::PwrMgr,
    plic_interrupt_service: &'a I,
    _cfg: PhantomData<CFG>,
    _pinmux: PhantomData<PINMUX>,
}

pub struct EarlGreyDefaultPeripherals<'a, CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig> {
    #[cfg(not(feature = "qemu"))]
    pub sram_ret: crate::sram_ret::SramCtrl,
    pub aes: lowrisc::aes::Aes<'a>,
    pub hmac: lowrisc::hmac::Hmac<'a>,
    pub clkmgr: crate::clkmgr::Clkmgr,
    pub usb: lowrisc::usb::Usb<'a>,
    pub uart0: lowrisc::uart::Uart<'a>,
    pub otbn: lowrisc::otbn::Otbn<'a>,
    pub otp: lowrisc::otp::Otp,
    pub gpio_port: crate::gpio::Port<'a>,
    pub i2c0: lowrisc::i2c::I2c<'a>,
    pub spi_host0: lowrisc::spi_host::SpiHost<'a>,
    pub spi_host1: lowrisc::spi_host::SpiHost<'a>,
    pub flash_ctrl: crate::flash_ctrl::FlashCtrl<'a>,
    pub rng: lowrisc::csrng::CsRng<'a>,
    pub watchdog: &'a lowrisc::aon_timer::AonTimer<'static>,
    #[cfg(not(feature = "qemu"))]
    pub sysreset: lowrisc::sysrst_ctrl::SysRstCtrl<'a>,
    pub timer: RvTimer<'static>,
    pub alert_handler: AlertHandler,
    pub pattgen: lowrisc::pattgen::PattGen<'a>,
    pub rst_mgmt: RstMgr,
    _cfg: PhantomData<CFG>,
    _pinmux: PhantomData<PINMUX>,
}

impl<'a, CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig>
    EarlGreyDefaultPeripherals<'a, CFG, PINMUX>
{
    pub unsafe fn new(
        flash_memory_protection_configuration: crate::flash_ctrl::MemoryProtectionConfiguration,
    ) -> Self {
        AON_TIMER.set_clk_freq(CFG::AON_TIMER_FREQ);
        Self {
            #[cfg(not(feature = "qemu"))]
            sram_ret: crate::sram_ret::SramCtrl::new(),
            aes: lowrisc::aes::Aes::new(crate::aes::AES_BASE),
            hmac: lowrisc::hmac::Hmac::new(crate::hmac::HMAC0_BASE),
            clkmgr: crate::clkmgr::Clkmgr::new(),
            usb: lowrisc::usb::Usb::new(crate::usbdev::USB0_BASE),
            uart0: lowrisc::uart::Uart::new(crate::uart::UART0_BASE, CFG::PERIPHERAL_FREQ),
            otbn: lowrisc::otbn::Otbn::new(crate::otbn::OTBN_BASE),
            otp: lowrisc::otp::Otp::new(crate::otp::OTP_BASE),
            gpio_port: crate::gpio::Port::new::<PINMUX>(),
            i2c0: lowrisc::i2c::I2c::new(crate::i2c::I2C0_BASE, (1 / CFG::CPU_FREQ) * 1000 * 1000),
            spi_host0: lowrisc::spi_host::SpiHost::new(
                crate::spi_host::SPIHOST0_BASE,
                CFG::CPU_FREQ,
            ),
            spi_host1: lowrisc::spi_host::SpiHost::new(
                crate::spi_host::SPIHOST1_BASE,
                CFG::CPU_FREQ,
            ),
            flash_ctrl: crate::flash_ctrl::FlashCtrl::new(flash_memory_protection_configuration),
            rng: lowrisc::csrng::CsRng::new(crate::csrng::CSRNG_BASE),
            watchdog: &*addr_of!(AON_TIMER),
            #[cfg(not(feature = "qemu"))]
            sysreset: lowrisc::sysrst_ctrl::SysRstCtrl::new(SYSRST_CTRL_AON_BASE_ADDR),
            timer: RvTimer::new(
                unsafe {
                    StaticRef::new(
                        RV_TIMER_BASE_ADDR
                            as *const lowrisc::registers::rv_timer_regs::RvTimerRegisters,
                    )
                },
                CFG::PERIPHERAL_FREQ,
            ),
            alert_handler: AlertHandler::new(),
            pattgen: lowrisc::pattgen::PattGen::new(crate::pattgen::PATTGEN_BASE),
            rst_mgmt: RstMgr::new(),
            _cfg: PhantomData,
            _pinmux: PhantomData,
        }
    }

    pub fn init(&'static self) {
        kernel::deferred_call::DeferredCallClient::register(&self.aes);
        kernel::deferred_call::DeferredCallClient::register(&self.uart0);
        // Recommended value by documentation
        const INTEGRITY_CHECK_PERIOD: u32 = 0x3_FFFF;
        // Recommended value by documentation
        const CONSISTENCY_CHECK_PERIOD: u32 = 0x3_FFFF;
        // Recommended value by documentation is at least 100_000.
        const CHECK_TIMEOUT: NonZeroU32 = create_non_zero_u32(100_000);

        self.otp
            .init(
                INTEGRITY_CHECK_PERIOD,
                CONSISTENCY_CHECK_PERIOD,
                Some(CHECK_TIMEOUT),
            )
            .expect("Failed to initialize OTP");
    }

    #[inline]
    pub fn handle_alert_interrupt(&self, class: AlertClass) {
        // retrieve alert state for this class and (try to) stop HW escalation
        let class_state = self.alert_handler.class_state(class);
        self.alert_handler.clear_esclation(class);

        // HANDLE LOCAL ALERTS
        // iterate multiple times through the local alerts, only handled once each alert (mark the alerts that have been handled in `handled_alerts` and don't reconsider them).
        let mut handled_alerts = LocalAlertFlags::empty();
        loop {
            // check which local alerts are still set
            let local_alerts = self.alert_handler.snapshot_local_alert_causes();

            // iterate through all of the set local alerts that have not been handled since the start of the interrupt
            let anything_new = handled_alerts.for_each_new(&local_alerts, |alert| {
                // send each alert to `alert_handler`
                let should_clear = self.alert_handler.handle_alert(alert, class_state);
                if should_clear {
                    self.alert_handler.clear_local_alert_cause(alert);
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
            let alerts = self.alert_handler.snapshot_alert_causes();
            // iterate over current alert flags that have not previously been handled (and marked as such in `handled_alerts`)
            let anything_new =
                handled_alerts.for_each_new(&alerts, |alert| self.handle_alert(alert));

            // break the loop when no new alert flags have been raised
            if !anything_new {
                break;
            }
        }

        // clear interrupt flag
        self.alert_handler.clear_interrupt(class);
    }

    fn handle_alert(&self, alert: AlertId) {
        let should_clear = match alert {
            AlertId::Uart0FatalFault => self.uart0.handle_alert(),
            _ => panic!("alert with no handle was triggered"),
        };
        self.alert_handler.notify_userspace(alert);
        if should_clear {
            self.alert_handler.clear_alert_cause(alert);
        }
    }
}

impl<'a, CFG: EarlGreyConfig, PINMUX: EarlGreyPinmuxConfig> InterruptService
    for EarlGreyDefaultPeripherals<'a, CFG, PINMUX>
{
    unsafe fn service_interrupt(&self, interrupt: u32) -> bool {
        match interrupt {
            interrupts::UART0_TX_WATERMARK..=interrupts::UART0_RX_PARITYERR => {
                self.uart0.handle_interrupt();
            }
            int_pin @ interrupts::GPIO_PIN0..=interrupts::GPIO_PIN31 => {
                let pin = &self.gpio_port[(int_pin - interrupts::GPIO_PIN0) as usize];
                pin.handle_interrupt();
            }
            interrupts::HMAC_HMACDONE..=interrupts::HMAC_HMACERR => {
                self.hmac.handle_interrupt();
            }
            raw_usb_interrupt @ interrupts::USBDEV_PKTRECEIVED..=interrupts::USBDEV_LINKOUTERR => {
                // PANIC: raw_usb_interrupt is a valid interrupt because of the match arm
                // CAST: u32 == usize on RV32I
                let usb_interrupt =
                    lowrisc::usb::UsbInterrupt::try_from_usize(raw_usb_interrupt as usize).unwrap();
                self.usb.handle_interrupt(usb_interrupt);
            }
            interrupts::FLASHCTRL_PROGEMPTY => {
                // Since writing is done on chunks of FIFO depth level, this interrupt is useless.
                return false;
            }
            interrupts::FLASHCTRL_PROGLVL => {
                // Since writing is done on chunks of FIFO depth level, this interrupt is useless.
                return false;
            }
            interrupts::FLASHCTRL_RDFULL => {
                // Since reading is done on chunks of FIFO depth level, this interrupt is useless.
                return false;
            }
            interrupts::FLASHCTRL_RDLVL => {
                // Since reading is done on chunks of FIFO depth level, this interrupt is useless.
                return false;
            }
            interrupts::FLASHCTRL_OPDONE => {
                self.flash_ctrl.handle_operation_done();
            }
            interrupts::FLASHCTRL_CORRERR => {
                // This interrupt may only occur due to a driver bug.
                return false;
            }
            interrupts::I2C0_FMTWATERMARK..=interrupts::I2C0_HOSTTIMEOUT => {
                self.i2c0.handle_interrupt()
            }
            interrupts::OTBN_DONE => self.otbn.handle_interrupt(),
            interrupts::CSRNG_CSCMDREQDONE..=interrupts::CSRNG_CSFATALERR => {
                self.rng.handle_interrupt()
            }
            interrupts::SPIHOST0_ERROR..=interrupts::SPIHOST0_SPIEVENT => {
                self.spi_host0.handle_interrupt()
            }
            interrupts::SPIHOST1_ERROR..=interrupts::SPIHOST1_SPIEVENT => {
                self.spi_host1.handle_interrupt()
            }
            #[cfg(not(feature = "qemu"))]
            interrupts::SYSRST_CTRL_AON_SYSRST_CTRL => self.sysreset.handle_interrupt(),
            interrupts::ALERTHANDLER_CLASSA => {
                self.handle_alert_interrupt(AlertClass::ClassA);
            }
            interrupts::ALERTHANDLER_CLASSB => {
                self.handle_alert_interrupt(AlertClass::ClassB);
            }
            interrupts::ALERTHANDLER_CLASSC => {
                self.handle_alert_interrupt(AlertClass::ClassC);
            }
            interrupts::ALERTHANDLER_CLASSD => {
                self.handle_alert_interrupt(AlertClass::ClassD);
            }
            interrupts::RVTIMERTIMEREXPIRED0_0 => self.timer.service_interrupt(),
            raw_pattgen_interrupt @ interrupts::PATTGENDONECH0..=interrupts::PATTGENDONECH1 => {
                // PANIC: raw_pattgen_interrupt is a valid interrupt because of the match arm
                // CAST: u32 == usize on RV32I
                let pattgen_interrupt =
                    lowrisc::pattgen::PattgenInterrupt::try_from(raw_pattgen_interrupt as usize)
                        .unwrap();
                self.pattgen.handle_interrupt(pattgen_interrupt);
            }
            interrupts::AON_TIMER_AON_WKUP_TIMER_EXPIRED
                ..=interrupts::AON_TIMER_AON_WDOG_TIMER_BARK => self.watchdog.handle_interrupt(),
            _ => return false,
        }
        true
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

    pub unsafe fn enable_plic_interrupts(&self) {
        self.plic.disable_all();
        self.plic.enable_all();
    }

    unsafe fn handle_plic_interrupts(&self) {
        while let Some(interrupt) = self.plic.get_saved_interrupts() {
            match interrupt {
                interrupts::PWRMGRAONWAKEUP => {
                    self.pwrmgr.handle_interrupt();
                    self.check_until_true_or_interrupt(
                        || self.pwrmgr.check_clock_propagation(),
                        None,
                    );
                }
                _ => {
                    if interrupt >= interrupts::HMAC_HMACDONE
                        && interrupt <= interrupts::HMAC_HMACERR
                    {
                        // Claim the interrupt before we handle it.
                        // Currently the interrupt has been claimed but not completed.
                        // This means that if the interrupt re-asserts we will loose the
                        // re-assertion. Generally this isn't a problem, but some of the
                        // interrupt handlers expect that interrupts could occur.
                        // For example the HMAC interrupt handler will write data to the
                        // HMAC buffer. We then rely on an interrupt triggering when that
                        // buffer becomes empty. This can happen while we are still in the
                        // interrupt handler. To ensure we don't loose the interrupt we
                        // claim it here.
                        // In order to stop an interrupt loop, we first disable the
                        // interrupt. `service_pending_interrupts()` will re-enable
                        // interrupts once they are all handled.
                        self.atomic(|| {
                            // Safe as interrupts are disabled
                            self.plic.disable(interrupt);
                            self.plic.complete(interrupt);
                        });
                    }
                    if !self.plic_interrupt_service.service_interrupt(interrupt) {
                        panic!("Unknown interrupt: {}", interrupt);
                    }
                }
            }

            match interrupt {
                interrupts::HMAC_HMACDONE..=interrupts::HMAC_HMACERR => {}
                _ => {
                    self.atomic(|| {
                        self.plic.complete(interrupt);
                    });
                }
            }
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
                let interrupt = PLIC.next_pending();

                match interrupt {
                    Some(irq) => {
                        // Safe as interrupts are disabled
                        PLIC.save_interrupt(irq);
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
                IBEX_EXTERNAL_NMI_MCAUSE => {
                    AON_TIMER.handle_interrupt();
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

pub unsafe fn configure_trap_handler() {
    // The Ibex CPU does not support non-vectored trap entries.
    CSR.mtvec
        .write(mtvec::trap_addr.val(_start_trap_vectored as usize >> 2) + mtvec::mode::Vectored)
}

// Mock implementation for crate tests that does not include the section
// specifier, as the test will not use our linker script, and the host
// compilation environment may not allow the section name.
#[cfg(not(all(target_arch = "riscv32", target_os = "none")))]
pub extern "C" fn _start_trap_vectored() {
    use core::hint::unreachable_unchecked;
    unsafe {
        unreachable_unchecked();
    }
}

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
extern "C" {
    pub fn _start_trap_vectored();
}

#[cfg(all(target_arch = "riscv32", target_os = "none"))]
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
          _start_trap_vectored:

            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
            j _start_trap
        "
);
