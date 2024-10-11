use winapi::{
    ctypes::c_void,
    um::{
        winnt::{ MEM_RESERVE, MEM_RELEASE, MEM_COMMIT, PAGE_READWRITE },
        memoryapi::{ VirtualAlloc, VirtualFree }
    }
};

use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
    TestResults,
};

// For these tests I want to stay within the L1's data cache.
// AMD Ryzen 9 5900X has a 32kb data cache.
const PAGE_SIZE: usize = 4096;
const PAGE_COUNT: usize = 8;
const BUFFER_SIZE: usize = PAGE_COUNT * PAGE_SIZE;
const SUB_REGION_SIZE: u64 = 16_384; // 16kb
const READ_AMOUNT: u64 = 2u64.pow(30); // 1gb
const SUB_REGION_READS: u64 = READ_AMOUNT / SUB_REGION_SIZE;
const BYTES_PROCESSED_PER_TEST: u64 = SUB_REGION_SIZE * SUB_REGION_READS;

#[link(name = "load_alignment_offset")]
extern "C" {
    fn load_with_alignment_offset(
        // Size of the buffer subregion, must be a multiple of 256
        sub_region_size: u64,
        // How many times to read the subregion.
        sub_region_count: u64,
        alignment_offset: u64,
        buffer: *const u8
    );
}

fn do_load_alignment_test(buffer: *const u8, offset: u64) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe {
        load_with_alignment_offset(
            SUB_REGION_SIZE,
            SUB_REGION_READS,
            offset,
            buffer,
        );
    }
    section.end(BYTES_PROCESSED_PER_TEST)
}

macro_rules! test_with_offset {
    ($offset:expr) => {
        |buffer: &mut *const u8| -> TimeTestResult {
            do_load_alignment_test(*buffer, $offset)
        }
    }
}

fn main() {
    // Getting pages directly from Windows because I want to ensure that
    // the buffer is aligned to the start of a page.
    let buffer: *mut u8 = unsafe {
        VirtualAlloc(
            std::ptr::null_mut(),
            BUFFER_SIZE,
            MEM_RESERVE | MEM_COMMIT,
            PAGE_READWRITE
        ) as *mut u8
    };

    // Force OS to map buffer's pages
    const U8_MASK: u8 = 0xFF;
    for i in 0 .. BUFFER_SIZE {
        unsafe { *buffer.add(i) = i as u8 & U8_MASK; }
    }

    let mut repetition_tester = RepetitionTester::new(buffer as *const u8);
    repetition_tester.register_test(test_with_offset!(0), "0");
    repetition_tester.register_test(test_with_offset!(1), "1");
    repetition_tester.register_test(test_with_offset!(2), "2");
    repetition_tester.register_test(test_with_offset!(4), "4");
    repetition_tester.register_test(test_with_offset!(8), "8");
    repetition_tester.register_test(test_with_offset!(16), "16");
    repetition_tester.register_test(test_with_offset!(24), "24");
    repetition_tester.register_test(test_with_offset!(32), "32");
    repetition_tester.register_test(test_with_offset!(48), "48");
    repetition_tester.register_test(test_with_offset!(56), "56");
    repetition_tester.register_test(test_with_offset!(64), "64");

    println!("BUFFER TOTAL SIZE: {BUFFER_SIZE}");
    println!("SUB REGION SIZE: {SUB_REGION_SIZE}");
    println!("READ AMOUNT: {READ_AMOUNT}");
    println!("SUB REGION READS: {SUB_REGION_READS}");
    println!("BYTES PER TEST: {BYTES_PROCESSED_PER_TEST}");
    let results = repetition_tester.run_tests_and_collect_results();

    unsafe {
        VirtualFree(
            buffer as *mut c_void,
            BUFFER_SIZE,
            MEM_RELEASE
        );
    }

    let cpu_freq = results.cpu_freq;
    println!("offset, throughput (gb/s)");
    for (label, TestResults { min, .. }) in results.results {
        println!("{label}, {:.4}", min.get_gbs_throughput(cpu_freq));
    }
}
