// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use parse::Ident as _;

use std::rc::Rc;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Peripherals {
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
    attestations: [Rc<
        crate::attestation::EarlgreyAttestation<
            parse::platform::capsules::info_flash::InfoFlashUser<crate::flash::FlashCtrl>,
        >,
    >; 1],
    oneshot_digests: [Rc<crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest>; 1],
    p256s: [Rc<crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP256<crate::timer::RvTimer>>; 1],
    p384s: [Rc<crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP384<crate::timer::RvTimer>>; 1],
}

impl Peripherals {
    pub fn new() -> Self {
        let timer = Rc::new(crate::timer::RvTimer::new());
        let flash = Rc::new(crate::flash::FlashCtrl::new());
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
                Rc::new(parse::platform::capsules::info_flash::InfoFlashUser::new(
                    flash,
                )),
            ))],
            oneshot_digests: [Rc::new(
                crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest::new(),
            )],
            p256s: [Rc::new(
                crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP256::new(cryptolib_mux.clone()),
            )],
            p384s: [Rc::new(
                crate::ffi::cryptolib::ecdsa::OtCryptoEcdsaP384::new(cryptolib_mux.clone()),
            )],
        }
    }
}

impl Default for Peripherals {
    fn default() -> Self {
        Self::new()
    }
}

impl parse::Component for Peripherals {
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
        Some(quote::quote! {
            earlgrey::chip::configure_trap_handler();
            use earlgrey::pinmux_config::EarlGreyPinmuxConfig;
            pinmux_layout::BoardPinmuxLayout::setup();
        })
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;
        let flash_memory_protection_configuration_identifier =
            quote::format_ident!("{}", self.flash_memory_protection_configuration.ident()?);

        Ok(quote::quote!(kernel::static_init!(
            #ty,
            earlgrey::chip::EarlGreyDefaultPeripherals::new(#flash_memory_protection_configuration_identifier)
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

impl parse::DefaultPeripherals for Peripherals {
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
    type Attestation = crate::attestation::EarlgreyAttestation<
        parse::platform::capsules::info_flash::InfoFlashUser<crate::flash::FlashCtrl>,
    >;
    type OneshotDigest = crate::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest;
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

    fn oneshot_digest(&self) -> Result<&[Rc<Self::OneshotDigest>], parse::Error> {
        Ok(&self.oneshot_digests)
    }

    fn p256(&self) -> Result<&[Rc<Self::P256>], parse::Error> {
        Ok(&self.p256s)
    }

    fn p384(&self) -> Result<&[Rc<Self::P384>], parse::Error> {
        Ok(&self.p384s)
    }
}
