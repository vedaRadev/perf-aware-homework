use winapi::um::{
    winnt::{ MEM_RESERVE, MEM_COMMIT, PAGE_READWRITE },
    memoryapi::VirtualAlloc
};
use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
    SuiteData,
    TestResults,
};

#[link(name = "read_buffer_masked_asm")]
extern "C" {
    fn read_buffer_masked(buffer_size: u64, buffer: *const u8, address_mask: u64);
}

struct TestArgs {
    buffer_size: u64,
    buffer_start: *const u8,
}

// Pulled this so that I could debug the assembly routine.
#[inline(never)]
#[no_mangle]
fn do_cache_test(buffer_size: u64, buffer_start: *const u8, mask: u64) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { read_buffer_masked(buffer_size, buffer_start, mask); }
    section.end(buffer_size)
}

macro_rules! create_test {
    ($mask:expr) => {
        |args: &mut TestArgs| -> TimeTestResult {
            let TestArgs { buffer_size, buffer_start } = *args;
            do_cache_test(buffer_size, buffer_start, $mask)
        }
    }
}

fn main() {
    // 256mb... should be larger than any typical L3 cache
    const BUFFER_SIZE: usize = 2usize.pow(28); 
    const PAGE_SIZE: usize = 4096;
    // BUFFER_SIZE and PAGE_SIZE should be powers of 2. Just being lazy with integer division here.
    const NUM_PAGES: usize = BUFFER_SIZE / PAGE_SIZE;

    // Using VirtualAlloc because I want to make sure the buffer is aligned to the start of a page.
    // Not freeing the memory because the repetition tester doesn't actually stop until the process is terminated.
    let buffer_start = unsafe { VirtualAlloc(std::ptr::null_mut(), BUFFER_SIZE, MEM_RESERVE | MEM_COMMIT, PAGE_READWRITE) };
    let buffer_start = buffer_start as *mut u8;

    // Buffer should already be zeroed because Windows but let's go ahead and get our buffer entirely mapped by the OS.
    for i in 0 .. NUM_PAGES {
        let addr = unsafe { buffer_start.add(i * PAGE_SIZE) };
        unsafe { *addr = 0xFF; }
    }

    let buffer_start = buffer_start as *const u8;

    const MASK_4KB: u64 = 2u64.pow(12) - 1;
    const MASK_8KB: u64 = 2u64.pow(13) - 1;
    const MASK_16KB: u64 = 2u64.pow(14) - 1;
    const MASK_32KB: u64 = 2u64.pow(15) - 1;
    const MASK_64KB: u64 = 2u64.pow(16) - 1;
    const MASK_128KB: u64 = 2u64.pow(17) - 1;
    const MASK_256KB: u64 = 2u64.pow(18) - 1;
    const MASK_512KB: u64 = 2u64.pow(19) - 1;
    const MASK_1MB: u64 = 2u64.pow(20) - 1;
    const MASK_2MB: u64 = 2u64.pow(21) - 1;
    const MASK_4MB: u64 = 2u64.pow(22) - 1;
    const MASK_8MB: u64 = 2u64.pow(23) - 1;
    const MASK_16MB: u64 = 2u64.pow(24) - 1;
    const MASK_32MB: u64 = 2u64.pow(25) - 1;
    const MASK_64MB: u64 = 2u64.pow(26) - 1;
    const MASK_128MB: u64 = 2u64.pow(27) - 1;
    const MASK_256MB: u64 = 2u64.pow(28) - 1;

    let test_args = TestArgs { buffer_size: BUFFER_SIZE as u64, buffer_start };
    let mut repetition_tester = RepetitionTester::new(test_args);
    repetition_tester.register_test(create_test!(MASK_4KB), "4kb");
    repetition_tester.register_test(create_test!(MASK_8KB), "8kb");
    repetition_tester.register_test(create_test!(MASK_16KB), "16kb");
    repetition_tester.register_test(create_test!(MASK_32KB), "32kb");
    repetition_tester.register_test(create_test!(MASK_64KB), "64kb");
    repetition_tester.register_test(create_test!(MASK_128KB), "128kb");
    repetition_tester.register_test(create_test!(MASK_256KB), "256kb");
    repetition_tester.register_test(create_test!(MASK_512KB), "512kb");
    repetition_tester.register_test(create_test!(MASK_1MB), "1mb");
    repetition_tester.register_test(create_test!(MASK_2MB), "2mb");
    repetition_tester.register_test(create_test!(MASK_4MB), "4mb");
    repetition_tester.register_test(create_test!(MASK_8MB), "8mb");
    repetition_tester.register_test(create_test!(MASK_16MB), "16mb");
    repetition_tester.register_test(create_test!(MASK_32MB), "32mb");
    repetition_tester.register_test(create_test!(MASK_64MB), "64mb");
    repetition_tester.register_test(create_test!(MASK_128MB), "128mb");
    repetition_tester.register_test(create_test!(MASK_256MB), "256mb");

    // run tests and print out min results in a format that can be dumped into a table and graphed
    let SuiteData { cpu_freq, results } = repetition_tester.run_tests_and_collect_results();
    println!("size, throughput (gb/s)");
    for (test_name, TestResults { min, .. }) in results {
        println!("{test_name}, {:.4},", min.get_gbs_throughput(cpu_freq));
    }
}
