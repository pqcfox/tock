// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::{peripherals::oneshot_digest, Capsule, Component};
use parse_macros::component;
use std::rc::Rc;

macro_rules! oneshot_digest_capsule {
    {
        capsule = $capsule:ident,
        board_capsule = $board_capsule:ident,
        board_module = $board_module:ident,
        capsule_ident = $capsule_ident:expr,
        hil = $hil:ident,
        driver_num = $driver_num:ident,
    } => {
        #[component(curr, ident = $capsule_ident)]
        pub struct $capsule<P: oneshot_digest::$hil + 'static> {
            peripheral: Rc<P>,
        }

        impl<P: oneshot_digest::$hil + 'static> $capsule<P> {
            #[inline]
            pub fn get(peripheral: Rc<P>) -> Rc<Self> {
                Rc::new(Self::new(peripheral))
            }
        }

        impl<P: oneshot_digest::$hil> Component for $capsule<P> {
            fn dependencies(&self) -> Option<Vec<Rc<dyn crate::Component>>> {
                Some(vec![self.peripheral.clone()])
            }

            fn ty(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
                let peripheral_ty = self.peripheral.ty()?;
                Ok(quote::quote!(
                    capsules_extra::oneshot_digest::$board_module::$board_capsule<#peripheral_ty>
                ))
            }

            fn init_expr(&self) -> Result<proc_macro2::TokenStream, crate::Error> {
                let peripheral_ident: proc_macro2::TokenStream = self.peripheral.ident()?.parse().unwrap();
                let ty = self.ty()?;
                let driver_num = self.driver_num();

                Ok(quote::quote!(kernel::static_init!(
                    #ty,
                    capsules_extra::oneshot_digest::$board_module::$board_capsule::new(
                        #peripheral_ident,
                        board_kernel.create_grant(#driver_num, &memory_allocation_cap)
                    ),
                )))
            }
        }

        impl<P: oneshot_digest::$hil> Capsule for $capsule<P> {
            fn driver_num(&self) -> proc_macro2::TokenStream {
                quote::quote!(capsules_extra::oneshot_digest::$driver_num)
            }
        }
    }
}
oneshot_digest_capsule! {
    capsule = OneshotSha256Capsule,
    board_capsule = OneshotSha256,
    board_module = hash,
    capsule_ident = "oneshot_sha256",
    hil = Sha256,
    driver_num = DRIVER_NUM_SHA256,
}
oneshot_digest_capsule! {
    capsule = OneshotSha384Capsule,
    board_capsule = OneshotSha384,
    board_module = hash,
    capsule_ident = "oneshot_sha384",
    hil = Sha384,
    driver_num = DRIVER_NUM_SHA384,
}
oneshot_digest_capsule! {
    capsule = OneshotSha512Capsule,
    board_capsule = OneshotSha512,
    board_module = hash,
    capsule_ident = "oneshot_sha512",
    hil = Sha512,
    driver_num = DRIVER_NUM_SHA512,
}
oneshot_digest_capsule! {
    capsule = OneshotSha3_224Capsule,
    board_capsule = OneshotSha3_224,
    board_module = hash,
    capsule_ident = "oneshot_sha3_224",
    hil = Sha3_224,
    driver_num = DRIVER_NUM_SHA3_224,
}
oneshot_digest_capsule! {
    capsule = OneshotSha3_256Capsule,
    board_capsule = OneshotSha3_256,
    board_module = hash,
    capsule_ident = "oneshot_sha3_256",
    hil = Sha3_256,
    driver_num = DRIVER_NUM_SHA3_256,
}
oneshot_digest_capsule! {
    capsule = OneshotSha3_384Capsule,
    board_capsule = OneshotSha3_384,
    board_module = hash,
    capsule_ident = "oneshot_sha3_384",
    hil = Sha3_384,
    driver_num = DRIVER_NUM_SHA3_384,
}
oneshot_digest_capsule! {
    capsule = OneshotSha3_512Capsule,
    board_capsule = OneshotSha3_512,
    board_module = hash,
    capsule_ident = "oneshot_sha3_512",
    hil = Sha3_512,
    driver_num = DRIVER_NUM_SHA3_512,
}
oneshot_digest_capsule! {
    capsule = OneshotShake128Capsule,
    board_capsule = OneshotShake128,
    board_module = shake,
    capsule_ident = "oneshot_shake128",
    hil = Shake128,
    driver_num = DRIVER_NUM_SHAKE128,
}
oneshot_digest_capsule! {
    capsule = OneshotShake256Capsule,
    board_capsule = OneshotShake256,
    board_module = shake,
    capsule_ident = "oneshot_shake256",
    hil = Shake256,
    driver_num = DRIVER_NUM_SHAKE256,
}
oneshot_digest_capsule! {
    capsule = OneshotCshake128Capsule,
    board_capsule = OneshotCshake128,
    board_module = cshake,
    capsule_ident = "oneshot_cshake128",
    hil = Cshake128,
    driver_num = DRIVER_NUM_CSHAKE128,
}
oneshot_digest_capsule! {
    capsule = OneshotCshake256Capsule,
    board_capsule = OneshotCshake256,
    board_module = cshake,
    capsule_ident = "oneshot_cshake256",
    hil = Cshake256,
    driver_num = DRIVER_NUM_CSHAKE256,
}
oneshot_digest_capsule! {
    capsule = OneshotHmacSha256Capsule,
    board_capsule = OneshotHmacSha256,
    board_module = hmac,
    capsule_ident = "oneshot_hmac_sha256",
    hil = HmacSha256,
    driver_num = DRIVER_NUM_HMAC_SHA256,
}
oneshot_digest_capsule! {
    capsule = OneshotHmacSha384Capsule,
    board_capsule = OneshotHmacSha384,
    board_module = hmac,
    capsule_ident = "oneshot_hmac_sha384",
    hil = HmacSha384,
    driver_num = DRIVER_NUM_HMAC_SHA384,
}
oneshot_digest_capsule! {
    capsule = OneshotHmacSha512Capsule,
    board_capsule = OneshotHmacSha512,
    board_module = hmac,
    capsule_ident = "oneshot_hmac_sha512",
    hil = HmacSha512,
    driver_num = DRIVER_NUM_HMAC_SHA512,
}
oneshot_digest_capsule! {
    capsule = OneshotKmac128Capsule,
    board_capsule = OneshotKmac128,
    board_module = kmac,
    capsule_ident = "oneshot_kmac128",
    hil = Kmac128,
    driver_num = DRIVER_NUM_KMAC128,
}
oneshot_digest_capsule! {
    capsule = OneshotKmac256Capsule,
    board_capsule = OneshotKmac256,
    board_module = kmac,
    capsule_ident = "oneshot_kmac256",
    hil = Kmac256,
    driver_num = DRIVER_NUM_KMAC256,
}
