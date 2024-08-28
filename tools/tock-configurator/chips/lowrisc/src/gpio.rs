use std::rc::Rc;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinId {
    Pin0,
    Pin1,
    Pin2,
    Pin3,
    Pin4,
    Pin5,
    Pin6,
    Pin7,
    Pin8,
    Pin9,
    Pin10,
    Pin11,
    Pin12,
    Pin13,
    Pin14,
    Pin15,
}

impl std::fmt::Display for PinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pin{}", *self as usize)
    }
}

impl parse::Ident for PinId {
    fn ident(&self) -> Result<String, parse::Error> {
        let index = *self as usize;
        Ok(format!("peripherals.gpio_port[{}]", index))
    }
}

impl parse::Component for PinId {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            earlgrey::gpio::GpioPin<'static, earlgrey::pinmux::PadConfig>
        ))
    }

    fn init_expr(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        todo!()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[derive(PartialEq)]
pub struct GpioPort {}

impl GpioPort {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl parse::Ident for GpioPort {
    fn ident(&self) -> Result<String, parse::Error> {
        Ok(String::from("peripherals.gpio_port"))
    }
}

impl parse::Component for GpioPort {
    fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(earlgrey::gpio::Port<'static>))
    }
}

impl std::fmt::Display for GpioPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "gpio_port")
    }
}

impl parse::peripherals::Gpio for GpioPort {
    type PinId = PinId;

    fn pins(&self) -> Option<std::rc::Rc<[Self::PinId]>> {
        Some(Rc::new([
            PinId::Pin0,
            PinId::Pin1,
            PinId::Pin2,
            PinId::Pin3,
            PinId::Pin4,
            PinId::Pin5,
            PinId::Pin6,
            PinId::Pin7,
            PinId::Pin8,
            PinId::Pin9,
            PinId::Pin10,
            PinId::Pin11,
            PinId::Pin12,
            PinId::Pin13,
            PinId::Pin14,
            PinId::Pin15,
        ]))
    }
}
