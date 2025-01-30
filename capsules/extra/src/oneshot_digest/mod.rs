// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

pub mod cshake;
pub mod hash;
pub mod hmac;
pub mod kmac;
pub mod shake;
mod utils;

use capsules_core::driver::NUM;

/// SHA-256 driver number
pub const DRIVER_NUM_SHA256: usize = NUM::OneshotSha256 as usize;
/// SHA-384 driver number
pub const DRIVER_NUM_SHA384: usize = NUM::OneshotSha384 as usize;
/// SHA-512 driver number
pub const DRIVER_NUM_SHA512: usize = NUM::OneshotSha512 as usize;
/// SHA3-224 driver number
pub const DRIVER_NUM_SHA3_224: usize = NUM::OneshotSha3_224 as usize;
/// SHA3-256 driver number
pub const DRIVER_NUM_SHA3_256: usize = NUM::OneshotSha3_256 as usize;
/// SHA3-384 driver number
pub const DRIVER_NUM_SHA3_384: usize = NUM::OneshotSha3_384 as usize;
/// SHA3-512 driver number
pub const DRIVER_NUM_SHA3_512: usize = NUM::OneshotSha3_512 as usize;
/// SHAKE-128 driver number
pub const DRIVER_NUM_SHAKE128: usize = NUM::OneshotShake128 as usize;
/// SHAKE-256 driver number
pub const DRIVER_NUM_SHAKE256: usize = NUM::OneshotShake256 as usize;
/// cSHAKE-128 driver number
pub const DRIVER_NUM_CSHAKE128: usize = NUM::OneshotCshake128 as usize;
/// cSHAKE-256 driver number
pub const DRIVER_NUM_CSHAKE256: usize = NUM::OneshotCshake256 as usize;
/// HMAC SHA-256 driver number
pub const DRIVER_NUM_HMAC_SHA256: usize = NUM::OneshotHmacSha256 as usize;
/// HMAC SHA-384 driver number
pub const DRIVER_NUM_HMAC_SHA384: usize = NUM::OneshotHmacSha384 as usize;
/// HMAC SHA-512 driver number
pub const DRIVER_NUM_HMAC_SHA512: usize = NUM::OneshotHmacSha512 as usize;
/// KMAC-128 driver number
pub const DRIVER_NUM_KMAC128: usize = NUM::OneshotKmac128 as usize;
/// KMAC-256 driver number
pub const DRIVER_NUM_KMAC256: usize = NUM::OneshotKmac256 as usize;
