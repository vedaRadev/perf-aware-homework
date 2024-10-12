use winapi::{
    ctypes::c_void,
    um::{
        winnt::{ MEM_RESERVE, MEM_RELEASE, MEM_COMMIT, PAGE_READWRITE },
        memoryapi::{ VirtualAlloc, VirtualFree }
    }
};

use const_format::concatcp;

use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
    TestResults,
};

const BUFFER_SIZE: usize = 2usize.pow(29);  // 256 mb
const READ_AMOUNT: u64 = 2u64.pow(30);      // 1gb

// These sizes were selected based on my Ryzen 9 5900X
const WITHIN_L1_DATA_CACHE: u64 = 16_384;   // stay within L1 data cache
const WITHIN_L2_CACHE: u64 = 131_072;       // stay within L2 cache
const WITHIN_L3_CACHE: u64 = 2u64.pow(25);  // stay within L3 cache
const MAIN_MEMORY: u64 = 2u64.pow(28);      // force trip to main memory

#[link(name = "load_alignment_offset")]
extern "C" {
    fn read_buffer_multiple_times(
        buffer: *const u8,
        sub_region_size: u64,
        sub_region_reads: u64,
    );
}

#[inline(never)]
fn do_load_alignment_test(
    base_pointer: *const u8,
    // size of region to read from buffer, must be a multiple of 128
    sub_region_size: u64,
    // number of times to read region
    sub_region_reads: u64,
    // didn't really have to include this since it can be easily
    // calc'd from the previous two args but there's really no
    // reason to calculate that value every test invocation...
    bytes_to_process: u64,
) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe {
        read_buffer_multiple_times(
            base_pointer,
            sub_region_size,
            sub_region_reads,
        );
    }
    section.end(bytes_to_process)
}

macro_rules! create_test_with_offset {
    ($sub_region_size:expr, $offset:expr) => {
        {
            let sub_region_size = $sub_region_size;
            let sub_region_reads = READ_AMOUNT / sub_region_size;
            let bytes_to_process = sub_region_size * sub_region_reads;
            move |buffer: &mut *const u8| -> TimeTestResult {
                do_load_alignment_test(
                    // Apply offset to buffer base pointer
                    unsafe { (*buffer).add($offset) },
                    sub_region_size,
                    sub_region_reads,
                    bytes_to_process,
                )
            }
        }
    }
}

// macro_rules! create_alignment_tests_for_size {
//     ($repetition_tester:ident, $sub_region_size:expr, $size_label:literal) => {
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 0), concat!($size_label, ", 0"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 1), concat!($size_label, ", 1"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 2), concat!($size_label, ", 2"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 4), concat!($size_label, ", 4"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 8), concat!($size_label, ", 8"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 16), concat!($size_label, ", 16"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 24), concat!($size_label, ", 24"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 32), concat!($size_label, ", 32"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 48), concat!($size_label, ", 48"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 56), concat!($size_label, ", 56"));
//         $repetition_tester.register_test(create_test_with_offset!($sub_region_size, 64), concat!($size_label, ", 64"));
//     }
// }

// The order that these tests are registered is CRITICAL to the formatting and correctness of the
// table that's output.
macro_rules! create_alignment_tests_for_offset {
    ($repetition_tester:ident, $offset:literal) => {
        $repetition_tester.register_test(
            create_test_with_offset!(WITHIN_L1_DATA_CACHE, $offset),
            concatcp!($offset, ", ", WITHIN_L1_DATA_CACHE)
        );

        $repetition_tester.register_test(
            create_test_with_offset!(WITHIN_L2_CACHE, $offset),
            concatcp!($offset, ", ", WITHIN_L2_CACHE)
        );

        $repetition_tester.register_test(
            create_test_with_offset!(WITHIN_L3_CACHE, $offset),
            concatcp!($offset, ", ", WITHIN_L3_CACHE)
        );

        $repetition_tester.register_test(
            create_test_with_offset!(MAIN_MEMORY, $offset),
            concatcp!($offset, ", ", MAIN_MEMORY)
        );
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
    create_alignment_tests_for_offset!(repetition_tester, 0usize);
    create_alignment_tests_for_offset!(repetition_tester, 1usize);
    create_alignment_tests_for_offset!(repetition_tester, 2usize);
    create_alignment_tests_for_offset!(repetition_tester, 4usize);
    create_alignment_tests_for_offset!(repetition_tester, 8usize);
    create_alignment_tests_for_offset!(repetition_tester, 16usize);
    create_alignment_tests_for_offset!(repetition_tester, 24usize);
    create_alignment_tests_for_offset!(repetition_tester, 32usize);
    create_alignment_tests_for_offset!(repetition_tester, 48usize);
    create_alignment_tests_for_offset!(repetition_tester, 56usize);
    create_alignment_tests_for_offset!(repetition_tester, 64usize);

    println!("BUFFER TOTAL SIZE: {BUFFER_SIZE}");
    println!("READ AMOUNT: {READ_AMOUNT}");
    let results = repetition_tester.run_tests_and_collect_results();

    unsafe {
        VirtualFree(
            buffer as *mut c_void,
            BUFFER_SIZE,
            MEM_RELEASE
        );
    }

    let cpu_freq = results.cpu_freq;
    // Using the repetition tester and trying to generate more complicated table-like outputs is
    // becoming cumbersome. It might be time to refactor the repetition tester again...
    println!("offset, {WITHIN_L1_DATA_CACHE}, {WITHIN_L2_CACHE}, {WITHIN_L3_CACHE}, {MAIN_MEMORY}");
    // 4 tests per offset
    for tests_for_alignment in results.results.chunks(4) {
        let offset = tests_for_alignment[0].0.split(',').next().expect("no entries in label after splitting on comma?");
        print!("{offset}");
        for (_, TestResults { min, .. }) in tests_for_alignment {
            print!(", {:.4}", min.get_gbs_throughput(cpu_freq));
        }
        println!();
    }
}
