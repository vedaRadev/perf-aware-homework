use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

#[link(name = "write_all_bytes_asm")]
extern "C" {
    fn mov_all_bytes_asm(len: u64, buf: *mut u8);
    fn nop_all_bytes_asm(len: u64, buf: *mut u8);
    fn cmp_all_bytes_asm(len: u64, buf: *mut u8);
    fn dec_all_bytes_asm(len: u64, buf: *mut u8);
}

const BUFFER_SIZE: usize = 2usize.pow(26); // 16mb

struct Params { buffer: Vec<u8> }

#[inline(never)]
#[no_mangle]
fn write_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let test_section = TimeTestSection::begin();
    for (index, element) in buffer.iter_mut().enumerate() {
        *element = index as u8;
    }
    test_section.end(buffer.len() as u64)
}

#[inline(never)]
#[no_mangle]
fn mov_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { mov_all_bytes_asm(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

#[inline(never)]
#[no_mangle]
fn nop_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { nop_all_bytes_asm(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

#[inline(never)]
#[no_mangle]
fn cmp_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { cmp_all_bytes_asm(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

#[inline(never)]
#[no_mangle]
fn dec_all_bytes(Params { buffer }: &mut Params) -> TimeTestResult {
    let len = buffer.len() as u64;
    let test_section = TimeTestSection::begin();
    unsafe { dec_all_bytes_asm(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}


fn main() {
    let mut buffer = vec![0u8; BUFFER_SIZE];
    buffer.fill(0); // pre-touch all the bytes so we don't incur page faults with the first test

    let mut repetition_tester = RepetitionTester::new(Params { buffer });
    repetition_tester.register_test(write_all_bytes, "write all bytes (control)");
    repetition_tester.register_test(mov_all_bytes, "mov all bytes");
    repetition_tester.register_test(nop_all_bytes, "nop all bytes");
    repetition_tester.register_test(cmp_all_bytes, "cmp all bytes");
    repetition_tester.register_test(dec_all_bytes, "dec all bytes");
    repetition_tester.run_tests();
}
