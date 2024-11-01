// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::integrity_unblinded_checksum;
use crate::status_type;

pub mod ecdsa;

// Implement status decoder for `cryptolib_ecc::otcrypto_status_t`.
status_type!(cryptolib_ecc::otcrypto_status_t);

// Implement checksum population for `cryptolib_ecc::otcrypto_unblinded_key.
integrity_unblinded_checksum!(cryptolib_ecc::otcrypto_unblinded_key);
