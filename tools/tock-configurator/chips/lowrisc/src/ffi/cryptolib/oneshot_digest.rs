// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

macro_rules! oneshot_digest_driver {
    {
        driver = $driver:ident,
        driver_ident = $driver_ident:expr,
        hil = $hil:ident,
    } => {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        pub struct $driver {}

        impl $driver {
            pub(crate) fn new() -> Self {
                Self {}
            }
        }

        impl parse::Ident for $driver {
            fn ident(&self) -> Result<String, parse::Error> {
                Ok(String::from($driver_ident))
            }
        }

        impl parse::Component for $driver {
            fn ty(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
                Ok(quote::quote!(
                    lowrisc::ffi::cryptolib::oneshot_digest::OtCryptoOneshotDigest
                ))
            }

            fn init_expr(&self) -> Result<proc_macro2::TokenStream, parse::Error> {
                let ty = self.ty()?;

                Ok(quote::quote!(#ty))
            }
        }

        impl std::fmt::Display for $driver {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $driver_ident)
            }
        }

        impl parse::peripherals::$hil for $driver {}
    }
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha256,
    driver_ident = "otcrypto_oneshot_sha256",
    hil = Sha256,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha384,
    driver_ident = "otcrypto_oneshot_sha384",
    hil = Sha384,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha512,
    driver_ident = "otcrypto_oneshot_sha512",
    hil = Sha512,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha3_224,
    driver_ident = "otcrypto_oneshot_sha3_224",
    hil = Sha3_224,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha3_256,
    driver_ident = "otcrypto_oneshot_sha3_256",
    hil = Sha3_256,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha3_384,
    driver_ident = "otcrypto_oneshot_sha3_384",
    hil = Sha3_384,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotSha3_512,
    driver_ident = "otcrypto_oneshot_sha3_512",
    hil = Sha3_512,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotShake128,
    driver_ident = "otcrypto_oneshot_shake128",
    hil = Shake128,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotShake256,
    driver_ident = "otcrypto_oneshot_shake256",
    hil = Shake256,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotCshake128,
    driver_ident = "otcrypto_oneshot_cshake128",
    hil = Cshake128,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotCshake256,
    driver_ident = "otcrypto_oneshot_cshake256",
    hil = Cshake256,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotHmacSha256,
    driver_ident = "otcrypto_oneshot_hmac_sha256",
    hil = HmacSha256,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotHmacSha384,
    driver_ident = "otcrypto_oneshot_hmac_sha384",
    hil = HmacSha384,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotHmacSha512,
    driver_ident = "otcrypto_oneshot_hmac_sha512",
    hil = HmacSha512,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotKmac128,
    driver_ident = "otcrypto_oneshot_kmac128",
    hil = Kmac128,
}
oneshot_digest_driver! {
    driver = OtCryptoOneshotKmac256,
    driver_ident = "otcrypto_oneshot_kmac256",
    hil = Kmac256,
}
