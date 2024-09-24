// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// This license header has to be included to be able to submit it to Tock
// It is up to ZeroRISC to decide if it keeps this header or not

// trait specific to OpenTitan MCUs
// Client interface for capsules that are triggered by the AlertHandler peripheral
pub trait OpentTitanAlertHandlerClient {
    fn alert_happened(&self, alert: u32);
}
