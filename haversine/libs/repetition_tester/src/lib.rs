use std::io::{stdout, Write};
use performance_metrics::{
    read_cpu_timer,
    read_os_page_fault_count,
    get_cpu_frequency_estimate,
};

/// Number of seconds to wait for a new hi/lo count before ending the test.
const MAX_WAIT_TIME_SECONDS: f64 = 5.0;
const MEGABYTES: u64 = 1024 * 1024;
const GIGABYTES: u64 = MEGABYTES * 1024;
const LINE_CLEAR: [u8; 64] = [b' '; 64];

#[derive(Default)]
pub struct TimeTestResult {
    cycles_elapsed: u64,
    bytes_processed: u64,
    page_faults: u64,
}

impl TimeTestResult {
    pub fn get_gbs_throughput(&self, cpu_freq: u64) -> f64 {
        let seconds = self.cycles_elapsed as f64 / cpu_freq as f64;
        self.bytes_processed as f64 / GIGABYTES as f64 / seconds
    }

    fn print_result(&self, cpu_freq: u64) {
        let mut stdout = stdout();

        let seconds = self.cycles_elapsed as f64 / cpu_freq as f64;
        _ = stdout.write(format!(
                "{} ({:.4}ms)",
                self.cycles_elapsed,
                seconds * 1000.0
        ).as_bytes());

        let gb_per_second = self.bytes_processed as f64 / GIGABYTES as f64 / seconds;
        _ = stdout.write(format!(" {:.4}gb/s", gb_per_second).as_bytes());

        _ = stdout.write(format!(" {:.4}pf", self.page_faults).as_bytes());
        if self.page_faults > 0 && self.bytes_processed > 0 {
            let kb_per_page_fault = self.bytes_processed as f64 / self.page_faults as f64 / 1024.0;
            _ = stdout.write(format!(" ({:.4}k/pf)", kb_per_page_fault).as_bytes());
        }
    }
}

pub struct TimeTestSection {
    result: TimeTestResult,
}

impl TimeTestSection {
    #[inline(never)]
    pub fn begin() -> Self {
        let page_faults_begin = read_os_page_fault_count();
        let cycles_begin = read_cpu_timer();
        let result = TimeTestResult {
            cycles_elapsed: cycles_begin,
            page_faults: page_faults_begin,
            bytes_processed: 0,
        };

        Self { result }
    }

    #[inline(never)]
    pub fn end(mut self, bytes_processed: u64) -> TimeTestResult {
        self.result.cycles_elapsed = read_cpu_timer() - self.result.cycles_elapsed;
        self.result.page_faults = read_os_page_fault_count() - self.result.page_faults;
        self.result.bytes_processed = bytes_processed;

        self.result
    }
}

pub trait TimeTestFunction<TestParams>: Fn(&mut TestParams) -> TimeTestResult {}
impl<TestParams, T> TimeTestFunction<TestParams> for T where T: Fn(&mut TestParams) -> TimeTestResult {}

pub struct TestResults {
    pub min: TimeTestResult,
    pub avg: TimeTestResult,
    pub max: TimeTestResult
}

pub struct SuiteData {
    pub cpu_freq: u64,
    pub results: Vec<(&'static str, TestResults)>,
}

impl SuiteData {
    fn new(cpu_freq: u64) -> Self {
        Self { cpu_freq, results: Vec::new() }
    }
}

pub struct RepetitionTester<TestParams> {
    shared_test_params: TestParams,
    tests: Vec<(Box<dyn TimeTestFunction<TestParams>>, &'static str)>
}

impl<TestParams> RepetitionTester<TestParams> {
    pub fn new(shared_test_params: TestParams) -> Self {
        Self {
            shared_test_params,
            tests: Vec::new()
        }
    }

    #[inline(always)]
    pub fn register_test(&mut self, test: impl TimeTestFunction<TestParams> + 'static, test_name: &'static str) {
        self.tests.push((Box::new(test), test_name));
    }

    /// Run through all tests and collect results.
    /// Wait wait_time_seconds for a new min before moving on to the next test (NOTE: This is measured
    /// in cumulative TEST time, not general time passed).
    fn internal_run_tests(&mut self, wait_time_seconds: f64, cpu_freq: u64) -> SuiteData {
        let mut stdout = stdout();
        let max_cycles_to_wait = (wait_time_seconds * cpu_freq as f64) as u64;
        let mut suite_results = SuiteData::new(cpu_freq);

        for (do_test, test_name) in &self.tests {
            let mut cycles_since_last_min = 0u64;
            let mut total = TimeTestResult::default();
            let mut min = TimeTestResult { cycles_elapsed: u64::MAX, ..Default::default() };
            let mut max = TimeTestResult { cycles_elapsed: u64::MIN, ..Default::default() };
            let mut iterations = 0;

            println!("====== {test_name} ======");
            loop {
                iterations += 1;

                let test_result = do_test(&mut self.shared_test_params);
                total.cycles_elapsed += test_result.cycles_elapsed;
                total.bytes_processed += test_result.bytes_processed;
                total.page_faults += test_result.page_faults;

                cycles_since_last_min += test_result.cycles_elapsed;

                if test_result.cycles_elapsed > max.cycles_elapsed { max = test_result; }
                else if test_result.cycles_elapsed < min.cycles_elapsed {
                    cycles_since_last_min = 0;

                    // printing through stdout with print! and println! only actually flush the buffer when
                    // a newline is encountered. If we want to use carriage return and update a single line
                    // then we have to write to and flush stdout manually.
                    _ = stdout.write(&LINE_CLEAR);
                    _ = stdout.write(b"\rMin: ");
                    test_result.print_result(cpu_freq);
                    _ = stdout.flush();

                    min = test_result;
                }

                if cycles_since_last_min > max_cycles_to_wait {
                    break;
                }
            }

            println!();

            print!("Max: ");
            max.print_result(cpu_freq);
            println!();

            print!("Avg: ");
            let mut average = total;
            average.cycles_elapsed /= iterations;
            average.bytes_processed /= iterations;
            average.page_faults /= iterations;
            average.print_result(cpu_freq);
            println!();
            println!();

            suite_results.results.push((test_name, TestResults { min, avg: average, max }));
        }

        suite_results
    }

    /// Do one run of each test and return the collected results.
    pub fn run_tests_and_collect_results(&mut self) -> SuiteData {
        let cpu_freq = get_cpu_frequency_estimate(1000);
        println!("cpu frequency estimate: {cpu_freq}\n");

        self.internal_run_tests(MAX_WAIT_TIME_SECONDS, cpu_freq)
    }

    /// Run tests forever.
    pub fn run_tests(&mut self) -> ! {
        let cpu_freq = get_cpu_frequency_estimate(1000);
        println!("cpu frequency estimate: {cpu_freq}\n");

        loop { _ = self.internal_run_tests(MAX_WAIT_TIME_SECONDS, cpu_freq); }
    }
}
