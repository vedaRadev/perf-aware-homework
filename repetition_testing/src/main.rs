// TODO instead of tests panicking for whatever result, they should return a result containing the
// TimeTestResult or an error. If they return an error then the repetition tester should just make
// note of the error and continue with whatever the next test is.
//
// TODO pass the repetition tester into tests and use helper functions to collect data.
// Then we could just collect the same exact info for every test without having to write all the
// boilerplate of doing so within the tests themselves.

use performance_metrics::{
    read_cpu_timer,
    get_cpu_frequency_estimate,
    read_os_page_fault_count,
};
use winapi::{
    shared::minwindef::DWORD,
    um::{
        errhandlingapi::GetLastError,
        handleapi::{
            INVALID_HANDLE_VALUE,
            CloseHandle
        },
        fileapi::{
            OPEN_EXISTING,
            CreateFileA,
            ReadFile,
        },
        winnt::{
            GENERIC_READ,
            FILE_SHARE_READ,
            FILE_SHARE_WRITE,
            FILE_ATTRIBUTE_NORMAL,
        },
    }
};
use libc::{ fread, fopen, fclose };
use std::{
    io::{ stdout, Write, BufReader, Read },
    os::windows::fs::MetadataExt,
    fs,
    ffi::CString,
};

/// Number of seconds to wait for a new hi/lo count before ending the test.
const MAX_WAIT_TIME_SECONDS: f64 = 5.0;
const MEGABYTES: u64 = 1024 * 1024;
const GIGABYTES: u64 = MEGABYTES * 1024;
const LINE_CLEAR: [u8; 64] = [b' '; 64];

#[derive(Default)]
struct TimeTestResult {
    cycles_elapsed: u64,
    bytes_processed: Option<u64>,
    page_faults: Option<u64>,
}

impl TimeTestResult {
    fn print_result(&self, cpu_freq: u64) {
        let mut stdout = stdout();

        let seconds = self.cycles_elapsed as f64 / cpu_freq as f64;
        _ = stdout.write_all(format!(
                "{} ({:.4}ms)",
                self.cycles_elapsed,
                seconds * 1000.0
        ).as_bytes());

        if let Some(bytes_processed) = self.bytes_processed {
            let gb_per_second = bytes_processed as f64 / GIGABYTES as f64 / seconds;
            _ = stdout.write_all(format!(" {:.4}gb/s", gb_per_second).as_bytes());
        }

        if let Some(page_faults) = self.page_faults {
            _ = stdout.write_all(format!(" {:.4}pf", page_faults).as_bytes());
            if page_faults > 0 {
                if let Some(bytes_processed) = self.bytes_processed {
                    _ = stdout.write_all(format!(" ({:.4}k/pf)", bytes_processed as f64 / page_faults as f64 / 1024.0).as_bytes());
                }
            }
        }
    }
}

struct TimeTestParams<'a> {
    file_name: &'a str,
    file_size: u64,
    buffer: &'a mut Vec<u8>,
}

type TimeTest = Box<dyn Fn(TimeTestParams) -> TimeTestResult>;
struct RepetitionTester {
    tests: Vec<(TimeTest, &'static str)>
}

impl RepetitionTester {
    fn new() -> Self {
        Self {
            tests: Vec::new()
        }
    }

    #[inline(always)]
    fn register_test(&mut self, test: TimeTest, test_name: &'static str) {
        self.tests.push((test, test_name));
    }

    fn run_tests(&self, file_name: &str) -> ! {
        let cpu_freq = get_cpu_frequency_estimate(1000);
        let mut stdout = stdout();
        let max_cycles_to_wait = (MAX_WAIT_TIME_SECONDS * cpu_freq as f64) as u64;
        let file_size = fs::metadata(file_name).expect("failed to read file metadata").file_size();
        let mut buffer = vec![0u8; file_size as usize];

        loop {
            for (do_test, test_name) in &self.tests {
                let mut total_cycles = 0u64;
                let mut total_bytes = 0u64;
                let mut total_page_faults = 0u64;
                let mut cycles_since_last_min = 0u64;
                let mut min = TimeTestResult { cycles_elapsed: u64::MAX, ..Default::default() };
                let mut max = TimeTestResult { cycles_elapsed: u64::MIN, ..Default::default() };
                let mut iterations = 0;

                println!("====== {test_name} ======");
                loop {
                    iterations += 1;

                    let test_result = do_test(TimeTestParams { file_name, file_size, buffer: &mut buffer });
                    cycles_since_last_min += test_result.cycles_elapsed;
                    total_cycles += test_result.cycles_elapsed;
                    if let Some(bytes_processed) = test_result.bytes_processed {
                        total_bytes += bytes_processed;
                    }
                    if let Some(page_faults) = test_result.page_faults {
                        total_page_faults += page_faults;
                    }

                    if test_result.cycles_elapsed > max.cycles_elapsed { max = test_result; }
                    else if test_result.cycles_elapsed < min.cycles_elapsed {
                        cycles_since_last_min = 0;

                        // printing through stdout with print! and println! only actually flush the buffer when
                        // a newline is encountered. If we want to use carriage return and update a single line
                        // then we have to write to and flush stdout manually.
                        _ = stdout.write_all(&LINE_CLEAR);
                        _ = stdout.write_all(b"\rMin: ");
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
                TimeTestResult {
                    cycles_elapsed: total_cycles / iterations,
                    bytes_processed: Some(total_bytes / iterations),
                    page_faults: Some(total_page_faults / iterations),
                }.print_result(cpu_freq);
                println!();
                println!();
            }
        }
    }
}

fn read_with_fs_read(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, .. } = params;

    let page_faults_begin = read_os_page_fault_count();
    let cycles_begin = read_cpu_timer();
    _ = fs::read(file_name);
    let cycles_elapsed = read_cpu_timer() - cycles_begin;
    let page_faults = read_os_page_fault_count() - page_faults_begin;

    TimeTestResult {
        cycles_elapsed,
        bytes_processed: Some(file_size),
        page_faults: Some(page_faults),
    }
}

fn buffered_read(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, buffer } = params;

    let mut file = BufReader::new(fs::File::open(file_name).expect("Failed to open file"));

    let page_faults_begin = read_os_page_fault_count();
    let cycles_begin = read_cpu_timer();
    let bytes_read = file.read(&mut buffer[..]).expect("failed to read file") as u64;
    let cycles_elapsed = read_cpu_timer() - cycles_begin;
    let page_faults = read_os_page_fault_count() - page_faults_begin;
    if bytes_read != file_size {
        panic!("file read failed: expected to read {file_size} bytes but actually read {bytes_read}");
    }

    TimeTestResult {
        cycles_elapsed,
        bytes_processed: Some(file_size),
        page_faults: Some(page_faults),
    }
}

// NOTE This will fail if the file size exceeds 64 bits.
fn read_with_win_read(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, buffer } = params;
    let file_name_cstr = CString::new(file_name).expect("failed to create cstring version of filename");

    let file_handle = unsafe {
        CreateFileA(
            file_name_cstr.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };

    if file_handle == INVALID_HANDLE_VALUE {
        let errno = unsafe { GetLastError() };
        panic!("(errno {}) Obtained invalid handle, failed to read file {}", errno, file_name);
    }

    let buffer_ptr = buffer.as_mut_ptr();
    let mut bytes_read: DWORD = 0;

    let page_faults_begin = read_os_page_fault_count();
    let cycles_begin = read_cpu_timer();
    let result = unsafe {
        ReadFile(
            file_handle,
            buffer_ptr as *mut winapi::ctypes::c_void,
            file_size as DWORD,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    let cycles_elapsed = read_cpu_timer() - cycles_begin;
    let page_faults = read_os_page_fault_count() - page_faults_begin;

    if result == 0 {
        let errno = unsafe { GetLastError() };
        panic!("(errno {}) Failed to read file {}", errno, file_name);
    }

    unsafe { CloseHandle(file_handle); }

    TimeTestResult {
        cycles_elapsed,
        bytes_processed: Some(bytes_read.into()),
        page_faults: Some(page_faults),
    }
}

fn read_with_libc_fread(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, buffer } = params;

    let file_name_cstr = CString::new(file_name).unwrap();
    let file_mode_cstr = CString::new("rb").unwrap();

    let file = unsafe { fopen(file_name_cstr.as_ptr(), file_mode_cstr.as_ptr()) };

    let page_faults_begin = read_os_page_fault_count();
    let cycles_begin = read_cpu_timer();
    let result = unsafe {
        fread(
            buffer.as_mut_ptr() as *mut libc::c_void,
            file_size as usize,
            1,
            file
        )
    };
    let cycles_elapsed = read_cpu_timer() - cycles_begin;
    let page_faults = read_os_page_fault_count() - page_faults_begin;

    if result != 1 {
        panic!("failed to read file {}", file_name);
    }

    unsafe { fclose(file) };

    TimeTestResult {
        cycles_elapsed,
        bytes_processed: Some(file_size),
        page_faults: Some(page_faults),
    }
}

fn write_to_all_bytes(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { buffer, .. } = params;

    let page_faults_begin = read_os_page_fault_count();
    let cycles_begin = read_cpu_timer();

    buffer.fill(0xFF);

    let cycles_elapsed = read_cpu_timer() - cycles_begin;
    let page_faults = read_os_page_fault_count() - page_faults_begin;

    TimeTestResult {
        cycles_elapsed,
        bytes_processed: Some(buffer.len() as u64),
        page_faults: Some(page_faults)
    }
}

fn with_buffer_alloc(test: fn(TimeTestParams) -> TimeTestResult) -> TimeTest {
    Box::new(
        move |params: TimeTestParams| {
            let TimeTestParams { file_size, file_name, .. } = params;
            let mut buffer = vec![0u8; file_size as usize];

            test(TimeTestParams { file_size, file_name, buffer: &mut buffer })
        }
    )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let file_name = args.next().unwrap();

    let mut repetition_tester = RepetitionTester::new();
    repetition_tester.register_test(Box::new(write_to_all_bytes), "write to all bytes");
    repetition_tester.register_test(Box::new(read_with_fs_read), "rust fs::read");
    repetition_tester.register_test(Box::new(buffered_read), "rust buffered read");
    repetition_tester.register_test(with_buffer_alloc(buffered_read), "rust buffered read with buffer alloc");
    repetition_tester.register_test(Box::new(read_with_libc_fread), "libc fread");
    repetition_tester.register_test(with_buffer_alloc(read_with_libc_fread), "libc fread with buffer alloc");
    repetition_tester.register_test(Box::new(read_with_win_read), "windows read");
    repetition_tester.register_test(with_buffer_alloc(read_with_win_read), "windows read with buffer alloc");
    repetition_tester.run_tests(&file_name);
}
