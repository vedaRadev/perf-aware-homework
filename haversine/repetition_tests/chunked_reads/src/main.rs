use repetition_tester::{
    RepetitionTester,
    TimeTestSection,
    TimeTestResult,
};

use std::{
    fs,
    io::{ BufReader, Read },
    os::windows::fs::MetadataExt,
};

// cache sizes based on zen 3 ryzen 9 5900x

const L1_DATA_CACHE_SIZE: usize = 32_768;    // 32kb
const L2_CACHE_SIZE: usize = 2usize.pow(19); // 512kb
const L3_CACHE_SIZE: usize = 2usize.pow(26); // 64mb

struct SharedTestParams {
    file_size: u64,
    file_name: String,
}

fn bufreader_open_and_read(params: &mut SharedTestParams, chunk_size: usize) -> TimeTestResult {
    let SharedTestParams { file_name, file_size } = params;

    let section = TimeTestSection::begin();

    let mut data = vec![0u8; chunk_size];
    let file = fs::File::open(file_name.as_str()).unwrap_or_else(|_| panic!("failed to open {file_name}"));
    let mut reader = BufReader::with_capacity(chunk_size, file);
    while reader.read_exact(data.as_mut_slice()).is_ok() {}

    section.end(*file_size)
}

macro_rules! create_chunked_open_and_read_test {
    ($repetition_tester:ident, $test_fn:ident, $chunk_size:expr) => {
        {
            let chunk_size = $chunk_size;
            $repetition_tester.register_test_2(
                move |params: &mut SharedTestParams| -> TimeTestResult { $test_fn(params, chunk_size) },
                format!("open and read: {} ({chunk_size} byte chunks)", stringify!($test_fn))
            );
        }
    }
}

// TODO figure out why I'm not seeing any page faults reported!!!!
fn main() {
    let mut args = std::env::args().skip(1);
    let file_name = args.next().expect("expected a filename");
    let file_size = fs::metadata(file_name.as_str()).expect("failed to read file metadata").file_size();
    println!("{file_name}: {file_size} bytes");
    println!("L1 DATA CACHE: {L1_DATA_CACHE_SIZE}");
    println!("L2 CACHE: {L2_CACHE_SIZE}");
    println!("L3 CACHE: {L3_CACHE_SIZE}");

    let params = SharedTestParams { file_name, file_size };
    let mut repetition_tester = RepetitionTester::new(params);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 4_096);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 8_192);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 16_384);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 32_768);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 65_536);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 131_072);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 262_144);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 393_216);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 458_752);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 524_288);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 589_824);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 655_360);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 1_048_576);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 2_097_152);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 4_194_304);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 8_388_608);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 16_777_216);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 33_554_432);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 67_108_864);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 134_217_728);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 268_435_456);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 536_870_912);
    create_chunked_open_and_read_test!(repetition_tester, bufreader_open_and_read, 1_073_741_824);
    repetition_tester.run_tests();
}
