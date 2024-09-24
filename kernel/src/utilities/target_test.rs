// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.
//
// Copyright zeroRISC Inc.
// Confidential information of zeroRISC Inc. All rights reserved.

use core::fmt::Write;
use tock_cells::optional_cell::OptionalCell;

pub struct TestRunner<'a> {
    execution_id: u32,
    printer: OptionalCell<&'a dyn Fn(&str)>,
    pub is_test_failed: bool,
}

impl<'a> Write for TestRunner<'a> {
    fn write_str(&mut self, string: &str) -> core::fmt::Result {
        self.printer.map(|funct| funct(string));
        Ok(())
    }
}

/// Test helper that takes a test text describing the test and a pass criteria for the test itself.
///
/// We do not print to the debug buffer anything unless it's a fail to prevent the debug buffer filling
///  with positive feedback. The buffer is limited because we run before the system is fully running and
/// the debugger manages to flush the data.
///
impl<'a> TestRunner<'a> {
    pub fn new() -> Self {
        Self {
            execution_id: 0,
            printer: OptionalCell::empty(),
            is_test_failed: false,
        }
    }

    pub fn set_print_func(&self, print: &'a dyn Fn(&str)) {
        self.printer.set(print);
    }

    /// Interface to set the execution ID in case of jumps across resets.
    pub fn set_test_execution_id(&mut self, id: u32) {
        self.execution_id = id;
    }

    pub fn assert(&mut self, test_info: &str, test: bool) -> bool {
        self.execution_id += 1;
        if test {
            let id = self.execution_id;
            self.write_fmt(format_args!(
                "*   Test No. {} passed! : {}\r\n",
                id, test_info
            ))
            .unwrap();
            true
        } else {
            self.write_fmt(format_args!(
                "*  ERROR: Test No. {} failed!!! : {}\r\n",
                self.execution_id.clone(),
                test_info,
            ))
            .unwrap();
            self.is_test_failed = true;
            false
        }
    }

    /// Assert that takes a function as an argument. The return of the function tells us if the test passed or not.
    /// The test_info tells us the test text.
    ///
    /// The test_info is only printed on fail in order to avoid debugger buffer getting full when running before full
    /// OS init.
    pub fn assert_function(&mut self, test_info: &str, f: impl Fn() -> bool) -> bool {
        self.execution_id += 1;
        if f() {
            self.write_fmt(format_args!(
                "*   Test No. {} Passed! : {}\r\n",
                self.execution_id.clone(),
                test_info
            ))
            .unwrap();
            true
        } else {
            self.write_fmt(format_args!(
                "*   Test No. {} Failed! : {}\r\n",
                self.execution_id.clone(),
                test_info
            ))
            .unwrap();
            self.is_test_failed = true;
            false
        }
    }
}

pub trait TargetTests {
    fn test(&self) -> bool {
        true
    }
}
