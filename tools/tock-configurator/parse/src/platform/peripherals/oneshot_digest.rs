// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use super::NoSupport;

macro_rules! oneshot_digest_peripheral {
    {$hil:ident} => {
        pub trait $hil: crate::Component + std::fmt::Debug + std::fmt::Display {}
        impl $hil for NoSupport {}
    }
}
oneshot_digest_peripheral! {Sha256}
oneshot_digest_peripheral! {Sha384}
oneshot_digest_peripheral! {Sha512}
oneshot_digest_peripheral! {Sha3_224}
oneshot_digest_peripheral! {Sha3_256}
oneshot_digest_peripheral! {Sha3_384}
oneshot_digest_peripheral! {Sha3_512}
oneshot_digest_peripheral! {Shake128}
oneshot_digest_peripheral! {Shake256}
oneshot_digest_peripheral! {Cshake128}
oneshot_digest_peripheral! {Cshake256}
oneshot_digest_peripheral! {HmacSha256}
oneshot_digest_peripheral! {HmacSha384}
oneshot_digest_peripheral! {HmacSha512}
oneshot_digest_peripheral! {Kmac128}
oneshot_digest_peripheral! {Kmac256}
