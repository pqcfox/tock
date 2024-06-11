// This license header is required for submitting to upstream Tock.
// It is up to ZeroRISC to decide if this header should be here or not.
//
// Licensed under the Apache License, Version 2.0 or the MIT License.
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright Tock Contributors 2022.

use crate::uart::Uart;

use kernel::utilities::cells::OptionalCell;
use kernel::utilities::registers::{interfaces::Readable, ReadWrite};
use kernel::ErrorCode;

use core::fmt::Write;

struct TestWriter {
    uart: OptionalCell<&'static Uart<'static>>,
}

impl TestWriter {
    fn set_uart(&self, uart: &'static Uart) {
        self.uart.set(uart);
    }
}

impl Write for TestWriter {
    fn write_str(&mut self, string: &str) -> core::fmt::Result {
        self.uart.map(|uart| uart.transmit_sync(string.as_bytes()));
        Ok(())
    }
}

static mut TEST_WRITER: TestWriter = TestWriter {
    uart: OptionalCell::empty(),
};

macro_rules! println {
    ($msg:expr) => ({
        // If tests are running on host, there is no underlying Tock kernel, so this function becomes a
        // NOP
        if !cfg!(test) {
            // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
            unsafe {
                // The result is ignored for simplicity
                let _ = TEST_WRITER.write_fmt(format_args!("{}\r\n", $msg));
            }
        }
    });
    ($fmt:expr, $($arg:tt)+) => ({
        // If tests are running on host, there is no underlying Tock kernel, so this function becomes a
        // NOP
        if !cfg!(test) {
            // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
            unsafe {
                // The result is ignored for simplicity
                let _ = TEST_WRITER.write_fmt(format_args!("{}\r\n", format_args!($fmt, $($arg)+)));
            }
        }
    });
}

pub(super) fn print_test_header(message: &str) {
    println!("STARTING TEST: {}", message);
}

pub(super) fn print_test_footer(message: &str) {
    println!("FINISHED TEST: {}", message);
}

pub fn run_all(uart: &'static Uart<'static>) {
    // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
    unsafe { TEST_WRITER.set_uart(uart) };

    super::page_index::tests::run_all();
    super::page_position::tests::run_all();
    super::page::tests::run_all();
    super::flash_address::tests::run_all();
}
