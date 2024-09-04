#[derive(Debug)]
#[parse::component(serde, ident = "scheduler_timer")]
pub struct Systick;

impl parse::Component for Systick {
    fn ty(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        Ok(quote::quote!(
            kernel::platform::scheduler_timer::VirtualSchedulerTimer<
                capsules_core::virtualizers::virtual_alarm::VirtualMuxAlarm<'static, earlgrey::timer::RvTimer<'static, ChipConfig>>,
            >
        ))
    }

    fn init_expr(&self) -> Result<parse::proc_macro2::TokenStream, parse::Error> {
        let ty = self.ty()?;

        Ok(quote::quote! {
            kernel::static_init!(
                #ty,
                kernel::platform::scheduler_timer::VirtualSchedulerTimer::new(scheduler_timer_virtual_alarm),
            )
        })
    }
}
