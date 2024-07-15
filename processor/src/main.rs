mod json;
mod performance_metrics;

use std::{
    env,
    process,
    fs,
    io,
    mem,
};

use json::JsonParser;
use performance_metrics::{ read_cpu_timer, get_cpu_frequency_estimate };

const EARTH_RADIUS: f64 = 6372.8;

type Degrees = f64;
fn calculate_haversine_distance(x0: Degrees, y0: Degrees, x1: Degrees, y1: Degrees, radius: Degrees) -> f64 {
    let lat_dist = (y1 - y0).to_radians();
    let lon_dist = (x1 - x0).to_radians();
    let lat1 = y0.to_radians();
    let lat2 = y1.to_radians();

    let a = (lat_dist / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (lon_dist / 2.0).sin().powi(2);
    let c = a.sqrt().asin() * 2.0;

    radius * c
}

fn read_f64<T: io::Read>(buf: &mut T) -> Result<f64, io::Error> {
    let mut value = [0u8; 8];
    buf.read_exact(&mut value)?;
    Ok(unsafe { mem::transmute::<[u8; 8], f64>(value) })
}

fn main() {
    let mut startup_cycles: u64;
    let mut json_parse_cycles: u64;
    let mut input_read_cycles: u64;
    let mut input_free_cycles: u64;
    let mut sums_cycles: u64;
    let mut output_cycles: u64;
    let mut json_free_cycles: u64;

    let mut total_cycles: u64 = read_cpu_timer();
    startup_cycles = read_cpu_timer();
    // TODO redo command line arg parsing so validation can still be optional and ms_to_wait for
    // perf metrics is required
    let mut args = env::args().skip(1);
    if args.len() == 0 {
        println!("usage <[] = required, () = optional>: [json input file] [cpu freq sample time millis] (validation file)");
        process::exit(1);
    }

    let haversine_json_filename: String = args.next().unwrap();
    let cpu_frequency_sample_millis: u64 = args.next().unwrap().parse().expect("expected millis as u64");
    let haversine_validation_filename: Option<String> = args.next();
    drop(args);
 
    startup_cycles = read_cpu_timer() - startup_cycles;

    input_read_cycles = read_cpu_timer();
    let haversine_json = fs::read(&haversine_json_filename).unwrap_or_else(|err| panic!("failed to read {}: {}", haversine_json_filename, err));
    let mut haversine_validation = haversine_validation_filename.map(|filename| {
        io::BufReader::new(fs::File::open(&filename).unwrap_or_else(|err| panic!("failed to read {}: {}", err, filename)))
    });
    input_read_cycles = read_cpu_timer() - input_read_cycles;

    json_parse_cycles = read_cpu_timer();
    let object = JsonParser::new(&haversine_json).parse().unwrap_or_else(|err| panic!("{}", err));
    json_parse_cycles = read_cpu_timer() - json_parse_cycles;
    input_free_cycles = read_cpu_timer();
    drop(haversine_json);
    input_free_cycles = read_cpu_timer() - input_free_cycles;

    sums_cycles = read_cpu_timer();
    let haversine_pairs = object.get_element("pairs").expect("expected top-level \"pairs\" object");
    let mut total_haversine: f64 = 0.0;
    let mut iterations: usize = 0;
    let mut validation_num_incorrect: usize = 0;
    for pair in haversine_pairs.iter() {
        let x0 = pair.get_element_value_as::<f64>("x0").unwrap().unwrap();
        let y0 = pair.get_element_value_as::<f64>("y0").unwrap().unwrap();
        let x1 = pair.get_element_value_as::<f64>("x1").unwrap().unwrap();
        let y1 = pair.get_element_value_as::<f64>("y1").unwrap().unwrap();
        let haversine_distance = calculate_haversine_distance(x0, y0, x1, y1, EARTH_RADIUS);
        total_haversine += haversine_distance;
        iterations += 1;
        if let Some(haversine_validation) = &mut haversine_validation {
            let expected_distance = read_f64(haversine_validation).unwrap_or_else(|err| panic!("failed to read f64 from validation file: {}", err));
            // TODO if we see that some calculations don't pass validation, could be due to
            // floating point precision error (though probably unlikely with a 64-bit value).
            if haversine_distance.to_bits() != expected_distance.to_bits() {
                validation_num_incorrect += 1;
            }
        }
    }
    let average_haversine = total_haversine / (iterations as f64);
    sums_cycles = read_cpu_timer() - sums_cycles;

    output_cycles = read_cpu_timer();
    println!("average: {}", average_haversine);
    if let Some(haversine_validation) = &mut haversine_validation {
        let expected_average_haversine = read_f64(haversine_validation).unwrap_or_else(|err| panic!("failed to read f64 from validation file: {}", err));
        println!("expected: {}", expected_average_haversine);
        println!("diff: {}", expected_average_haversine - average_haversine);
        println!("invalid calculations: {}", validation_num_incorrect);
    }
    output_cycles = read_cpu_timer() - output_cycles;

    json_free_cycles = read_cpu_timer();
    drop(object);
    json_free_cycles = read_cpu_timer() - json_free_cycles;

    total_cycles = read_cpu_timer() - total_cycles;

    // PRINT PERFORMANCE INFORMATION
    fn print_profile_info(label: &str, section_cycles: u64, total_cycles: u64) {
        let percent_total = section_cycles as f64 / total_cycles as f64 * 100.0;
        println!("{}: {} cycles ({:.2}%", label, section_cycles, percent_total);
    }

    let cpu_frequency = get_cpu_frequency_estimate(cpu_frequency_sample_millis);
    println!("Total time: {:.2} ms (cpu freq estimate: {})", total_cycles as f64 / cpu_frequency as f64 * 1000.0, cpu_frequency);
    print_profile_info("Startup", startup_cycles, total_cycles);
    print_profile_info("Input Read", input_read_cycles, total_cycles);
    print_profile_info("Input Free", input_free_cycles, total_cycles);
    print_profile_info("JSON Parse", json_parse_cycles, total_cycles);
    print_profile_info("Sums", sums_cycles, total_cycles);
    print_profile_info("JSON Free", json_free_cycles, total_cycles);
    print_profile_info("Output", output_cycles, total_cycles);
}
