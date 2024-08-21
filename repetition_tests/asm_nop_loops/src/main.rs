use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

#[link(name = "nop_loops_asm")]
extern "C" {
    fn nop_3x1_all_bytes(len: u64, buf: *mut u8);
    fn nop_1x3_all_bytes(len: u64, buf: *mut u8);
    fn nop_1x9_all_bytes(len: u64, buf: *mut u8);
}

const BUFFER_SIZE: usize = 2usize.pow(26); // 16mb
struct Params { buffer: Vec<u8> }

#[inline(never)]
#[no_mangle]
fn test_nop_3x1_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_3x1_all_bytes(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

#[inline(never)]
#[no_mangle]
fn test_nop_1x3_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_1x3_all_bytes(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

#[inline(never)]
#[no_mangle]
fn test_nop_1x9_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_1x9_all_bytes(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn main() {
    let shared_params = Params { buffer: vec![0u8; BUFFER_SIZE] };
    let mut repetition_tester = RepetitionTester::new(shared_params);
    repetition_tester.register_test(test_nop_3x1_all_bytes, "nop 3x1 all bytes");
    repetition_tester.register_test(test_nop_1x3_all_bytes, "nop 1x3 all bytes");
    repetition_tester.register_test(test_nop_1x9_all_bytes, "nop 1x9 all bytes");
    repetition_tester.run_tests();
}
