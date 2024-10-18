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

const INPUT_BUFFER_SIZE: usize = L2_CACHE_SIZE as usize;
const OUTPUT_BUFFER_SIZE: usize = L3_CACHE_SIZE as usize * 8;
const MAX_INPUT_BUFFER_READS: usize = OUTPUT_BUFFER_SIZE / INPUT_BUFFER_SIZE;
const BYTES_TO_PROCESS: usize = MAX_INPUT_BUFFER_READS * INPUT_BUFFER_SIZE;

#[link(name = "test_routines")]
extern "C" {
    fn temporal_stores(
        input_buffer_start: *const u8,
        output_buffer_start: *mut u8,
        input_subregion_size: u64,
        read_write_loops: u64,
    );

    fn nontemporal_stores(
        input_buffer_start: *const u8,
        output_buffer_start: *mut u8,
        input_subregion_size: u64,
        read_write_loops: u64,
    );
}

#[allow(dead_code)]
struct SharedTestParams {
    input_buffer: *const u8,
    output_buffer: *mut u8,
}

fn temporal_test(
    SharedTestParams { input_buffer, output_buffer }: &mut SharedTestParams,
    subregion_size: u64,
    subregion_reads: u64,
    bytes_to_process: u64,
) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe {
        temporal_stores(
            *input_buffer,
            *output_buffer,
            subregion_size,
            subregion_reads,
        )
    }
    section.end(bytes_to_process)
}

fn nontemporal_test(
    SharedTestParams { input_buffer, output_buffer }: &mut SharedTestParams,
    subregion_size: u64,
    subregion_reads: u64,
    bytes_to_process: u64,
) -> TimeTestResult {
    let section = TimeTestSection::begin();
    unsafe {
        nontemporal_stores(
            *input_buffer,
            *output_buffer,
            subregion_size,
            subregion_reads,
        )
    }
    section.end(bytes_to_process)
}

macro_rules! create_test {
    ($repetition_tester:ident, $test_fn_name:ident, $subregion_size_expr:expr) => {
        {
            let subregion_size = $subregion_size_expr as u64;
            let subregion_reads = OUTPUT_BUFFER_SIZE as u64 / subregion_size;
            let bytes_to_process = subregion_reads * subregion_size;
            let test = move |params: &mut SharedTestParams| -> TimeTestResult {
                $test_fn_name(params, subregion_size, subregion_reads, bytes_to_process)
            };

            $repetition_tester.register_test_2(
                test,
                format!(
                    "{}: {} byte input region, writing {} bytes",
                    stringify!($test_fn_name),
                    subregion_size,
                    bytes_to_process
                )
            );
        }
    }
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
        output_buffer: output_buffer.as_mut_ptr(),
    };

    let mut repetition_tester = RepetitionTester::new(shared_test_params);
    create_test!(repetition_tester, temporal_test, INPUT_BUFFER_SIZE / 32);
    create_test!(repetition_tester, nontemporal_test, INPUT_BUFFER_SIZE / 32);
    create_test!(repetition_tester, temporal_test, INPUT_BUFFER_SIZE / 16);
    create_test!(repetition_tester, nontemporal_test, INPUT_BUFFER_SIZE / 16);
    create_test!(repetition_tester, temporal_test, INPUT_BUFFER_SIZE / 8);
    create_test!(repetition_tester, nontemporal_test, INPUT_BUFFER_SIZE / 8);
    create_test!(repetition_tester, temporal_test, INPUT_BUFFER_SIZE / 4);
    create_test!(repetition_tester, nontemporal_test, INPUT_BUFFER_SIZE / 4);
    create_test!(repetition_tester, temporal_test, INPUT_BUFFER_SIZE / 2);
    create_test!(repetition_tester, nontemporal_test, INPUT_BUFFER_SIZE / 2);
    create_test!(repetition_tester, temporal_test, INPUT_BUFFER_SIZE);
    create_test!(repetition_tester, nontemporal_test, INPUT_BUFFER_SIZE);

    println!("Processing {BYTES_TO_PROCESS} worth of reads/writes");
    println!("INPUT BUFFER SIZE: {INPUT_BUFFER_SIZE}");
    println!("OUTPUT_BUFFER_SIZE: {OUTPUT_BUFFER_SIZE}");
    println!("MAX INPUT READS: {MAX_INPUT_BUFFER_READS}");
    println!("L1 DATA CACHE SIZE: {L1_DATA_CACHE_SIZE}");
    println!("L2 CACHE SIZE: {L2_CACHE_SIZE}");
    println!("L3 CACHE SIZE: {L3_CACHE_SIZE}");
    repetition_tester.run_tests();
}
