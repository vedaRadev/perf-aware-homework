use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
    TimeTestFunction,
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
    io::{ BufReader, Read },
    os::windows::fs::MetadataExt,
    fs,
    slice,
    alloc::{ alloc, dealloc, Layout },
    ffi::CString,
};

// These tests highlight the fact that, due to page faults, reusing a buffer is typically faster
// than allocating new buffers everywhere (pretty obvious).

struct FileReadTestParams<'a> {
    file_name: String,
    file_size: u64,
    buffer: &'a mut [u8],
}

#[inline(never)]
#[no_mangle]
fn write_all_bytes(params: &mut FileReadTestParams) -> TimeTestResult {
    let FileReadTestParams { buffer, .. } = params;

    let test_section = TimeTestSection::begin();
    for (index, element) in buffer.iter_mut().enumerate() {
        *element = index as u8;
    }
    test_section.end(buffer.len() as u64)
}

#[inline(never)]
#[no_mangle]
fn read_with_fs_read(params: &mut FileReadTestParams) -> TimeTestResult {
    let FileReadTestParams { file_name, file_size, .. } = params;

    let test_section = TimeTestSection::begin();
    _ = fs::read(file_name);
    test_section.end(*file_size)
}

#[inline(never)]
#[no_mangle]
fn buffered_read(params: &mut FileReadTestParams) -> TimeTestResult {
    let FileReadTestParams { file_name, file_size, buffer } = params;
    let mut file = BufReader::new(fs::File::open(file_name).expect("Failed to open file"));

    let test_section = TimeTestSection::begin();
    let bytes_read = file.read(&mut buffer[..]).expect("failed to read file") as u64;
    let test_result = test_section.end(bytes_read);

    if bytes_read != *file_size {
        panic!("file read failed: expected to read {file_size} bytes but actually read {bytes_read}");
    }

    test_result
}

// NOTE This will fail if the file size exceeds 64 bits.
#[inline(never)]
#[no_mangle]
fn read_with_win_read(params: &mut FileReadTestParams) -> TimeTestResult {
    let FileReadTestParams { file_name, file_size, buffer } = params;
    let file_name_cstr = CString::new(file_name.clone()).expect("failed to create cstring version of filename");

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

    let test_section = TimeTestSection::begin();
    let result = unsafe {
        ReadFile(
            file_handle,
            buffer_ptr as *mut winapi::ctypes::c_void,
            *file_size as DWORD,
            &mut bytes_read,
            std::ptr::null_mut(),
        )
    };
    let test_result = test_section.end(bytes_read as u64);

    if result == 0 {
        let errno = unsafe { GetLastError() };
        panic!("(errno {}) Failed to read file {}", errno, file_name);
    }

    unsafe { CloseHandle(file_handle); }

    test_result
}

#[inline(never)]
#[no_mangle]
fn read_with_libc_fread(params: &mut FileReadTestParams) -> TimeTestResult {
    let FileReadTestParams { file_name, file_size, buffer } = params;
    let file_name_cstr = CString::new(file_name.clone()).unwrap();
    let file_mode_cstr = CString::new("rb").unwrap();
    let file = unsafe { fopen(file_name_cstr.as_ptr(), file_mode_cstr.as_ptr()) };

    let test_section = TimeTestSection::begin();
    let result = unsafe {
        fread(
            buffer.as_mut_ptr() as *mut libc::c_void,
            *file_size as usize,
            1,
            file
        )
    };
    let test_result = test_section.end(*file_size);

    if result != 1 {
        panic!("failed to read file {}", *file_name);
    }

    unsafe { fclose(file) };

    test_result
}

fn with_buffer_alloc(
    test: fn(&mut FileReadTestParams) -> TimeTestResult
) -> impl TimeTestFunction<FileReadTestParams<'static>> {
    Box::new(
        move |params: &mut FileReadTestParams| {
            let FileReadTestParams { file_size, file_name, .. } = params;
            let layout = Layout::array::<u8>(*file_size as usize).expect("Failed to create memory layout for u8 array");
            let buffer_start = unsafe { alloc(layout) };
            let buffer = unsafe { slice::from_raw_parts_mut(buffer_start, *file_size as usize) };

            let mut test_params = FileReadTestParams {
                file_size: *file_size,
                file_name: file_name.clone(),
                buffer,
            };
            let result = test(&mut test_params);

            unsafe { dealloc(buffer_start, layout); }

            result
        }
    )
}

fn main() {
    let mut args = std::env::args().skip(1);
    let file_name = args.next().unwrap();
    let file_size = fs::metadata(file_name.as_str()).expect("failed to read file metadata").file_size();
    let buffer_layout = Layout::array::<u8>(file_size as usize).expect("failed to create layout for u8 array");
    let buffer_start = unsafe { alloc(buffer_layout) };
    let buffer = unsafe { slice::from_raw_parts_mut(buffer_start, file_size as usize) };

    let shared_test_params = FileReadTestParams { file_name, file_size, buffer };
    let mut repetition_tester = RepetitionTester::new(shared_test_params);
    repetition_tester.register_test(write_all_bytes, "write to all bytes");
    repetition_tester.register_test(with_buffer_alloc(write_all_bytes), "write to all bytes with buffer alloc");
    repetition_tester.register_test(read_with_fs_read, "rust fs::read");
    repetition_tester.register_test(buffered_read, "rust buffered read");
    repetition_tester.register_test(buffered_read, "rust buffered read with buffer alloc");
    repetition_tester.register_test(read_with_libc_fread, "libc fread");
    repetition_tester.register_test(read_with_libc_fread, "libc fread with buffer alloc");
    repetition_tester.register_test(read_with_win_read, "windows read");
    repetition_tester.register_test(with_buffer_alloc(read_with_win_read), "windows read with buffer alloc");
    repetition_tester.run_tests();
}
