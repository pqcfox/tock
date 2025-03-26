// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Timeouts for cryptolib operations, in CPU cycles.

/// ECDSA P-256 verify operation timeout (~5X the normal operation time as
/// indicated in
/// https://opentitan.org/book/hw/ip/otbn/doc/otbn_intro.html#performance).
pub const ECDSA_P256_VERIFY_TIMEOUT: u64 = 2_100_000;
/// ECDSA P-384 verify operation timeout (~5X the normal operation time as
/// indicated in
/// https://opentitan.org/book/hw/ip/otbn/doc/otbn_intro.html#performance).
pub const ECDSA_P384_VERIFY_TIMEOUT: u64 = 5_400_000;
