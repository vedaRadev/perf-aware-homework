// TODO instead of tests panicking for whatever result, they should return a result containing the
// TimeTestResult or an error. If they return an error then the repetition tester should just make
// note of the error and continue with whatever the next test is.
// TODO buffer alloc switch within tests

use performance_metrics::{ read_cpu_timer, get_cpu_frequency_estimate };
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

struct TimeTestResult { cycles_elapsed: u64, bytes_processed: Option<u64> }
struct TimeTestParams<'a> {
    file_name: &'a str,
    file_size: u64,
    buffer: &'a mut Vec<u8>,
}

type TimeTest = fn(TimeTestParams) -> TimeTestResult;
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

    fn run_tests(&self, file_name: &str) {
        let cpu_freq = get_cpu_frequency_estimate(1000);
        let mut stdout = stdout();
        let max_cycles_to_wait = (MAX_WAIT_TIME_SECONDS * cpu_freq as f64) as u64;
        let file_size = fs::metadata(file_name).expect("failed to read file metadata").file_size();
        let mut buffer = vec![0u8; file_size as usize];

        for (do_test, test_name) in &self.tests {
            let mut total_cycles = 0u64;
            let mut total_bytes = 0u64;
            let mut cycles_since_last_min = 0u64;
            let mut min = TimeTestResult { cycles_elapsed: u64::MAX, bytes_processed: None };
            let mut max = TimeTestResult { cycles_elapsed: u64::MIN, bytes_processed: None };
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

                if test_result.cycles_elapsed > max.cycles_elapsed { max = test_result; }
                else if test_result.cycles_elapsed < min.cycles_elapsed {
                    cycles_since_last_min = 0;

                    // printing through stdout with print! and println! only actually flush the buffer when
                    // a newline is encountered. If we want to use carriage return and update a single line
                    // then we have to write to and flush stdout manually.
                    _ = stdout.write_all(&LINE_CLEAR);
                    let seconds = test_result.cycles_elapsed as f64 / cpu_freq as f64;
                    _ = stdout.write_all(format!(
                        "\rMin: {} ({:.4}ms)",
                        test_result.cycles_elapsed,
                        seconds * 1000.0
                    ).as_bytes());

                    if let Some(bytes_processed) = test_result.bytes_processed {
                        let gb_per_second = bytes_processed as f64 / GIGABYTES as f64 / seconds;
                        _ = stdout.write_all(format!(" {:.4}gb/s", gb_per_second).as_bytes());
                    }

                    _ = stdout.flush();

                    min = test_result;
                }

                if cycles_since_last_min > max_cycles_to_wait {
                    break;
                }
            }

            println!();

            let seconds = max.cycles_elapsed as f64 / cpu_freq as f64;
            print!("Max: {} ({:.4}ms)", max.cycles_elapsed, seconds * 1000.0);
            if let Some(bytes_processed) = max.bytes_processed {
                print!(" {:.4}gb/s", bytes_processed as f64 / GIGABYTES as f64 / seconds);
            }
            println!();

            let avg_cycles = total_cycles / iterations;
            let avg_seconds = avg_cycles as f64 / cpu_freq as f64;
            print!("Avg: {} ({:.4}ms)", avg_cycles, avg_seconds * 1000.0);
            if total_bytes > 0 {
                let avg_bytes = total_bytes / iterations;
                print!(" {:.4}gb/s", avg_bytes as f64 / GIGABYTES as f64 / avg_seconds);
            }
            println!();
            println!();
        }
    }
}

fn read_with_fs_read(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, .. } = params;

    let cycles_begin = read_cpu_timer();
    _ = fs::read(file_name);
    let cycles_elapsed = read_cpu_timer() - cycles_begin;

    TimeTestResult { cycles_elapsed, bytes_processed: Some(file_size) }
}

fn buffered_read(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, buffer } = params;

    let mut file = BufReader::new(fs::File::open(file_name).expect("Failed to open file"));

    let cycles_begin = read_cpu_timer();
    let bytes_read = file.read(&mut buffer[..]).expect("failed to read file") as u64;
    let cycles_elapsed = read_cpu_timer() - cycles_begin;
    if bytes_read != file_size {
        panic!("file read failed: expected to read {file_size} bytes but actually read {bytes_read}");
    }

    TimeTestResult { cycles_elapsed, bytes_processed: Some(file_size) }
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

    if result == 0 {
        let errno = unsafe { GetLastError() };
        panic!("(errno {}) Failed to read file {}", errno, file_name);
    }

    unsafe { CloseHandle(file_handle); }

    TimeTestResult { cycles_elapsed, bytes_processed: Some(bytes_read.into()) }
}

fn read_with_libc_fread(params: TimeTestParams) -> TimeTestResult {
    let TimeTestParams { file_name, file_size, buffer } = params;

    let file_name_cstr = CString::new(file_name).unwrap();
    let file_mode_cstr = CString::new("rb").unwrap();

    let file = unsafe { fopen(file_name_cstr.as_ptr(), file_mode_cstr.as_ptr()) };

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

    if result != 1 {
        panic!("failed to read file {}", file_name);
    }

    unsafe { fclose(file) };

    TimeTestResult { cycles_elapsed, bytes_processed: Some(file_size) }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let file_name = args.next().unwrap();

    let mut repetition_tester = RepetitionTester::new();
    repetition_tester.register_test(read_with_fs_read, "rust fs::read");
    repetition_tester.register_test(buffered_read, "rust buffered read");
    repetition_tester.register_test(read_with_libc_fread, "libc fread");
    repetition_tester.register_test(read_with_win_read, "windows read");
    loop {
        repetition_tester.run_tests(&file_name);
    }
}
