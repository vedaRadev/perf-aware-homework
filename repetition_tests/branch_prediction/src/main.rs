use winapi::shared::bcrypt::{
    BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    BCryptGenRandom,
};

use repetition_tester::{
    RepetitionTester,
    TimeTestResult,
    TimeTestSection,
};

use std::cell::OnceCell;
use rand::prelude::*;

#[link(name = "conditional_nop_asm")]
extern "C" {
    fn conditional_nop(len: u64, buf: *mut u8);
}

const BUFFER_SIZE: usize = 2usize.pow(28); // 256mb

struct SharedParams { buffer: Vec<u8> }

fn branch_never(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    let len = buffer.len() as u64;
    buffer.fill(0);
    
    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn branch_always(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    let len = buffer.len() as u64;
    buffer.fill(1);

    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn branch_every_2(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    let len = buffer.len() as u64;
    for i in 0 .. len {
        buffer[i as usize] = (i % 2 == 0) as u8;
    }

    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn branch_every_3(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    let len = buffer.len() as u64;
    for i in 0 .. len {
        buffer[i as usize] = (i % 3 == 0) as u8;
    }

    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn branch_every_4(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    let len = buffer.len() as u64;
    for i in 0 .. len {
        buffer[i as usize] = (i % 4 == 0) as u8;
    }

    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn branch_rust_rand(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    // Using SmallRng because StdRng was so slow for some reason that it took way too long to even
    // do a single iteration of this test...
    static mut RNG: OnceCell<SmallRng> = OnceCell::new();
    // Have to do this for now because get_mut_or_init is unstable...
    let rng = match unsafe { RNG.get_mut() } {
        Some(rng) => rng,
        None => {
            unsafe {
                _ = RNG.set(SmallRng::seed_from_u64(0));
                RNG.get_mut().unwrap()
            }
        }
    };

    let len = buffer.len() as u64;
    rng.fill_bytes(buffer);

    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn branch_bcrypt_rand(SharedParams { buffer }: &mut SharedParams) -> TimeTestResult {
    let len = buffer.len() as u64;
    unsafe {
        let result = BCryptGenRandom(
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            len as u32,
            BCRYPT_USE_SYSTEM_PREFERRED_RNG
        );

        if result != 0 {
            panic!("bcrypt failed to fill buffer with random bytes!");
        }
    }

    let test_section = TimeTestSection::begin();
    unsafe { conditional_nop(len, buffer.as_mut_ptr()); }
    test_section.end(len)
}

fn main() {
    let buffer = vec![0u8; BUFFER_SIZE];
    let shared_params = SharedParams { buffer };

    let mut repetition_tester = RepetitionTester::new(shared_params);
    repetition_tester.register_test(branch_never, "branch never");
    repetition_tester.register_test(branch_always, "branch always");
    repetition_tester.register_test(branch_every_2, "branch every 2");
    repetition_tester.register_test(branch_every_3, "branch every 3");
    repetition_tester.register_test(branch_every_4, "branch every 4");
    repetition_tester.register_test(branch_rust_rand, "branch rust rand");
    repetition_tester.register_test(branch_bcrypt_rand, "branch bcrypt rand");
    repetition_tester.run_tests();
}
