use crate::debug;
use core::fmt::Write;
use tock_cells::optional_cell::OptionalCell;

macro_rules! println {
    ($msg:expr) => ({
        // If tests are running on host, there is no underlying Tock kernel, so this function becomes a
        // NOP
        if !cfg!(test) {
            // SAFETY: Tock is mono threaded, so mutating a static variable is safe.
            unsafe {
                // The result is ignored for simplicity
                let _ = self.write_fmt(format_args!("{}\r\n", $msg));
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
                let _ = self.write_fmt(format_args!("{}\r\n", format_args!($fmt, $($arg)+)));
            }
        }
    });
}

pub struct TestRunner<'a> {
    execution_id: u32,
    printer: OptionalCell<&'a dyn Fn(&'a str)>,
    pub is_test_failed: bool,
}

// impl Write for TestRunner {
//     fn write_str(&mut self, string: &str) -> core::fmt::Result {
//         self.uart.map(|uart| uart.transmit_sync(string.as_bytes()));
//         Ok(())
//     }
// }

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

    pub fn set_print_func(&self, print: &'a dyn Fn(&'a str)) {
        self.printer.set(print);
    }

    fn print(&self, string: &'a str) {
        self.printer.map(|funct| funct(string));
    }

    /// Interface to set the execution ID in case of jumps across resets.
    pub fn set_test_execution_id(&mut self, id: u32) {
        self.execution_id = id;
    }

    pub fn assert(&mut self, test_info: &str, test: bool) -> bool {
        self.execution_id += 1;
        if test {
            // Keep test success silent, we don't want to fill the buffer if everything is OK!
            self.print("*   Test No. {} passed! : {}");
            true
        } else {
            self.print("*   Test No. {} Failed! : {}");
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
            // Keep test success silent, we don't want to fill the buffer if everything is OK!
            true
        } else {
            self.print("*   Test No. {} Failed! : {}", self.execution_id, test_info);
            self.is_test_failed = true;
            false
        }
    }
}
