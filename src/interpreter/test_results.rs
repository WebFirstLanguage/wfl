use std::time::Duration;

#[derive(Debug, Default, Clone)]
pub struct TestResults {
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub failures: Vec<TestFailure>,
}

#[derive(Debug, Clone)]
pub struct TestFailure {
    pub describe_context: Vec<String>,
    pub test_name: String,
    pub assertion_message: String,
    pub duration: Duration,
    pub line: usize,
    pub column: usize,
}

pub enum TestResult {
    Pass {
        name: String,
        duration: Duration,
    },
    Fail {
        failure: TestFailure,
    },
}

impl TestResult {
    pub fn pass(name: &str, duration: Duration) -> Self {
        TestResult::Pass {
            name: name.to_string(),
            duration,
        }
    }

    pub fn fail(
        name: &str,
        message: String,
        duration: Duration,
        context: Vec<String>,
        line: usize,
        column: usize,
    ) -> Self {
        TestResult::Fail {
            failure: TestFailure {
                describe_context: context,
                test_name: name.to_string(),
                assertion_message: message,
                duration,
                line,
                column,
            },
        }
    }
}

impl TestResults {
    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        match result {
            TestResult::Pass { .. } => {
                self.passed_tests += 1;
            }
            TestResult::Fail { failure } => {
                self.failed_tests += 1;
                self.failures.push(failure);
            }
        }
    }
}
