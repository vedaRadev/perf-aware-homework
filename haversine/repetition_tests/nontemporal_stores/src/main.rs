use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

// Following sizes are based on zen 3 ryzen 9 5900x
#[allow(dead_code)]
const L1_DATA_CACHE_SIZE: u64 = 16_384;
#[allow(dead_code)]
const L2_CACHE_SIZE: u64 = 2u64.pow(19);
#[allow(dead_code)]
const L3_CACHE_SIZE: u64 = 2u64.pow(26);

const BUFFER_SIZE: usize = L3_CACHE_SIZE as usize;
const MAX_READ_SIZE: usize = 2usize.pow(30); // 1gb
const NUM_READ_LOOPS: usize = MAX_READ_SIZE / BUFFER_SIZE;
const BYTES_PROCESSED: usize = NUM_READ_LOOPS * BUFFER_SIZE;

#[link(name = "test_routines")]
extern "C" {
    fn temporal_stores(
        input_buffer_start: *const u8,
        output_buffer_start: *mut u8,
        buffer_size: u64,
        read_write_loops: u64,
    );

    fn nontemporal_stores(
        input_buffer_start: *const u8,
        output_buffer_start: *mut u8,
        buffer_size: u64,
        read_write_loops: u64,
    );
}

#[allow(dead_code)]
struct SharedTestParams {
    input_buffer: *const u8,
    output_buffer: *mut u8,
}

fn test_temporal_stores(params: &mut SharedTestParams) -> TimeTestResult {
    let SharedTestParams { input_buffer, output_buffer } = params;

    let section = TimeTestSection::begin();
    unsafe {
        temporal_stores(
            *input_buffer,
            *output_buffer,
            BUFFER_SIZE as u64,
            NUM_READ_LOOPS as u64,
        )
    }
    section.end(BYTES_PROCESSED as u64)
}

fn test_nontemporal_stores(params: &mut SharedTestParams) -> TimeTestResult {
    let SharedTestParams { input_buffer, output_buffer } = params;

    let section = TimeTestSection::begin();
    unsafe {
        nontemporal_stores(
            *input_buffer,
            *output_buffer,
            BUFFER_SIZE as u64,
            NUM_READ_LOOPS as u64,
        )
    }
    section.end(BYTES_PROCESSED as u64)
}

fn main() {
    let mut input_buffer = vec![0u8; BUFFER_SIZE];
    let mut output_buffer = vec![0u8; BUFFER_SIZE];
    for (index, item) in input_buffer.iter_mut().enumerate() {
        *item = index as u8 & u8::MAX;
    }
    output_buffer.fill(0);

    let shared_test_params = SharedTestParams {
        input_buffer: input_buffer.as_ptr(),
        output_buffer: output_buffer.as_mut_ptr(),
    };
    let mut repetition_tester = RepetitionTester::new(shared_test_params);
    repetition_tester.register_test(test_temporal_stores, "temporal stores");
    repetition_tester.register_test(test_nontemporal_stores, "nontemporal stores");

    println!("Processing {BYTES_PROCESSED} worth of reads/writes");
    println!("INPUT/OUTPUT BUFFER SIZE: {BUFFER_SIZE}");
    println!("L1 DATA CACHE SIZE: {L1_DATA_CACHE_SIZE}");
    println!("L2 CACHE SIZE: {L2_CACHE_SIZE}");
    println!("L3 CACHE SIZE: {L3_CACHE_SIZE}");
    repetition_tester.run_tests();
}
