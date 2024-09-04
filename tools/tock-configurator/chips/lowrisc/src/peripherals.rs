use parse::Ident as _;

use std::rc::Rc;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Peripherals {
    flash_memory_protection_configuration: Rc<crate::flash_memory_protection::FlashMemoryProtectionConfiguration>,
    gpios: [Rc<crate::gpio::GpioPort>; 1],
    timers: [Rc<crate::timer::RvTimer>; 1],
    uarts: [Rc<crate::uart::Uart>; 1],
}

impl Peripherals {
    pub fn new() -> Self {
        Self {
            flash_memory_protection_configuration: Rc::new(super::flash_memory_protection::FlashMemoryProtectionConfiguration::new()),
            gpios: [Rc::new(crate::gpio::GpioPort::new())],
            timers: [Rc::new(crate::timer::RvTimer::new())],
            uarts: [Rc::new(crate::uart::Uart::new())],
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
            earlgrey::chip::EarlGreyDefaultPeripherals<ChipConfig, crate::pinmux_layout::BoardPinmuxLayout>
        ))
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;
        let flash_memory_protection_configuration_identifier = quote::format_ident!("{}", self.flash_memory_protection_configuration.ident()?);

        Ok(quote::quote!(
            kernel::static_init!(
                #ty,
                earlgrey::chip::EarlGreyDefaultPeripherals::new(#flash_memory_protection_configuration_identifier)
            )
        ))
    }

    fn dependencies(&self) -> Option<Vec<Rc<dyn parse::Component>>> {
        Some(vec![self.flash_memory_protection_configuration.clone()])
    }
}

impl parse::DefaultPeripherals for Peripherals {
    type Gpio = crate::gpio::GpioPort;
    type Uart = crate::uart::Uart;
    type Timer = crate::timer::RvTimer;
    type Spi = parse::NoSupport;
    type I2c = parse::NoSupport;
    type BleAdvertisement = parse::NoSupport;
    type Flash = parse::NoSupport;
    type Temperature = parse::NoSupport;
    type Rng = parse::NoSupport;

    fn gpio(&self) -> Result<&[Rc<Self::Gpio>], parse::Error> {
        Ok(&self.gpios)
    }

    fn timer(&self) -> Result<&[Rc<Self::Timer>], parse::Error> {
        Ok(&self.timers)
    }

    fn uart(&self) -> Result<&[Rc<Self::Uart>], parse::Error> {
        Ok(&self.uarts)
    }
}
