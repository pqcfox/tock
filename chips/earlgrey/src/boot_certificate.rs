// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use kernel::{ErrorCode, ProcessId};

/// Wrapper around flash controller driver that reads boot
/// certificates from the flash info partition on Earlgrey
struct EarlgreyBootCertIO<'a>(&'a FlashCtrl<'a>);

impl BootCertificateReader for EarlgreyBootCertIO {
    /// Read a boot certificate stored in hardware. Which driver is
    /// used to retrieve the certificate is implementation-dependent.
    fn read_boot_certificate(calling_process: ProcessId) -> Result<(), ErrorCode>;
}



