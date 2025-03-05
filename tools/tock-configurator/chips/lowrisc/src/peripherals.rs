// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use parse::Ident as _;

use std::cell::RefCell;
use std::rc::Rc;

/// Set of drivers supported on Earlgrey, which may or may not have a 1:1
/// relationship with a particular hardware peripheral.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Drivers {
    aes: [Rc<crate::aes::Aes>; 1],
    alert_handlers: [Rc<crate::alert_handler::AlertHandler>; 1],
    flash_memory_protection_configuration:
        Rc<crate::flash_memory_protection::FlashMemoryProtectionConfiguration>,
    flashes: [Rc<crate::flash::FlashCtrl>; 1],
    gpios: [Rc<crate::gpio::GpioPort>; 1],
    hmacs: [Rc<crate::hmac::Hmac>; 1],
    i2cs: [Rc<crate::i2c::I2c>; 1],
    pattgens: [Rc<crate::pattgen::Pattgen>; 1],
    reset_managers: [Rc<crate::reset_manager::ResetManager>; 1],
    rngs: [Rc<crate::rng::CsRng>; 1],
    spis: [Rc<crate::spi::SpiHost>; 1],
    system_reset_controllers: [Rc<crate::system_reset_controller::SystemResetController>; 1],
    timers: [Rc<crate::timer::RvTimer>; 1],
    uarts: [Rc<crate::uart::Uart>; 1],
    usbs: [Rc<crate::usb::Usb>; 1],
    attestations: [Rc<crate::attestation::EarlgreyAttestation<crate::flash::FlashCtrl>>; 1],
    oneshot_sha256s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha256>; 1],
    oneshot_sha384s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha384>; 1],
    oneshot_sha512s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha512>; 1],
    oneshot_sha3_224s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_224>; 1],
    oneshot_sha3_256s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_256>; 1],
    oneshot_sha3_384s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_384>; 1],
    oneshot_sha3_512s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_512>; 1],
    oneshot_shake128s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotShake128>; 1],
    oneshot_shake256s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotShake256>; 1],
    oneshot_cshake128s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotCshake128>; 1],
    oneshot_cshake256s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotCshake256>; 1],
    oneshot_hmac_sha256s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha256>; 1],
    oneshot_hmac_sha384s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha384>; 1],
    oneshot_hmac_sha512s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha512>; 1],
    oneshot_kmac128s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotKmac128>; 1],
    oneshot_kmac256s: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotKmac256>; 1],
    p256s: [Rc<crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP256<crate::timer::RvTimer>>; 1],
    p384s: [Rc<crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP384<crate::timer::RvTimer>>; 1],

    /// Peripheral configuration. This is only used during board generation and
    /// does not need to be deserialized from the JSON.
    #[serde(skip)]
    peripheral_config: Rc<RefCell<EarlgreyPeripheralConfig>>,
}

impl Drivers {
    pub fn new(peripheral_config: Rc<RefCell<EarlgreyPeripheralConfig>>) -> Self {
        let timer = Rc::new(crate::timer::RvTimer::new());
        let flash = Rc::new(crate::flash::FlashCtrl::new(Rc::clone(&peripheral_config)));
        let mux_alarm = Rc::new(parse::peripherals::timer::MuxAlarm::new(timer.clone()));
        let timeout_mux = Rc::new(parse::timeout_mux::TimeoutMux::new(mux_alarm));
        let cryptolib_mux = Rc::new(crate::ffi::cryptolib::mux::CryptolibMux::new(timeout_mux));
        Self {
            aes: [Rc::new(crate::aes::Aes::new())],
            alert_handlers: [Rc::new(crate::alert_handler::AlertHandler::new())],
            flash_memory_protection_configuration: Rc::new(
                super::flash_memory_protection::FlashMemoryProtectionConfiguration::new(),
            ),
            flashes: [flash.clone()],
            gpios: [Rc::new(crate::gpio::GpioPort::new())],
            hmacs: [Rc::new(crate::hmac::Hmac::new())],
            i2cs: [Rc::new(crate::i2c::I2c::new())],
            pattgens: [Rc::new(crate::pattgen::Pattgen::new())],
            reset_managers: [Rc::new(crate::reset_manager::ResetManager::new())],
            rngs: [Rc::new(crate::rng::CsRng::new())],
            system_reset_controllers: [Rc::new(
                crate::system_reset_controller::SystemResetController::new(),
            )],
            spis: [Rc::new(crate::spi::SpiHost::new())],
            timers: [timer.clone()],
            uarts: [Rc::new(crate::uart::Uart::new())],
            usbs: [Rc::new(crate::usb::Usb::new())],
            attestations: [Rc::new(crate::attestation::EarlgreyAttestation::new(
                flash.clone(),
            ))],
            oneshot_sha256s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha256::new(),
            )],
            oneshot_sha384s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha384::new(),
            )],
            oneshot_sha512s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha512::new(),
            )],
            oneshot_sha3_224s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_224::new(),
            )],
            oneshot_sha3_256s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_256::new(),
            )],
            oneshot_sha3_384s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_384::new(),
            )],
            oneshot_sha3_512s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_512::new(),
            )],
            oneshot_shake128s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotShake128::new(),
            )],
            oneshot_shake256s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotShake256::new(),
            )],
            oneshot_cshake128s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotCshake128::new(),
            )],
            oneshot_cshake256s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotCshake256::new(),
            )],
            oneshot_hmac_sha256s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha256::new(),
            )],
            oneshot_hmac_sha384s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha384::new(),
            )],
            oneshot_hmac_sha512s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha512::new(),
            )],
            oneshot_kmac128s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotKmac128::new(),
            )],
            oneshot_kmac256s: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotKmac256::new(),
            )],
            p256s: [Rc::new(
                crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP256::new(cryptolib_mux.clone()),
            )],
            p384s: [Rc::new(
                crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP384::new(cryptolib_mux.clone()),
            )],
            peripheral_config,
        }
    }
}

impl Default for Drivers {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl parse::Component for Drivers {
    fn ty(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            earlgrey::chip::EarlGreyDefaultPeripherals<
                'static,
                ChipConfig,
                crate::pinmux_layout::BoardPinmuxLayout,
            >
        ))
    }

    fn before_init(&self) -> Option<parse::proc_macro2::TokenStream> {
        // Paste in the driver configuration
        let peripheral_config = &*self.peripheral_config.borrow();
        let sram_ret_enable = quote_enable(peripheral_config.get_enabled(Peripheral::SramRet));
        let adc_ctrl_enable = quote_enable(peripheral_config.get_enabled(Peripheral::AdcCtrl));
        let aes_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Aes));
        let csrng_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Csrng));
        let edn0_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Edn0));
        let edn1_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Edn1));
        let entropy_src_enable =
            quote_enable(peripheral_config.get_enabled(Peripheral::EntropySrc));
        let hmac_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Hmac));
        let keymgr_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Keymgr));
        let kmac_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Kmac));
        let clkmgr_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Clkmgr));
        let usb_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Usb));
        let uart0_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Uart0));
        let uart1_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Uart1));
        let uart2_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Uart2));
        let uart3_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Uart3));
        let otbn_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Otbn));
        let otp_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Otp));
        let i2c0_enable = quote_enable(peripheral_config.get_enabled(Peripheral::I2c0));
        let i2c1_enable = quote_enable(peripheral_config.get_enabled(Peripheral::I2c1));
        let i2c2_enable = quote_enable(peripheral_config.get_enabled(Peripheral::I2c2));
        let spi_device_enable = quote_enable(peripheral_config.get_enabled(Peripheral::SpiDevice));
        let spi_host0_enable = quote_enable(peripheral_config.get_enabled(Peripheral::SpiHost0));
        let spi_host1_enable = quote_enable(peripheral_config.get_enabled(Peripheral::SpiHost1));
        let flash_ctrl_enable = quote_enable(peripheral_config.get_enabled(Peripheral::FlashCtrl));
        let rng_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Rng));
        let watchdog_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Watchdog));
        let sensor_ctrl_enable =
            quote_enable(peripheral_config.get_enabled(Peripheral::SensorCtrl));
        let sysreset_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Sysreset));
        let timer_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Timer));
        let alert_handler_enable =
            quote_enable(peripheral_config.get_enabled(Peripheral::AlertHandler));
        let pattgen_enable = quote_enable(peripheral_config.get_enabled(Peripheral::Pattgen));
        let rst_mgmt_enable = quote_enable(peripheral_config.get_enabled(Peripheral::RstMgmt));

        let GpioEnable {
            driver_enabled,
            interrupts_enabled,
        } = peripheral_config.get_gpio_enabled();
        // Arrays do not implement `quote::ToTokens`, so we have to do this.
        let gpio0_intr_enable = interrupts_enabled[0];
        let gpio1_intr_enable = interrupts_enabled[1];
        let gpio2_intr_enable = interrupts_enabled[2];
        let gpio3_intr_enable = interrupts_enabled[3];
        let gpio4_intr_enable = interrupts_enabled[4];
        let gpio5_intr_enable = interrupts_enabled[5];
        let gpio6_intr_enable = interrupts_enabled[6];
        let gpio7_intr_enable = interrupts_enabled[7];
        let gpio8_intr_enable = interrupts_enabled[8];
        let gpio9_intr_enable = interrupts_enabled[9];
        let gpio10_intr_enable = interrupts_enabled[10];
        let gpio11_intr_enable = interrupts_enabled[11];
        let gpio12_intr_enable = interrupts_enabled[12];
        let gpio13_intr_enable = interrupts_enabled[13];
        let gpio14_intr_enable = interrupts_enabled[14];
        let gpio15_intr_enable = interrupts_enabled[15];
        let gpio16_intr_enable = interrupts_enabled[16];
        let gpio17_intr_enable = interrupts_enabled[17];
        let gpio18_intr_enable = interrupts_enabled[18];
        let gpio19_intr_enable = interrupts_enabled[19];
        let gpio20_intr_enable = interrupts_enabled[20];
        let gpio21_intr_enable = interrupts_enabled[21];
        let gpio22_intr_enable = interrupts_enabled[22];
        let gpio23_intr_enable = interrupts_enabled[23];
        let gpio24_intr_enable = interrupts_enabled[24];
        let gpio25_intr_enable = interrupts_enabled[25];
        let gpio26_intr_enable = interrupts_enabled[26];
        let gpio27_intr_enable = interrupts_enabled[27];
        let gpio28_intr_enable = interrupts_enabled[28];
        let gpio29_intr_enable = interrupts_enabled[29];
        let gpio30_intr_enable = interrupts_enabled[30];
        let gpio31_intr_enable = interrupts_enabled[31];
        let gpio_port_enable: parse::proc_macro2::TokenStream = quote::quote!(
            earlgrey::chip::GpioPeripheralConfig {
                driver_enabled: #driver_enabled,
                interrupts_enabled: [
                    #gpio0_intr_enable,
                    #gpio1_intr_enable,
                    #gpio2_intr_enable,
                    #gpio3_intr_enable,
                    #gpio4_intr_enable,
                    #gpio5_intr_enable,
                    #gpio6_intr_enable,
                    #gpio7_intr_enable,
                    #gpio8_intr_enable,
                    #gpio9_intr_enable,
                    #gpio10_intr_enable,
                    #gpio11_intr_enable,
                    #gpio12_intr_enable,
                    #gpio13_intr_enable,
                    #gpio14_intr_enable,
                    #gpio15_intr_enable,
                    #gpio16_intr_enable,
                    #gpio17_intr_enable,
                    #gpio18_intr_enable,
                    #gpio19_intr_enable,
                    #gpio20_intr_enable,
                    #gpio21_intr_enable,
                    #gpio22_intr_enable,
                    #gpio23_intr_enable,
                    #gpio24_intr_enable,
                    #gpio25_intr_enable,
                    #gpio26_intr_enable,
                    #gpio27_intr_enable,
                    #gpio28_intr_enable,
                    #gpio29_intr_enable,
                    #gpio30_intr_enable,
                    #gpio31_intr_enable,
                ]
            }
        );
        Some(quote::quote!(
            const EARLGREY_PERIPHERAL_CONFIG: earlgrey::chip::EarlgreyPeripheralConfig =
                earlgrey::chip::EarlgreyPeripheralConfig {
                    sram_ret: #sram_ret_enable,
                    adc_ctrl: #adc_ctrl_enable,
                    aes: #aes_enable,
                    csrng: #csrng_enable,
                    edn0: #edn0_enable,
                    edn1: #edn1_enable,
                    entropy_src: #entropy_src_enable,
                    hmac: #hmac_enable,
                    keymgr: #keymgr_enable,
                    kmac: #kmac_enable,
                    clkmgr: #clkmgr_enable,
                    usb: #usb_enable,
                    uart0: #uart0_enable,
                    uart1: #uart1_enable,
                    uart2: #uart2_enable,
                    uart3: #uart3_enable,
                    otbn: #otbn_enable,
                    otp: #otp_enable,
                    gpio_port: #gpio_port_enable,
                    i2c0: #i2c0_enable,
                    i2c1: #i2c1_enable,
                    i2c2: #i2c2_enable,
                    spi_host0: #spi_host0_enable,
                    spi_host1: #spi_host1_enable,
                    spi_device: #spi_device_enable,
                    flash_ctrl: #flash_ctrl_enable,
                    rng: #rng_enable,
                    watchdog: #watchdog_enable,
                    sensor_ctrl: #sensor_ctrl_enable,
                    sysreset: #sysreset_enable,
                    timer: #timer_enable,
                    alert_handler: #alert_handler_enable,
                    pattgen: #pattgen_enable,
                    rst_mgmt: #rst_mgmt_enable,
                };
            earlgrey::chip::configure_trap_handler();
            use earlgrey::pinmux_config::EarlGreyPinmuxConfig;
            pinmux_layout::BoardPinmuxLayout::setup();
        ))
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;
        let flash_memory_protection_configuration_identifier =
            quote::format_ident!("{}", self.flash_memory_protection_configuration.ident()?);

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            earlgrey::chip::EarlGreyDefaultPeripherals::new(#flash_memory_protection_configuration_identifier, EARLGREY_PERIPHERAL_CONFIG)
        )))
    }

    fn after_init(&self) -> Option<parse::proc_macro2::TokenStream> {
        use parse::DefaultPeripherals;
        let ident: parse::proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        let timer_ident: parse::proc_macro2::TokenStream =
            self.timer().unwrap()[0].ident().unwrap().parse().unwrap();
        Some(quote::quote! {
            #ident.init();
            #timer_ident.setup();
        })
    }

    fn dependencies(&self) -> Option<Vec<Rc<dyn parse::Component>>> {
        Some(vec![self.flash_memory_protection_configuration.clone()])
    }
}

impl parse::DefaultPeripherals for Drivers {
    type Gpio = crate::gpio::GpioPort;
    type Uart = crate::uart::Uart;
    type Timer = crate::timer::RvTimer;
    type Spi = crate::spi::SpiHost;
    type I2c = crate::i2c::I2c;
    type BleAdvertisement = parse::NoSupport;
    type Flash = crate::flash::FlashCtrl;
    type Temperature = parse::NoSupport;
    type Rng = crate::rng::CsRng;
    type Hmac = crate::hmac::Hmac;
    type Aes = crate::aes::Aes;
    type Pattgen = crate::pattgen::Pattgen;
    type SystemResetController = crate::system_reset_controller::SystemResetController;
    type AlertHandler = crate::alert_handler::AlertHandler;
    type Usb = crate::usb::Usb;
    type ResetManager = crate::reset_manager::ResetManager;
    type Attestation = crate::attestation::EarlgreyAttestation<crate::flash::FlashCtrl>;
    type OneshotSha256 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha256;
    type OneshotSha384 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha384;
    type OneshotSha512 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha512;
    type OneshotSha3_224 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_224;
    type OneshotSha3_256 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_256;
    type OneshotSha3_384 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_384;
    type OneshotSha3_512 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotSha3_512;
    type OneshotShake128 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotShake128;
    type OneshotShake256 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotShake256;
    type OneshotCshake128 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotCshake128;
    type OneshotCshake256 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotCshake256;
    type OneshotHmacSha256 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha256;
    type OneshotHmacSha384 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha384;
    type OneshotHmacSha512 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotHmacSha512;
    type OneshotKmac128 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotKmac128;
    type OneshotKmac256 = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotKmac256;
    type P256 = crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP256<crate::timer::RvTimer>;
    type P384 = crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP384<crate::timer::RvTimer>;

    fn aes(&self) -> Result<&[Rc<Self::Aes>], parse::Error> {
        Ok(&self.aes)
    }

    fn alert_handler(&self) -> Result<&[Rc<Self::AlertHandler>], parse::Error> {
        Ok(&self.alert_handlers)
    }

    fn flash(&self) -> Result<&[Rc<Self::Flash>], parse::Error> {
        Ok(&self.flashes)
    }

    fn gpio(&self) -> Result<&[Rc<Self::Gpio>], parse::Error> {
        Ok(&self.gpios)
    }

    fn hmac(&self) -> Result<&[Rc<Self::Hmac>], parse::Error> {
        Ok(&self.hmacs)
    }

    fn i2c(&self) -> Result<&[Rc<Self::I2c>], parse::Error> {
        Ok(&self.i2cs)
    }

    fn pattgen(&self) -> Result<&[Rc<Self::Pattgen>], parse::Error> {
        Ok(&self.pattgens)
    }

    fn reset_manager(&self) -> Result<&[Rc<Self::ResetManager>], parse::Error> {
        Ok(&self.reset_managers)
    }

    fn rng(&self) -> Result<&[Rc<Self::Rng>], parse::Error> {
        Ok(&self.rngs)
    }

    fn spi(&self) -> Result<&[Rc<Self::Spi>], parse::Error> {
        Ok(&self.spis)
    }

    fn system_reset_controller(&self) -> Result<&[Rc<Self::SystemResetController>], parse::Error> {
        Ok(&self.system_reset_controllers)
    }

    fn timer(&self) -> Result<&[Rc<Self::Timer>], parse::Error> {
        Ok(&self.timers)
    }

    fn uart(&self) -> Result<&[Rc<Self::Uart>], parse::Error> {
        Ok(&self.uarts)
    }

    fn usb(&self) -> Result<&[Rc<Self::Usb>], parse::Error> {
        Ok(&self.usbs)
    }

    fn attestation(&self) -> Result<&[Rc<Self::Attestation>], parse::Error> {
        Ok(&self.attestations)
    }

    fn oneshot_sha256(&self) -> Result<&[Rc<Self::OneshotSha256>], parse::Error> {
        Ok(&self.oneshot_sha256s)
    }
    fn oneshot_sha384(&self) -> Result<&[Rc<Self::OneshotSha384>], parse::Error> {
        Ok(&self.oneshot_sha384s)
    }
    fn oneshot_sha512(&self) -> Result<&[Rc<Self::OneshotSha512>], parse::Error> {
        Ok(&self.oneshot_sha512s)
    }
    fn oneshot_sha3_224(&self) -> Result<&[Rc<Self::OneshotSha3_224>], parse::Error> {
        Ok(&self.oneshot_sha3_224s)
    }
    fn oneshot_sha3_256(&self) -> Result<&[Rc<Self::OneshotSha3_256>], parse::Error> {
        Ok(&self.oneshot_sha3_256s)
    }
    fn oneshot_sha3_384(&self) -> Result<&[Rc<Self::OneshotSha3_384>], parse::Error> {
        Ok(&self.oneshot_sha3_384s)
    }
    fn oneshot_sha3_512(&self) -> Result<&[Rc<Self::OneshotSha3_512>], parse::Error> {
        Ok(&self.oneshot_sha3_512s)
    }
    fn oneshot_shake128(&self) -> Result<&[Rc<Self::OneshotShake128>], parse::Error> {
        Ok(&self.oneshot_shake128s)
    }
    fn oneshot_shake256(&self) -> Result<&[Rc<Self::OneshotShake256>], parse::Error> {
        Ok(&self.oneshot_shake256s)
    }
    fn oneshot_cshake128(&self) -> Result<&[Rc<Self::OneshotCshake128>], parse::Error> {
        Ok(&self.oneshot_cshake128s)
    }
    fn oneshot_cshake256(&self) -> Result<&[Rc<Self::OneshotCshake256>], parse::Error> {
        Ok(&self.oneshot_cshake256s)
    }
    fn oneshot_hmac_sha256(&self) -> Result<&[Rc<Self::OneshotHmacSha256>], parse::Error> {
        Ok(&self.oneshot_hmac_sha256s)
    }
    fn oneshot_hmac_sha384(&self) -> Result<&[Rc<Self::OneshotHmacSha384>], parse::Error> {
        Ok(&self.oneshot_hmac_sha384s)
    }
    fn oneshot_hmac_sha512(&self) -> Result<&[Rc<Self::OneshotHmacSha512>], parse::Error> {
        Ok(&self.oneshot_hmac_sha512s)
    }
    fn oneshot_kmac128(&self) -> Result<&[Rc<Self::OneshotKmac128>], parse::Error> {
        Ok(&self.oneshot_kmac128s)
    }
    fn oneshot_kmac256(&self) -> Result<&[Rc<Self::OneshotKmac256>], parse::Error> {
        Ok(&self.oneshot_kmac256s)
    }

    fn p256(&self) -> Result<&[Rc<Self::P256>], parse::Error> {
        Ok(&self.p256s)
    }

    fn p384(&self) -> Result<&[Rc<Self::P384>], parse::Error> {
        Ok(&self.p384s)
    }
}

/// Whether to enable interrupts and/or the standard driver for a given
/// peripheral.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeripheralEnable {
    /// Disable peripheral-specific driver and interrupts.
    #[default]
    Disabled,
    /// Enable interrupts, but disable peripheral-specific driver. Useful when
    /// nonstandard drivers that rely on the peripheral interrupt, such as
    /// OpenTitan cryptolib APIs.
    InterruptsOnly,
    /// Enable peripheral-specific driver and interrupts.
    Enabled,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PeripheralCount {
    /// This peripheral has no users.
    #[default]
    None,
    /// This peripheral has a single user.
    Single,
    /// This peripheral has multiple users.
    Multiple,
}

impl PeripheralCount {
    /// Increments the peripheral count.
    fn increment(&mut self) {
        *self = match self {
            PeripheralCount::None => PeripheralCount::Single,
            _ => PeripheralCount::Multiple,
        };
    }
}

/// Configuration for a single peripheral.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PeripheralConfig {
    count: PeripheralCount,
    enable: PeripheralEnable,
}

/// Placeholder value indicating a peripheral configuration API takes no
/// parameter.
pub const NO_PARAM: usize = 0;

impl PeripheralConfig {
    /// Returns a new `PeripheralConfig`.
    fn new() -> PeripheralConfig {
        PeripheralConfig {
            count: PeripheralCount::None,
            enable: PeripheralEnable::Disabled,
        }
    }

    /// Add a dependent for the peripheral driver.
    fn require(&mut self) {
        self.count.increment();
        self.enable = PeripheralEnable::Enabled;
    }

    /// Record that a driver requires interrupts from the peripheral, but this
    /// doesn't affect our decision to insert a virtualizer or not.
    fn require_interrupts(&mut self) {
        self.enable = match self.enable {
            PeripheralEnable::Enabled => PeripheralEnable::Enabled,
            _ => PeripheralEnable::InterruptsOnly,
        }
    }

    fn should_virtualize(&self) -> bool {
        self.count == PeripheralCount::Multiple
    }

    fn get_enabled(&self) -> PeripheralEnable {
        self.enable
    }
}

/// Configuration for the FlashCtrl peripheral, which needs to track reference
/// counts for info/data virtualizers separately.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FlashCtrlConfig {
    data_count: PeripheralCount,
    info_count: PeripheralCount,
    enable: PeripheralEnable,
}

/// A flash_ctrl peripheral dependency corresponds to data pages.
pub const FLASH_CTRL_CONFIG_DATA: usize = 0;
/// A flash_ctrl peripheral dependency corresponds to info pages.
pub const FLASH_CTRL_CONFIG_INFO: usize = 1;

impl FlashCtrlConfig {
    /// Returns a new `FlashCtrlConfig`.
    fn new() -> FlashCtrlConfig {
        FlashCtrlConfig {
            data_count: PeripheralCount::None,
            info_count: PeripheralCount::None,
            enable: PeripheralEnable::Disabled,
        }
    }

    /// Add a dependent for the peripheral driver.
    fn require(&mut self, mode: usize) {
        match mode {
            FLASH_CTRL_CONFIG_DATA => self.data_count.increment(),
            FLASH_CTRL_CONFIG_INFO => self.info_count.increment(),
            _ => panic!("Invalid flash_ctrl mode constant"),
        };
        self.enable = PeripheralEnable::Enabled;
    }

    /// Record that a driver requires interrupts from the peripheral, but this
    /// doesn't affect our decision to insert a virtualizer or not.
    fn require_interrupts(&mut self) {
        self.enable = match self.enable {
            PeripheralEnable::Enabled => PeripheralEnable::Enabled,
            _ => PeripheralEnable::InterruptsOnly,
        }
    }

    fn should_virtualize(&self) -> bool {
        self.data_count == PeripheralCount::Multiple
    }

    fn should_virtualize_info(&self) -> bool {
        self.info_count == PeripheralCount::Multiple
    }

    fn get_enabled(&self) -> PeripheralEnable {
        self.enable
    }
}

/// Enable state for GPIO pins, which enable interrupts for individual pins
/// separately.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GpioEnable {
    /// Whether to enable the driver.
    driver_enabled: bool,
    /// Whether to enable interrupts for each pin.
    interrupts_enabled: [bool; crate::gpio::GPIO_PINS],
}

/// Configuration for GPIO pins, which enable interrupts for individual pins
/// separately.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct GpioConfig {
    /// GPIO enable state.
    enabled: GpioEnable,
    /// Store the count separately, so we can selectively include virtualizers
    /// for each GPIO pin separately, if an implementation is provided.
    counts: [PeripheralCount; crate::gpio::GPIO_PINS],
}

impl GpioConfig {
    /// Returns a new `GpioConfig`
    fn new() -> GpioConfig {
        GpioConfig {
            enabled: GpioEnable {
                driver_enabled: false,
                interrupts_enabled: [false; crate::gpio::GPIO_PINS],
            },
            counts: [PeripheralCount::None; crate::gpio::GPIO_PINS],
        }
    }

    /// Add a dependent for the peripheral driver.
    fn require(&mut self, pin: usize) {
        if pin >= crate::gpio::GPIO_PINS {
            panic!("GPIO pin ID out of bounds");
        }
        self.counts[pin] = match self.counts[pin] {
            PeripheralCount::None => PeripheralCount::Single,
            _ => PeripheralCount::Multiple,
        };
        self.enabled.driver_enabled = true;
        self.enabled.interrupts_enabled[pin] = true;
    }

    /// Record that a driver requires interrupts from the peripheral, but this
    /// doesn't affect our decision to insert a virtualizer or not.
    fn require_interrupts(&mut self, pin: usize) {
        if pin >= crate::gpio::GPIO_PINS {
            panic!("GPIO pin ID out of bounds");
        }
        self.enabled.interrupts_enabled[pin] = true;
    }

    /// TODO: remove this annotation if GPIO virtualization support is added.
    #[allow(unused)]
    fn should_virtualize(&self, pin: usize) -> bool {
        if pin >= crate::gpio::GPIO_PINS {
            panic!("GPIO pin ID out of bounds");
        }
        self.counts[pin] == PeripheralCount::Multiple
    }
}

/// Set of Earlgrey Peripherals to configure.
#[repr(usize)]
pub enum Peripheral {
    AdcCtrl = 0,
    Aes,
    AlertHandler,
    Clkmgr,
    Csrng,
    Edn0,
    Edn1,
    EntropySrc,
    FlashCtrl,
    GpioPort,
    Hmac,
    I2c0,
    I2c1,
    I2c2,
    Keymgr,
    Kmac,
    Otbn,
    Otp,
    Pattgen,
    RstMgmt,
    Rng,
    SensorCtrl,
    SpiDevice,
    SpiHost0,
    SpiHost1,
    SramRet,
    Sysreset,
    Timer,
    Uart0,
    Uart1,
    Uart2,
    Uart3,
    Usb,
    Watchdog,
}

impl TryFrom<usize> for Peripheral {
    type Error = ();
    fn try_from(n: usize) -> Result<Peripheral, ()> {
        match n {
            0 => Ok(Peripheral::AdcCtrl),
            1 => Ok(Peripheral::Aes),
            2 => Ok(Peripheral::AlertHandler),
            3 => Ok(Peripheral::Clkmgr),
            4 => Ok(Peripheral::Csrng),
            5 => Ok(Peripheral::Edn0),
            6 => Ok(Peripheral::Edn1),
            7 => Ok(Peripheral::EntropySrc),
            8 => Ok(Peripheral::FlashCtrl),
            9 => Ok(Peripheral::GpioPort),
            10 => Ok(Peripheral::Hmac),
            11 => Ok(Peripheral::I2c0),
            12 => Ok(Peripheral::I2c1),
            13 => Ok(Peripheral::I2c2),
            14 => Ok(Peripheral::Keymgr),
            15 => Ok(Peripheral::Kmac),
            16 => Ok(Peripheral::Otbn),
            17 => Ok(Peripheral::Otp),
            18 => Ok(Peripheral::Pattgen),
            19 => Ok(Peripheral::RstMgmt),
            20 => Ok(Peripheral::Rng),
            21 => Ok(Peripheral::SensorCtrl),
            22 => Ok(Peripheral::SpiDevice),
            23 => Ok(Peripheral::SpiHost0),
            24 => Ok(Peripheral::SpiHost1),
            25 => Ok(Peripheral::SramRet),
            26 => Ok(Peripheral::Sysreset),
            27 => Ok(Peripheral::Timer),
            28 => Ok(Peripheral::Uart0),
            29 => Ok(Peripheral::Uart1),
            30 => Ok(Peripheral::Uart2),
            31 => Ok(Peripheral::Uart3),
            32 => Ok(Peripheral::Usb),
            33 => Ok(Peripheral::Watchdog),
            _ => Err(()),
        }
    }
}

/// Earlgrey peripheral configuration.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EarlgreyPeripheralConfig {
    adc_ctrl: PeripheralConfig,
    aes: PeripheralConfig,
    alert_handler: PeripheralConfig,
    clkmgr: PeripheralConfig,
    csrng: PeripheralConfig,
    edn0: PeripheralConfig,
    edn1: PeripheralConfig,
    entropy_src: PeripheralConfig,
    flash_ctrl: FlashCtrlConfig,
    gpio_port: GpioConfig,
    hmac: PeripheralConfig,
    i2c0: PeripheralConfig,
    i2c1: PeripheralConfig,
    i2c2: PeripheralConfig,
    keymgr: PeripheralConfig,
    kmac: PeripheralConfig,
    otbn: PeripheralConfig,
    otp: PeripheralConfig,
    pattgen: PeripheralConfig,
    spi_device: PeripheralConfig,
    spi_host0: PeripheralConfig,
    spi_host1: PeripheralConfig,
    sram_ret: PeripheralConfig,
    rng: PeripheralConfig,
    sensor_ctrl: PeripheralConfig,
    sysreset: PeripheralConfig,
    rst_mgmt: PeripheralConfig,
    timer: PeripheralConfig,
    uart0: PeripheralConfig,
    uart1: PeripheralConfig,
    uart2: PeripheralConfig,
    uart3: PeripheralConfig,
    usb: PeripheralConfig,
    watchdog: PeripheralConfig,
}

impl EarlgreyPeripheralConfig {
    /// Construct a new `EarlgreyPeripheralConfig`.
    pub fn new() -> EarlgreyPeripheralConfig {
        EarlgreyPeripheralConfig {
            adc_ctrl: PeripheralConfig::new(),
            aes: PeripheralConfig::new(),
            alert_handler: PeripheralConfig::new(),
            clkmgr: PeripheralConfig::new(),
            csrng: PeripheralConfig::new(),
            edn0: PeripheralConfig::new(),
            edn1: PeripheralConfig::new(),
            entropy_src: PeripheralConfig::new(),
            flash_ctrl: FlashCtrlConfig::new(),
            gpio_port: GpioConfig::new(),
            hmac: PeripheralConfig::new(),
            i2c0: PeripheralConfig::new(),
            i2c1: PeripheralConfig::new(),
            i2c2: PeripheralConfig::new(),
            keymgr: PeripheralConfig::new(),
            kmac: PeripheralConfig::new(),
            otbn: PeripheralConfig::new(),
            otp: PeripheralConfig::new(),
            pattgen: PeripheralConfig::new(),
            spi_device: PeripheralConfig::new(),
            spi_host0: PeripheralConfig::new(),
            spi_host1: PeripheralConfig::new(),
            sram_ret: PeripheralConfig::new(),
            rng: PeripheralConfig::new(),
            sensor_ctrl: PeripheralConfig::new(),
            sysreset: PeripheralConfig::new(),
            rst_mgmt: PeripheralConfig::new(),
            timer: PeripheralConfig::new(),
            uart0: PeripheralConfig::new(),
            uart1: PeripheralConfig::new(),
            uart2: PeripheralConfig::new(),
            uart3: PeripheralConfig::new(),
            usb: PeripheralConfig::new(),
            watchdog: PeripheralConfig::new(),
        }
    }

    /// Whether the board should include the base driver for the given
    /// peripheral. Meaning of `param` is peripheral-specific.
    pub fn get_enabled(&self, peripheral: Peripheral) -> PeripheralEnable {
        match peripheral {
            Peripheral::AdcCtrl => self.adc_ctrl.get_enabled(),
            Peripheral::Aes => self.aes.get_enabled(),
            Peripheral::AlertHandler => self.alert_handler.get_enabled(),
            Peripheral::Clkmgr => self.clkmgr.get_enabled(),
            Peripheral::Csrng => self.csrng.get_enabled(),
            Peripheral::Edn0 => self.edn0.get_enabled(),
            Peripheral::Edn1 => self.edn1.get_enabled(),
            Peripheral::EntropySrc => self.entropy_src.get_enabled(),
            Peripheral::FlashCtrl => self.flash_ctrl.get_enabled(),
            Peripheral::GpioPort => panic!("Should call get_gpio_enabled() instead"),
            Peripheral::Hmac => self.hmac.get_enabled(),
            Peripheral::I2c0 => self.i2c0.get_enabled(),
            Peripheral::I2c1 => self.i2c1.get_enabled(),
            Peripheral::I2c2 => self.i2c2.get_enabled(),
            Peripheral::Keymgr => self.keymgr.get_enabled(),
            Peripheral::Kmac => self.kmac.get_enabled(),
            Peripheral::Otbn => self.otbn.get_enabled(),
            Peripheral::Otp => self.otp.get_enabled(),
            Peripheral::Pattgen => self.pattgen.get_enabled(),
            Peripheral::RstMgmt => self.rst_mgmt.get_enabled(),
            Peripheral::Rng => self.rng.get_enabled(),
            Peripheral::SensorCtrl => self.sensor_ctrl.get_enabled(),
            Peripheral::SpiDevice => self.spi_device.get_enabled(),
            Peripheral::SpiHost0 => self.spi_host0.get_enabled(),
            Peripheral::SpiHost1 => self.spi_host1.get_enabled(),
            Peripheral::SramRet => self.sram_ret.get_enabled(),
            Peripheral::Sysreset => self.sysreset.get_enabled(),
            Peripheral::Timer => self.timer.get_enabled(),
            Peripheral::Uart0 => self.uart0.get_enabled(),
            Peripheral::Uart1 => self.uart1.get_enabled(),
            Peripheral::Uart2 => self.uart2.get_enabled(),
            Peripheral::Uart3 => self.uart3.get_enabled(),
            Peripheral::Usb => self.usb.get_enabled(),
            Peripheral::Watchdog => self.watchdog.get_enabled(),
        }
    }

    fn get_gpio_enabled(&self) -> GpioEnable {
        self.gpio_port.enabled
    }
}

impl parse::component::ConfigPeripherals for EarlgreyPeripheralConfig {
    /// Invoked by a meta-driver to indicate it corresponds to a particular real
    /// driver that should be included in the board definition. Parameters are
    /// implementation-dependent.
    fn require(&mut self, peripheral: usize, param: usize) {
        match Peripheral::try_from(peripheral).expect("Invalid peripheral identifier.") {
            Peripheral::AdcCtrl => self.adc_ctrl.require(),
            Peripheral::Aes => self.aes.require(),
            Peripheral::AlertHandler => self.alert_handler.require(),
            Peripheral::Clkmgr => self.clkmgr.require(),
            Peripheral::Csrng => self.csrng.require(),
            Peripheral::Edn0 => self.edn0.require(),
            Peripheral::Edn1 => self.edn1.require(),
            Peripheral::EntropySrc => self.entropy_src.require(),
            Peripheral::FlashCtrl => self.flash_ctrl.require(param),
            Peripheral::GpioPort => self.gpio_port.require(param),
            Peripheral::Hmac => self.hmac.require(),
            Peripheral::I2c0 => self.i2c0.require(),
            Peripheral::I2c1 => self.i2c1.require(),
            Peripheral::I2c2 => self.i2c2.require(),
            Peripheral::Keymgr => self.keymgr.require(),
            Peripheral::Kmac => self.kmac.require(),
            Peripheral::Otbn => self.otbn.require(),
            Peripheral::Otp => self.otp.require(),
            Peripheral::Pattgen => self.pattgen.require(),
            Peripheral::RstMgmt => self.rst_mgmt.require(),
            Peripheral::Rng => self.rng.require(),
            Peripheral::SensorCtrl => self.sensor_ctrl.require(),
            Peripheral::SpiDevice => self.spi_device.require(),
            Peripheral::SpiHost0 => self.spi_host0.require(),
            Peripheral::SpiHost1 => self.spi_host1.require(),
            Peripheral::SramRet => self.sram_ret.require(),
            Peripheral::Sysreset => self.sysreset.require(),
            Peripheral::Timer => self.timer.require(),
            Peripheral::Uart0 => self.uart0.require(),
            Peripheral::Uart1 => self.uart1.require(),
            Peripheral::Uart2 => self.uart2.require(),
            Peripheral::Uart3 => self.uart3.require(),
            Peripheral::Usb => self.usb.require(),
            Peripheral::Watchdog => self.watchdog.require(),
        }
    }

    /// Invoked by a meta-driver to indicate its corresponding real driver
    /// requires interrupts enabled for a particular HWIP, but not necessarily
    /// its own peripheral driver. Parameters are implementation-dependent.
    fn require_interrupts(&mut self, peripheral: usize, param: usize) {
        match Peripheral::try_from(peripheral).expect("Invalid peripheral identifier.") {
            Peripheral::AdcCtrl => self.adc_ctrl.require_interrupts(),
            Peripheral::Aes => self.aes.require_interrupts(),
            Peripheral::AlertHandler => self.alert_handler.require_interrupts(),
            Peripheral::Clkmgr => self.clkmgr.require_interrupts(),
            Peripheral::Csrng => self.csrng.require_interrupts(),
            Peripheral::Edn0 => self.edn0.require_interrupts(),
            Peripheral::Edn1 => self.edn1.require_interrupts(),
            Peripheral::EntropySrc => self.entropy_src.require_interrupts(),
            Peripheral::FlashCtrl => self.flash_ctrl.require_interrupts(),
            Peripheral::GpioPort => self.gpio_port.require_interrupts(param),
            Peripheral::Hmac => self.hmac.require_interrupts(),
            Peripheral::I2c0 => self.i2c0.require_interrupts(),
            Peripheral::I2c1 => self.i2c1.require_interrupts(),
            Peripheral::I2c2 => self.i2c2.require_interrupts(),
            Peripheral::Keymgr => self.keymgr.require_interrupts(),
            Peripheral::Kmac => self.kmac.require_interrupts(),
            Peripheral::Otbn => self.otbn.require_interrupts(),
            Peripheral::Otp => self.otp.require_interrupts(),
            Peripheral::Pattgen => self.pattgen.require_interrupts(),
            Peripheral::RstMgmt => self.rst_mgmt.require_interrupts(),
            Peripheral::Rng => self.rng.require_interrupts(),
            Peripheral::SensorCtrl => self.sensor_ctrl.require_interrupts(),
            Peripheral::SpiDevice => self.spi_device.require_interrupts(),
            Peripheral::SpiHost0 => self.spi_host0.require_interrupts(),
            Peripheral::SpiHost1 => self.spi_host1.require_interrupts(),
            Peripheral::SramRet => self.sram_ret.require_interrupts(),
            Peripheral::Sysreset => self.sysreset.require_interrupts(),
            Peripheral::Timer => self.timer.require_interrupts(),
            Peripheral::Uart0 => self.uart0.require_interrupts(),
            Peripheral::Uart1 => self.uart1.require_interrupts(),
            Peripheral::Uart2 => self.uart2.require_interrupts(),
            Peripheral::Uart3 => self.uart3.require_interrupts(),
            Peripheral::Usb => self.usb.require_interrupts(),
            Peripheral::Watchdog => self.watchdog.require_interrupts(),
        }
    }

    /// Whether the peripheral configuration indicates that a flash virtualizer
    /// should be used.
    fn should_virtualize_flash(&self) -> bool {
        self.flash_ctrl.should_virtualize()
    }

    /// Whether the peripheral configuration indicates that an info flash
    /// virtualizer should be used.
    fn should_virtualize_info_flash(&self) -> bool {
        self.flash_ctrl.should_virtualize_info()
    }

    /// Whether the peripheral configuration indicates that a timer virtualizer
    /// should be used.
    fn should_virtualize_timer(&self) -> bool {
        self.timer.should_virtualize()
    }
}

fn quote_enable(enable: PeripheralEnable) -> parse::proc_macro2::TokenStream {
    match enable {
        PeripheralEnable::Disabled => quote::quote!(earlgrey::chip::PeripheralConfig::Disabled),
        PeripheralEnable::InterruptsOnly => {
            quote::quote!(earlgrey::chip::PeripheralConfig::InterruptsOnly)
        }
        PeripheralEnable::Enabled => quote::quote!(earlgrey::chip::PeripheralConfig::Enabled),
    }
}
