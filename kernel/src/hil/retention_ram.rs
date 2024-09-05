// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::ErrorCode;

pub trait OwnerRetentionRam {
    type Data: Copy;
    type ID: Copy;

    fn read(&self, id: Self::ID) -> Result<Self::Data, ErrorCode>;

    fn write(&self, id: Self::ID, data: Self::Data) -> Result<(), ErrorCode>;
}

pub trait CreatorRetentionRam {
    type Data: Copy;
    type ID: Copy;

    fn read(&self, id: Self::ID) -> Result<Self::Data, ErrorCode>;
}
