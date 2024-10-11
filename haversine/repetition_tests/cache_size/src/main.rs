use winapi::um::{
    winnt::{ MEM_RESERVE, MEM_COMMIT, MEM_RELEASE, PAGE_READWRITE },
    memoryapi::{ VirtualAlloc, VirtualFree }
};
use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
    SuiteData,
    TestResults,
};

#[link(name = "cache_tests")]
extern "C" {
    fn read_buffer_power_of_two_mask(buffer_size: u64, buffer: *const u8, address_mask: u64);
    fn read_buffer_non_power_of_two(buffer: *const u8, sub_buffer_size_bytes: u64, sub_buffer_iterations: u64);
}

struct TestArgs {
    buffer_size: u64,
    buffer_start: *const u8,
}

// Pulled this so that I could debug the assembly routine.
#[inline(never)]
#[no_mangle]
/// buffer_size should be a power of two.
/// mask must be 2^n - 1.
fn do_power_of_two_cache_test(buffer_size: u64, buffer_start: *const u8, mask: u64) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe { read_buffer_power_of_two_mask(buffer_size, buffer_start, mask); }
    section.end(buffer_size)
}

#[inline(never)]
#[no_mangle]
/// buffer_size should be a power of two.
/// sub_buffer_size must be a multiple of 128.
fn do_non_power_of_two_cache_test(buffer_size: u64, buffer_start: *const u8, sub_buffer_size: u64) -> TimeTestResult {
    let sub_buffer_iterations = buffer_size / sub_buffer_size;
    // If sub_buffer_size isn't a power of two but buffer_size is then we won't be able to read
    // buffer_size bytes worth from the sub buffer. We'll actually be a little short, so we need to
    // adjust what we're reporting as the number of bytes we process.
    let bytes_processed = sub_buffer_size * sub_buffer_iterations;

    let section = TimeTestSection::begin();
    unsafe { read_buffer_non_power_of_two(buffer_start, sub_buffer_size, sub_buffer_iterations); }
    section.end(bytes_processed)
}

macro_rules! create_power_of_two_test {
    ($mask:expr) => {
        |args: &mut TestArgs| -> TimeTestResult {
            let TestArgs { buffer_size, buffer_start } = *args;
            do_power_of_two_cache_test(buffer_size, buffer_start, $mask)
        }
    }
}

macro_rules! create_non_power_of_two_test {
    ($sub_buffer_size:expr) => {
        |args: &mut TestArgs| -> TimeTestResult {
            let TestArgs { buffer_start, buffer_size } = *args;
            do_non_power_of_two_cache_test(buffer_size, buffer_start, $sub_buffer_size)
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

    repetition_tester.register_test(create_power_of_two_test!(MASK_4KB), "4kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_8KB), "8kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_16KB), "16kb");
    repetition_tester.register_test(create_non_power_of_two_test!(24_576), "24kb");
    repetition_tester.register_test(create_non_power_of_two_test!(30_720), "30kb");
    repetition_tester.register_test(create_non_power_of_two_test!(31_744), "31kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_32KB), "32kb");
    repetition_tester.register_test(create_non_power_of_two_test!(40_960), "40kb");
    repetition_tester.register_test(create_non_power_of_two_test!(49_152), "48kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_64KB), "64kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_128KB), "128kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_256KB), "256kb");
    repetition_tester.register_test(create_non_power_of_two_test!(307_200), "300kb");
    repetition_tester.register_test(create_non_power_of_two_test!(358_400), "350kb");
    repetition_tester.register_test(create_non_power_of_two_test!(409_600), "400kb");
    repetition_tester.register_test(create_non_power_of_two_test!(460_800), "450kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_512KB), "512kb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_1MB), "1mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_2MB), "2mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_4MB), "4mb");
    repetition_tester.register_test(create_non_power_of_two_test!(6_291_456), "6mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_8MB), "8mb");
    repetition_tester.register_test(create_non_power_of_two_test!(12_582_912), "12mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_16MB), "16mb");
    repetition_tester.register_test(create_non_power_of_two_test!(25_165_824), "24mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_32MB), "32mb");
    repetition_tester.register_test(create_non_power_of_two_test!(41_943_040), "40mb");
    repetition_tester.register_test(create_non_power_of_two_test!(50_331_648), "48mb");
    repetition_tester.register_test(create_non_power_of_two_test!(58_720_256), "56mb");
    repetition_tester.register_test(create_non_power_of_two_test!(62_914_560), "60mb");
    repetition_tester.register_test(create_non_power_of_two_test!(65_011_712), "62mb");
    repetition_tester.register_test(create_non_power_of_two_test!(66_060_288), "63mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_64MB), "64mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_128MB), "128mb");
    repetition_tester.register_test(create_power_of_two_test!(MASK_256MB), "256mb");

    // run tests and print out min results in a format that can be dumped into a table and graphed
    let SuiteData { cpu_freq, results } = repetition_tester.run_tests_and_collect_results();
    println!("size, throughput (gb/s)");
    for (test_name, TestResults { min, .. }) in results {
        println!("{test_name}, {:.4},", min.get_gbs_throughput(cpu_freq));
    }

    unsafe {
        VirtualFree(buffer_start as *mut winapi::ctypes::c_void, BUFFER_SIZE, MEM_RELEASE);
    }
}
