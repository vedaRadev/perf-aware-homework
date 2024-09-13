use repetition_tester::{
    RepetitionTester,
    TimeTestResult,
    TimeTestSection,
};

#[link(name = "reads_asm")]
extern "C" {
    fn read_x1(count: u64, buffer: *const u8);
    fn read_x2(count: u64, buffer: *const u8);
    fn read_x3(count: u64, buffer: *const u8);
    fn read_x4(count: u64, buffer: *const u8);
}

#[allow(clippy::ptr_arg)]
fn test_read_x1(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_x1(count, buffer.as_ptr()); }
    section.end(count)
}

#[allow(clippy::ptr_arg)]
fn test_read_x2(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_x2(count, buffer.as_ptr()); }
    section.end(count)
}

#[allow(clippy::ptr_arg)]
fn test_read_x3(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_x3(count, buffer.as_ptr()); }
    section.end(count)
}

#[allow(clippy::ptr_arg)]
fn test_read_x4(buffer: &mut Vec<u8>) -> TimeTestResult {
    let count = buffer.len() as u64;
    let section = TimeTestSection::begin();
    unsafe { read_x4(count, buffer.as_ptr()); }
    section.end(count)
}

fn main() {
    const BUFFER_SIZE: usize = 2usize.pow(22); // 4096kb
    let buffer = vec![0u8; BUFFER_SIZE];

    let mut repetition_tester = RepetitionTester::new(buffer);
    repetition_tester.register_test(test_read_x1, "1 read per iteration");
    repetition_tester.register_test(test_read_x2, "2 reads per iteration");
    repetition_tester.register_test(test_read_x3, "3 reads per iteration");
    repetition_tester.register_test(test_read_x4, "4 reads per iteration");
    repetition_tester.run_tests();
}
