// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright OxidOS Automotive SRL 2024
//
// Author: Irina Nita <irina.nita@oxidos.io>
// Author: Darius Jipa <darius.jipa@oxidos.io>

use super::{
    aes::Aes, alert_handler::AlertHandler, ble::BleAdvertisement, gpio::Gpio, pattgen::Pattgen,
    reset_manager::ResetManager, system_reset_controller::SystemResetController, timer::Timer,
    uart::Uart, usb::Usb, AsymmetricCrypto, Attestation, Cshake128, Cshake256, Flash, Hmac,
    HmacSha256, HmacSha384, HmacSha512, I2c, Kmac128, Kmac256, Rng, Sha256, Sha384, Sha3_224,
    Sha3_256, Sha3_384, Sha3_512, Sha512, Shake128, Shake256, Spi, Temperature,
};
use crate::Component;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, PartialEq, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct NoSupport;

impl std::fmt::Display for NoSupport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not Supported")
    }
}

impl Component for NoSupport {}

/// The [`DefaultPeripherals`] trait defines a type that contains all of a chip's supported
/// peripherals. For non-supported peripherals, the unit type `()` can serve as the placeholder
/// for the trait item.
pub trait DefaultPeripherals: Component {
    type Uart: Uart + 'static + for<'de> serde::Deserialize<'de> + serde::Serialize;
    type Timer: Timer + 'static + for<'de> serde::Deserialize<'de> + serde::Serialize;
    type Gpio: Gpio + 'static + PartialEq;
    type Spi: Spi + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type I2c: I2c + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type BleAdvertisement: BleAdvertisement
        + for<'de> serde::Deserialize<'de>
        + serde::Serialize
        + 'static;

    type Flash: Flash + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Temperature: Temperature + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Rng: Rng + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Hmac: Hmac + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Aes: Aes + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Pattgen: Pattgen + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type SystemResetController: SystemResetController
        + for<'de> serde::Deserialize<'de>
        + serde::Serialize
        + 'static;
    type AlertHandler: AlertHandler + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Usb: Usb + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type ResetManager: ResetManager + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type Attestation: Attestation + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha256: Sha256 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha384: Sha384 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha512: Sha512 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha3_224: Sha3_224 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha3_256: Sha3_256 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha3_384: Sha3_384 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotSha3_512: Sha3_512 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotShake128: Shake128 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotShake256: Shake256 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotCshake128: Cshake128 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotCshake256: Cshake256 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotHmacSha256: HmacSha256
        + for<'de> serde::Deserialize<'de>
        + serde::Serialize
        + 'static;
    type OneshotHmacSha384: HmacSha384
        + for<'de> serde::Deserialize<'de>
        + serde::Serialize
        + 'static;
    type OneshotHmacSha512: HmacSha512
        + for<'de> serde::Deserialize<'de>
        + serde::Serialize
        + 'static;
    type OneshotKmac128: Kmac128 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type OneshotKmac256: Kmac256 + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type P256: AsymmetricCrypto + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;
    type P384: AsymmetricCrypto + for<'de> serde::Deserialize<'de> + serde::Serialize + 'static;

    /// Return an array slice of pointers to the `Gpio` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn gpio(&self) -> Result<&[Rc<Self::Gpio>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `Uart` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn uart(&self) -> Result<&[Rc<Self::Uart>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `Timer` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn timer(&self) -> Result<&[Rc<Self::Timer>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `Spi` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn spi(&self) -> Result<&[Rc<Self::Spi>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `I2c` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn i2c(&self) -> Result<&[Rc<Self::I2c>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `BleAdvertisement` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn ble(&self) -> Result<&[Rc<Self::BleAdvertisement>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `Flash` peripherals or a [`crate::Error`]
    /// if the peripheralis is non-existent.
    fn flash(&self) -> Result<&[Rc<Self::Flash>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `Temperature` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn temp(&self) -> Result<&[Rc<Self::Temperature>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    /// Return an array slice of pointers to the `Rng` peripherals or a [`crate::Error`]
    /// if the peripheral is non-existent.
    fn rng(&self) -> Result<&[Rc<Self::Rng>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn hmac(&self) -> Result<&[Rc<Self::Hmac>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn aes(&self) -> Result<&[Rc<Self::Aes>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn pattgen(&self) -> Result<&[Rc<Self::Pattgen>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn system_reset_controller(&self) -> Result<&[Rc<Self::SystemResetController>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn alert_handler(&self) -> Result<&[Rc<Self::AlertHandler>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn usb(&self) -> Result<&[Rc<Self::Usb>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn reset_manager(&self) -> Result<&[Rc<Self::ResetManager>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn attestation(&self) -> Result<&[Rc<Self::Attestation>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha256(&self) -> Result<&[Rc<Self::OneshotSha256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha384(&self) -> Result<&[Rc<Self::OneshotSha384>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha512(&self) -> Result<&[Rc<Self::OneshotSha512>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha3_224(&self) -> Result<&[Rc<Self::OneshotSha3_224>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha3_256(&self) -> Result<&[Rc<Self::OneshotSha3_256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha3_384(&self) -> Result<&[Rc<Self::OneshotSha3_384>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_sha3_512(&self) -> Result<&[Rc<Self::OneshotSha3_512>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn oneshot_shake128(&self) -> Result<&[Rc<Self::OneshotShake128>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_shake256(&self) -> Result<&[Rc<Self::OneshotShake256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_cshake128(&self) -> Result<&[Rc<Self::OneshotCshake128>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_cshake256(&self) -> Result<&[Rc<Self::OneshotCshake256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_hmac_sha256(&self) -> Result<&[Rc<Self::OneshotHmacSha256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_hmac_sha384(&self) -> Result<&[Rc<Self::OneshotHmacSha384>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_hmac_sha512(&self) -> Result<&[Rc<Self::OneshotHmacSha512>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_kmac128(&self) -> Result<&[Rc<Self::OneshotKmac128>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
    fn oneshot_kmac256(&self) -> Result<&[Rc<Self::OneshotKmac256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn p256(&self) -> Result<&[Rc<Self::P256>], crate::Error> {
        Err(crate::Error::NoSupport)
    }

    fn p384(&self) -> Result<&[Rc<Self::P384>], crate::Error> {
        Err(crate::Error::NoSupport)
    }
}

/// The [`Chip`] trait defines a type that contains the default peripherals and optionally a systick
/// for the scheduler timer.
pub trait Chip: Component {
    type Peripherals: DefaultPeripherals
        + 'static
        + for<'de> serde::Deserialize<'de>
        + serde::Serialize;
    type Systick: for<'de> serde::Deserialize<'de> + serde::Serialize + 'static + Component;
    type Watchdog: for<'de> serde::Deserialize<'de> + serde::Serialize + 'static + Component;

    /// Return chip prelude code needed before booting the platform.
    /// If this returns Some, it should be called before setting up the platform
    /// and entering main loop.
    fn before_boot(&self) -> Option<proc_macro2::TokenStream> {
        None
    }

    /// Return a pointer to the chip's default peripherals.
    fn peripherals(&self) -> Rc<Self::Peripherals>;

    /// Return a pointer to the chip's systick.
    fn systick(&self) -> Result<Rc<Self::Systick>, crate::Error>;

    /// Return a pointer to the chip's watchdog.
    fn watchdog(&self) -> Result<Rc<Self::Watchdog>, crate::Error>;

    /// Returns a reference to the chip-specific peripheral / configuration
    /// data.
    fn peripheral_config(&self) -> Rc<RefCell<dyn crate::component::ConfigPeripherals>>;
}
