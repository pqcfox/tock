use parse::Chip as _;
use parse::DefaultPeripherals as _;
use parse::Ident as _;

use std::rc::Rc;

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Chip {
    epmp: Rc<crate::epmp::Epmp>,
    peripherals: Rc<crate::peripherals::Peripherals>,
    scheduler_timer: Rc<parse::SchedulerTimer<crate::timer::RvTimer>>,
}

impl Default for Chip {
    fn default() -> Self {
        let peripherals = Rc::new(crate::peripherals::Peripherals::new());
        let timer = peripherals.timer().unwrap()[0].clone();
        let mux_alarm = Rc::new(parse::MuxAlarm::new(timer));
        let virtual_mux_alarm = Rc::new(parse::VirtualMuxAlarm::new(mux_alarm));

        Self {
            epmp: Rc::new(crate::epmp::Epmp::new()),
            peripherals,
            scheduler_timer: parse::SchedulerTimer::new(virtual_mux_alarm),
        }
    }
}

impl Chip {
    pub fn new() -> Self {
        Self::default()
    }
}

impl parse::Ident for Chip {
    fn ident(&self) -> Result<String, parse::error::Error> {
        Ok(parse::constants::CHIP.clone())
    }
}

impl parse::Component for Chip {
    fn ty(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            earlgrey::chip::EarlGrey<
                'static,
                { <earlgrey::epmp::EPMPDebugDisable as earlgrey::epmp::EPMPDebugConfig>::TOR_USER_REGIONS },
                earlgrey::chip::EarlGreyDefaultPeripherals<'static, ChipConfig, crate::pinmux_layout::BoardPinmuxLayout>,
                ChipConfig,
                crate::pinmux_layout::BoardPinmuxLayout,
                earlgrey::epmp::EarlGreyEPMP<{ EPMP_HANDOVER_CONFIG_CHECK }, earlgrey::epmp::EPMPDebugDisable>,
            >
        ))
    }

    fn dependencies(&self) -> Option<Vec<Rc<dyn parse::Component>>> {
        let peripherals = self.peripherals();
        let epmp = self.epmp.clone();

        Some(vec![epmp, peripherals])
    }

    fn before_init(&self) -> Option<parse::proc_macro2::TokenStream> {
        None
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;
        let peripherals = self.peripherals();
        let peripherals_identifier = quote::format_ident!("{}", peripherals.ident()?);

        Ok(quote::quote!(
            kernel::static_init!(
                #ty,
                earlgrey::chip::EarlGrey::new(#peripherals_identifier, earlgrey_epmp),
            )
        ))
    }

    fn after_init(&self) -> Option<parse::proc_macro2::TokenStream> {
        let ident: parse::proc_macro2::TokenStream = self.ident().unwrap().parse().unwrap();
        Some(quote::quote! {
            use kernel::utilities::registers::interfaces::ReadWriteable;
            #ident.enable_plic_interrupts();
            // enable interrupts globally
            rv32i::csr::CSR.mie.modify(
                rv32i::csr::mie::mie::msoft::SET + rv32i::csr::mie::mie::mtimer::CLEAR + rv32i::csr::mie::mie::mext::SET,
            );
            rv32i::csr::CSR.mstatus.modify(rv32i::csr::mstatus::mstatus::mie::SET);
        })
    }

}

impl parse::Chip for Chip {
    type Peripherals = crate::peripherals::Peripherals;
    type Systick = parse::SchedulerTimer<crate::timer::RvTimer>;

    fn peripherals(&self) -> Rc<Self::Peripherals> {
        self.peripherals.clone()
    }

    fn systick(&self) -> Result<Rc<Self::Systick>, parse::Error> {
        Ok(self.scheduler_timer.clone())
    }
}
