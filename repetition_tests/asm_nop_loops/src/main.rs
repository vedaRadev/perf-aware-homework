use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

#[link(name = "nop_loops_asm")]
extern "C" {
    fn nop_3x1_all_bytes(len: u64, buf: *mut u8);
    fn nop_1x3_all_bytes(len: u64, buf: *mut u8);
    fn nop_3x3_all_bytes(len: u64, buf: *mut u8);
    fn nop_1x9_all_bytes(len: u64, buf: *mut u8);
    fn nop_5x3_all_bytes(len: u64, buf: *mut u8);
    fn nop_1x15_all_bytes(len: u64, buf: *mut u8);
}

const BUFFER_SIZE: usize = 2usize.pow(28); // 256mb
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
fn test_nop_3x3_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_3x3_all_bytes(len, buffer.as_mut_ptr()); }
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

#[inline(never)]
#[no_mangle]
fn test_nop_5x3_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_5x3_all_bytes(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

#[inline(never)]
#[no_mangle]
fn test_nop_1x15_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_1x15_all_bytes(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn main() {
    println!("buffer size: {BUFFER_SIZE}");

    let shared_params = Params { buffer: vec![0u8; BUFFER_SIZE] };
    let mut repetition_tester = RepetitionTester::new(shared_params);
    repetition_tester.register_test(test_nop_3x1_all_bytes, "nop 3x1 all bytes");
    repetition_tester.register_test(test_nop_1x3_all_bytes, "nop 1x3 all bytes");
    repetition_tester.register_test(test_nop_3x3_all_bytes, "nop 3x3 all bytes");
    repetition_tester.register_test(test_nop_1x9_all_bytes, "nop 1x9 all bytes");
    repetition_tester.register_test(test_nop_5x3_all_bytes, "nop 5x3 all bytes");
    repetition_tester.register_test(test_nop_1x15_all_bytes, "nop 1x15 all bytes");
    repetition_tester.run_tests();
}
