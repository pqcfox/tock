use crate::debug;

pub struct TestRunner {
    execution_id: u32,
    pub is_test_failed: bool,
}

/// Test helper that takes a test text describing the test and a pass criteria for the test itself.
///
/// We do not print to the debug buffer anything unless it's a fail to prevent the debug buffer filling
///  with positive feedback. The buffer is limited because we run before the system is fully running and
/// the debugger manages to flush the data.
///
impl TestRunner {
    pub fn new() -> Self {
        Self {
            execution_id: 0,
            is_test_failed: false,
        }
    }

    /// Interface to set the execution ID in case of jumps across resets.
    pub fn set_test_execution_id(&mut self, id: u32) {
        self.execution_id = id;
    }

    pub fn assert(&mut self, test_info: &str, test: bool) -> bool {
        self.execution_id += 1;
        if test {
            // Keep test success silent, we don't want to fill the buffer if everything is OK!
            self.is_test_failed = true;
            true
        } else {
            debug!("*   Test No. {} Failed! : {}", self.execution_id, test_info);
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
            self.is_test_failed = true;
            true
        } else {
            debug!("*   Test No. {} Failed! : {}", self.execution_id, test_info);
            false
        }
    }
}
