use std::rc::Rc;

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
    type PinId = usize;

    fn pins(&self) -> Option<std::rc::Rc<[Self::PinId]>> {
        Some(Rc::new([
            0,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            10,
            11,
            12,
            13,
            14,
            15,
        ]))
    }
}
