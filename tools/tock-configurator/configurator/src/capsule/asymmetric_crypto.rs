// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use std::rc::Rc;

use crate::menu::capsule_popup;
use crate::state::Data;
use parse::peripherals::{Chip, DefaultPeripherals};

macro_rules! config_asymmetric_crypto {
    {$peripheral:ident, $peripheral_func:ident, $on_submit:ident, $update:ident, $remove:ident, $config:ident, $config_unknown:ident, $name:expr} => {
        /// Menu for configuring the AsymmetricCrypto capsule.
        pub fn $config<C: Chip + 'static + serde::Serialize>(
            chip: Rc<C>,
            choice: Option<Rc<<<C as parse::peripherals::Chip>::Peripherals as DefaultPeripherals>::$peripheral>>,
        ) -> cursive::views::LinearLayout {
            match choice {
                None => $config_unknown(chip),
                Some(inner) => match chip.peripherals().$peripheral_func() {
                    Ok(peripherals) => {
                        capsule_popup::<C, _>(crate::views::radio_group_with_null_known(
                            Vec::from(peripherals),
                            $on_submit::<C>,
                            inner,
                        ))
                    }
                    Err(_) => capsule_popup::<C, _>(crate::menu::no_support($name)),
                },
            }
        }

        fn $config_unknown<C: Chip + 'static + serde::ser::Serialize>(
            chip: Rc<C>,
        ) -> cursive::views::LinearLayout {
            match chip.peripherals().$peripheral_func() {
                Ok(peripherals) => capsule_popup::<C, _>(crate::views::radio_group_with_null(
                    Vec::from(peripherals),
                    $on_submit::<C>,
                )),
                Err(_) => capsule_popup::<C, _>(crate::menu::no_support($name)),
            }
        }

        /// Configure asymmetric crypto based on the submitted configuration
        fn $on_submit<C: Chip + 'static + serde::ser::Serialize>(
            siv: &mut cursive::Cursive,
            submit: &Option<Rc<<C::Peripherals as DefaultPeripherals>::$peripheral>>,
        ) {
            if let Some(data) = siv.user_data::<Data<C>>() {
                if let Some(asymmetric_crypto) = submit {
                    data.platform.$update(asymmetric_crypto.clone());
                } else {
                    data.platform.$remove();
                }
            }
        }
    }
}

// Configuration code for P-256
config_asymmetric_crypto! {P256, p256, on_p256_submit, update_p256, remove_p256, config_p256, config_unknown_p256, "ASYMMETRIC CRYPTO P-256"}
// Configuration code for P-384
config_asymmetric_crypto! {P384, p384, on_p384_submit, update_p384, remove_p384, config_p384, config_unknown_p384, "ASYMMETRIC CRYPTO P-384"}
