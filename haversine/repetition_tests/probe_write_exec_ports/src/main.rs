use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

#[link(name = "writes_asm")]
extern "C" {
    fn write_x1(count: u64, buffer: *mut u8);
    fn write_x2(count: u64, buffer: *mut u8);
    fn write_x3(count: u64, buffer: *mut u8);
    fn write_x4(count: u64, buffer: *mut u8);
}

#[allow(clippy::ptr_arg)]
fn test_write_x1(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { write_x1(count, buffer.as_mut_ptr()); }
    section.end(count)
}

#[allow(clippy::ptr_arg)]
fn test_write_x2(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { write_x2(count, buffer.as_mut_ptr()); }
    section.end(count)
}

#[allow(clippy::ptr_arg)]
fn test_write_x3(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { write_x3(count, buffer.as_mut_ptr()); }
    section.end(count)
}

#[allow(clippy::ptr_arg)]
fn test_write_x4(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { write_x4(count, buffer.as_mut_ptr()); }
    section.end(count)
}

fn main() {
    const BUFFER_SIZE: usize = 2usize.pow(22); // 4096kb
    let buffer = vec![0u8; BUFFER_SIZE];

    let mut repetition_tester = RepetitionTester::new(buffer);
    repetition_tester.register_test(test_write_x1, "1 write per iteration");
    repetition_tester.register_test(test_write_x2, "2 writes per iteration");
    repetition_tester.register_test(test_write_x3, "3 writes per iteration");
    repetition_tester.register_test(test_write_x4, "4 writes per iteration");
    repetition_tester.run_tests();
}
