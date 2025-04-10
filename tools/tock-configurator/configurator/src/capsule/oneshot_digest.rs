// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

//!  Menus for configuring the OneshotDigest capsules.

macro_rules! oneshot_menu {
    {
        peripheral = $peripheral:ident,
        peripheral_ty = $peripheral_ty:ident,
        update = $update:ident,
        remove = $remove:ident,
        name = $name:expr,
    } => {
        pub mod $peripheral {
            use std::rc::Rc;
            use crate::menu::capsule_popup;
            use crate::state::Data;
            use parse::peripherals::{Chip, DefaultPeripherals};

            pub fn config<C: Chip + 'static + serde::Serialize>(
                chip: Rc<C>,
                choice: Option<
                    Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::$peripheral_ty>,
                >,
            ) -> cursive::views::LinearLayout {
                match choice {
                    None => config_unknown(chip),
                    Some(inner) => match chip.peripherals().$peripheral() {
                        Ok(peripherals) => {
                            capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                                Vec::from(peripherals),
                                on_submit::<C>,
                                inner,
                            ))
                        }
                        Err(_) => capsule_popup::<C, _>(crate::menu::no_support($name)),
                    },
                }
            }

            fn config_unknown<C: Chip + 'static + serde::ser::Serialize>(
                chip: Rc<C>,
            ) -> cursive::views::LinearLayout {
                match chip.peripherals().$peripheral() {
                    Ok(peripherals) => {
                        capsule_popup::<C, _>(crate::views::radio_group_with_null(
                            Vec::from(peripherals),
                            on_submit::<C>,
                        ))
                    }
                    Err(_) => capsule_popup::<C, _>(crate::menu::no_support($name)),
                }
            }

            /// Configure the given oneshot digest function based on the submitted peripheral.
            fn on_submit<C: Chip + 'static + serde::ser::Serialize>(
                siv: &mut cursive::Cursive,
                submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::$peripheral_ty>>,
            ) {
                if let Some(data) = siv.user_data::<Data<C>>() {
                    if let Some(s) = submit {
                        data.platform.$update(s.clone());
                    } else {
                        data.platform.$remove();
                    }
                }
            }
        }
    }
}
oneshot_menu! { peripheral = oneshot_sha256, peripheral_ty = OneshotSha256, update = update_oneshot_sha256, remove = remove_oneshot_sha256, name = "ONESHOT_SHA256", }
oneshot_menu! { peripheral = oneshot_sha384, peripheral_ty = OneshotSha384, update = update_oneshot_sha384, remove = remove_oneshot_sha384, name = "ONESHOT_SHA384", }
oneshot_menu! { peripheral = oneshot_sha512, peripheral_ty = OneshotSha512, update = update_oneshot_sha512, remove = remove_oneshot_sha512, name = "ONESHOT_SHA512", }
oneshot_menu! { peripheral = oneshot_sha3_224, peripheral_ty = OneshotSha3_224, update = update_oneshot_sha3_224, remove = remove_oneshot_sha3_224, name = "ONESHOT_SHA3_224", }
oneshot_menu! { peripheral = oneshot_sha3_256, peripheral_ty = OneshotSha3_256, update = update_oneshot_sha3_256, remove = remove_oneshot_sha3_256, name = "ONESHOT_SHA3_256", }
oneshot_menu! { peripheral = oneshot_sha3_384, peripheral_ty = OneshotSha3_384, update = update_oneshot_sha3_384, remove = remove_oneshot_sha3_384, name = "ONESHOT_SHA3_384", }
oneshot_menu! { peripheral = oneshot_sha3_512, peripheral_ty = OneshotSha3_512, update = update_oneshot_sha3_512, remove = remove_oneshot_sha3_512, name = "ONESHOT_SHA3_512", }
oneshot_menu! { peripheral = oneshot_shake128, peripheral_ty = OneshotShake128, update = update_oneshot_shake128, remove = remove_oneshot_shake128, name = "ONESHOT_SHAKE128", }
oneshot_menu! { peripheral = oneshot_shake256, peripheral_ty = OneshotShake256, update = update_oneshot_shake256, remove = remove_oneshot_shake256, name = "ONESHOT_SHAKE256", }
oneshot_menu! { peripheral = oneshot_cshake128, peripheral_ty = OneshotCshake128, update = update_oneshot_cshake128, remove = remove_oneshot_cshake128, name = "ONESHOT_CSHAKE128", }
oneshot_menu! { peripheral = oneshot_cshake256, peripheral_ty = OneshotCshake256, update = update_oneshot_cshake256, remove = remove_oneshot_cshake256, name = "ONESHOT_CSHAKE256", }
oneshot_menu! { peripheral = oneshot_hmac_sha256, peripheral_ty = OneshotHmacSha256, update = update_oneshot_hmac_sha256, remove = remove_oneshot_hmac_sha256, name = "ONESHOT_HMAC_SHA256", }
oneshot_menu! { peripheral = oneshot_hmac_sha384, peripheral_ty = OneshotHmacSha384, update = update_oneshot_hmac_sha384, remove = remove_oneshot_hmac_sha384, name = "ONESHOT_HMAC_SHA384", }
oneshot_menu! { peripheral = oneshot_hmac_sha512, peripheral_ty = OneshotHmacSha512, update = update_oneshot_hmac_sha512, remove = remove_oneshot_hmac_sha512, name = "ONESHOT_HMAC_SHA512", }
oneshot_menu! { peripheral = oneshot_kmac128, peripheral_ty = OneshotKmac128, update = update_oneshot_kmac128, remove = remove_oneshot_kmac128, name = "ONESHOT_KMAC128", }
oneshot_menu! { peripheral = oneshot_kmac256, peripheral_ty = OneshotKmac256, update = update_oneshot_kmac256, remove = remove_oneshot_kmac256, name = "ONESHOT_KMAC256", }
