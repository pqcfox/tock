// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//! Components for Flash
//!
//! Provides `FlashMux` / `FlashUser` (virtual flash) and
//! `InfoFlashMux` / `InfoFlashUser` (virtual info flash).
//!
//! Usage
//! -----
//! ```rust
//!    let mux_flash = components::flash::FlashMuxComponent::new(&base_peripherals.nvmc).finalize(
//!       components::flash_mux_component_static!(nrf52833::nvmc::Nvmc),
//!    );
//!
//!    let virtual_app_flash = components::flash::FlashUserComponent::new(mux_flash).finalize(
//!       components::flash_user_component_static!(nrf52833::nvmc::Nvmc),
//!    );
//!
//!    let mux_info_flash = components::info_flash::InfoFlashMuxComponent::new(&base_peripherals.nvmc).finalize(
//!       components::flash_mux_component_static!(nrf52833::nvmc::Nvmc),
//!    );
//!
//!    let virtual_app_info_flash = components::info_flash::InfoFlashUserComponent::new(mux_info_flash).finalize(
//!       components::info_flash_user_component_static!(nrf52833::nvmc::Nvmc),
//!    );
//! ```

use capsules_core::virtualizers::virtual_flash::{FlashUser, InfoFlashUser};
use capsules_core::virtualizers::virtual_flash::{MuxFlash, MuxInfoFlash};
use core::mem::MaybeUninit;
use kernel::component::Component;
use kernel::hil::flash::{Flash, HasClient, HasInfoClient, InfoFlash};

// Setup static space for the objects.
#[macro_export]
macro_rules! flash_user_component_static {
    ($F:ty) => {{
        kernel::static_buf!(capsules_core::virtualizers::virtual_flash::FlashUser<'static, $F>)
    };};
}

#[macro_export]
macro_rules! flash_mux_component_static {
    ($F:ty) => {{
        kernel::static_buf!(capsules_core::virtualizers::virtual_flash::MuxFlash<'static, $F>)
    };};
}

pub struct FlashMuxComponent<F: 'static + Flash + HasClient<'static, MuxFlash<'static, F>>> {
    flash: &'static F,
}

impl<F: 'static + Flash + HasClient<'static, MuxFlash<'static, F>>> FlashMuxComponent<F> {
    pub fn new(flash: &'static F) -> FlashMuxComponent<F> {
        FlashMuxComponent { flash }
    }
}

impl<F: 'static + Flash + HasClient<'static, MuxFlash<'static, F>>> Component
    for FlashMuxComponent<F>
{
    type StaticInput = &'static mut MaybeUninit<MuxFlash<'static, F>>;
    type Output = &'static MuxFlash<'static, F>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let mux_flash = s.write(MuxFlash::new(self.flash));
        HasClient::set_client(self.flash, mux_flash);

        mux_flash
    }
}

pub struct FlashUserComponent<F: 'static + Flash + HasClient<'static, MuxFlash<'static, F>>> {
    mux_flash: &'static MuxFlash<'static, F>,
}

impl<F: 'static + Flash + HasClient<'static, MuxFlash<'static, F>>> FlashUserComponent<F> {
    pub fn new(mux_flash: &'static MuxFlash<'static, F>) -> Self {
        Self { mux_flash }
    }
}

impl<F: 'static + Flash + HasClient<'static, MuxFlash<'static, F>>> Component
    for FlashUserComponent<F>
{
    type StaticInput = &'static mut MaybeUninit<FlashUser<'static, F>>;
    type Output = &'static FlashUser<'static, F>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        s.write(FlashUser::new(self.mux_flash))
    }
}

// Setup static space for the objects.
#[macro_export]
macro_rules! info_flash_user_component_static {
    ($F:ty) => {{
        kernel::static_buf!(capsules_core::virtualizers::virtual_flash::InfoFlashUser<'static, $F>)
    };};
}

#[macro_export]
macro_rules! info_flash_mux_component_static {
    ($F:ty) => {{
        kernel::static_buf!(capsules_core::virtualizers::virtual_flash::MuxInfoFlash<'static, $F>)
    };};
}

pub struct InfoFlashMuxComponent<
    F: 'static + InfoFlash + HasInfoClient<'static, MuxInfoFlash<'static, F>>,
> where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    info_flash: &'static F,
}

impl<F: 'static + InfoFlash + HasInfoClient<'static, MuxInfoFlash<'static, F>>>
    InfoFlashMuxComponent<F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    pub fn new(info_flash: &'static F) -> InfoFlashMuxComponent<F> {
        InfoFlashMuxComponent { info_flash }
    }
}

impl<F: 'static + InfoFlash + HasInfoClient<'static, MuxInfoFlash<'static, F>>> Component
    for InfoFlashMuxComponent<F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    type StaticInput = &'static mut MaybeUninit<MuxInfoFlash<'static, F>>;
    type Output = &'static MuxInfoFlash<'static, F>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        let mux_info_flash = s.write(MuxInfoFlash::new(self.info_flash));
        HasInfoClient::set_info_client(self.info_flash, mux_info_flash);

        mux_info_flash
    }
}

pub struct InfoFlashUserComponent<
    F: 'static + InfoFlash + HasInfoClient<'static, MuxInfoFlash<'static, F>>,
> where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    mux_info_flash: &'static MuxInfoFlash<'static, F>,
}

impl<F: 'static + InfoFlash + HasInfoClient<'static, MuxInfoFlash<'static, F>>>
    InfoFlashUserComponent<F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    pub fn new(mux_info_flash: &'static MuxInfoFlash<'static, F>) -> Self {
        Self { mux_info_flash }
    }
}

impl<F: 'static + InfoFlash + HasInfoClient<'static, MuxInfoFlash<'static, F>>> Component
    for InfoFlashUserComponent<F>
where
    F::InfoType: Copy,
    F::BankType: Copy,
{
    type StaticInput = &'static mut MaybeUninit<InfoFlashUser<'static, F>>;
    type Output = &'static InfoFlashUser<'static, F>;

    fn finalize(self, s: Self::StaticInput) -> Self::Output {
        s.write(InfoFlashUser::new(self.mux_info_flash))
    }
}
