// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use super::NoSupport;

pub trait SystemResetController: crate::Component + std::fmt::Debug + std::fmt::Display {}

impl SystemResetController for NoSupport {}
