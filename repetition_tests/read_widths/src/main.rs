use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

#[link(name = "read_widths_asm")]
extern "C" {
    fn read_4x3(len: u64, array: *const u8);
    fn read_8x3(len: u64, array: *const u8);
    fn read_16x3(len: u64, array: *const u8);
    fn read_32x3(len: u64, array: *const u8);
}

#[allow(clippy::ptr_arg)]
fn test_read_4x3(buffer: &mut Vec<u8>) -> TimeTestResult {
    let len = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_4x3(len, buffer.as_ptr()); }
    section.end(len)
}

#[allow(clippy::ptr_arg)]
fn test_read_8x3(buffer: &mut Vec<u8>) -> TimeTestResult {
    let len = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_8x3(len, buffer.as_ptr()); }
    section.end(len)
}

#[allow(clippy::ptr_arg)]
fn test_read_16x3(buffer: &mut Vec<u8>) -> TimeTestResult {
    let len = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_16x3(len, buffer.as_ptr()); }
    section.end(len)
}

#[allow(clippy::ptr_arg)]
fn test_read_32x3(buffer: &mut Vec<u8>) -> TimeTestResult {
    let len = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_32x3(len, buffer.as_ptr()); }
    section.end(len)
}

fn main() {
    const BUFFER_SIZE: usize = 2usize.pow(24); // 16mb
    let buffer = vec![0u8; BUFFER_SIZE];

    let mut repetition_tester = RepetitionTester::new(buffer);
    repetition_tester.register_test(test_read_4x3, "3 4-byte (32-bit) reads");
    repetition_tester.register_test(test_read_8x3, "3 8-byte (64-bit) reads");
    repetition_tester.register_test(test_read_16x3, "3 16-byte (128-bit) reads");
    repetition_tester.register_test(test_read_32x3, "3 32-byte (256-bit) reads");
    repetition_tester.run_tests();
}
