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

const INPUT_BUFFER_SIZE: usize = L2_CACHE_SIZE as usize * 4;
const OUTPUT_BUFFER_SIZE: usize = (L3_CACHE_SIZE * 8) as usize;
const WRITES_PER_INPUT_VALUE: usize = OUTPUT_BUFFER_SIZE / INPUT_BUFFER_SIZE;
const OUTPUT_BUFFER_REMAINDER: usize = OUTPUT_BUFFER_SIZE - INPUT_BUFFER_SIZE * WRITES_PER_INPUT_VALUE;

#[link(name = "test_routines")]
extern "C" {
    fn temporal_stores(
        input_buffer_start: *const u8,
        input_buffer_size: u64,
        output_buffer_start: *mut u8,
        writes_per_input_value: u64,
    );

    fn nontemporal_stores(
        input_buffer_start: *const u8,
        input_buffer_size: u64,
        output_buffer_start: *mut u8,
        writes_per_input_value: u64,
    );
}

#[allow(dead_code)]
struct SharedTestParams {
    input_buffer: *const u8,
    input_buffer_size: usize,
    output_buffer: *mut u8,
    output_buffer_size: usize,
    writes_per_input_value: usize,
}

fn test_temporal_stores(params: &mut SharedTestParams) -> TimeTestResult {
    let SharedTestParams {
        input_buffer,
        input_buffer_size,
        output_buffer,
        writes_per_input_value,
        ..
    } = params;

    let section = TimeTestSection::begin();
    unsafe {
        temporal_stores(
            *input_buffer,
            *input_buffer_size as u64,
            *output_buffer,
            *writes_per_input_value as u64,
        )
    }
    section.end(*input_buffer_size as u64)
}

fn test_nontemporal_stores(params: &mut SharedTestParams) -> TimeTestResult {
    let SharedTestParams {
        input_buffer,
        input_buffer_size,
        output_buffer,
        writes_per_input_value,
        ..
    } = params;

    let section = TimeTestSection::begin();
    unsafe {
        nontemporal_stores(
            *input_buffer,
            *input_buffer_size as u64,
            *output_buffer,
            *writes_per_input_value as u64,
        )
    }
    section.end(*input_buffer_size as u64)
}

fn main() {
    let mut input_buffer = vec![0u8; INPUT_BUFFER_SIZE];
    let mut output_buffer = vec![0u8; OUTPUT_BUFFER_SIZE];
    for (index, item) in input_buffer.iter_mut().enumerate() {
        *item = index as u8 & u8::MAX;
    }
    output_buffer.fill(0);

    let shared_test_params = SharedTestParams {
        input_buffer: input_buffer.as_ptr(),
        input_buffer_size: INPUT_BUFFER_SIZE,
        output_buffer: output_buffer.as_mut_ptr(),
        output_buffer_size: OUTPUT_BUFFER_SIZE,
        writes_per_input_value: WRITES_PER_INPUT_VALUE,
    };
    let mut repetition_tester = RepetitionTester::new(shared_test_params);
    repetition_tester.register_test(test_temporal_stores, "temporal stores");
    repetition_tester.register_test(test_nontemporal_stores, "nontemporal stores");

    println!("Input buffer size: {INPUT_BUFFER_SIZE}");
    println!("Output buffer size: {OUTPUT_BUFFER_SIZE}");
    println!("Writes per input value: {WRITES_PER_INPUT_VALUE}");
    println!("Output buffer remainder: {OUTPUT_BUFFER_REMAINDER}");
    repetition_tester.run_tests();
}
