// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// HIL for controlling key security level (e.g. using the OpenTitan cryptography
// library).

/// Key security level, as defined by the OpenTitan cryptography library.
///
/// https://opentitan.org/book/doc/security/cryptolib/cryptolib_api.html#bookkeeping-data-structures
#[derive(Clone, Copy, Debug)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
}
