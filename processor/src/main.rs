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
use performance_metrics::{ profile, print_profile_info };

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

#[inline(always)]
fn read_f64<T: io::Read>(buf: &mut T) -> Result<f64, io::Error> {
    let mut value = [0u8; 8];
    buf.read_exact(&mut value)?;
    Ok(unsafe { mem::transmute::<[u8; 8], f64>(value) })
}

fn main() {
    profile! { "startup";
        // TODO redo command line arg parsing so validation can still be optional and ms_to_wait for
        // perf metrics is required
        let mut args = env::args().skip(1);
        if args.len() == 0 {
            println!("usage <[] = required, () = optional>: [json input file] [cpu freq sample time millis] (validation file)");
            process::exit(1);
        }

        profile! { "bleep";
            let haversine_json_filename: String = args.next().unwrap();
            let cpu_frequency_sample_millis: u64 = args.next().unwrap().parse().expect("expected millis as u64");
            let haversine_validation_filename: Option<String> = args.next();
            drop(args);
        }
    }

    profile! { "input read";
        let haversine_json = fs::read(&haversine_json_filename).unwrap_or_else(|err| panic!("failed to read {}: {}", haversine_json_filename, err));
        let mut haversine_validation = haversine_validation_filename.map(|filename| {
            io::BufReader::new(fs::File::open(&filename).unwrap_or_else(|err| panic!("failed to read {}: {}", err, filename)))
        });
    }

    profile! { "json parse";
        let object = JsonParser::new(&haversine_json).parse().unwrap_or_else(|err| panic!("{}", err));
    }

    drop(haversine_json);

    profile! { "sums";
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
                if (haversine_distance - expected_distance).abs() > 0.000001 {
                    validation_num_incorrect += 1;
                }
            }
        }
        let average_haversine = total_haversine / (iterations as f64);
    }

    profile! { "output";
        println!("average: {}", average_haversine);
        if let Some(haversine_validation) = &mut haversine_validation {
            let expected_average_haversine = read_f64(haversine_validation).unwrap_or_else(|err| panic!("failed to read f64 from validation file: {}", err));
            println!("expected: {}", expected_average_haversine);
            println!("diff: {}", expected_average_haversine - average_haversine);
            println!("invalid calculations: {}", validation_num_incorrect);
        }
    }

    profile! { "json free";
        drop(object);
    }

    print_profile_info!(cpu_frequency_sample_millis);
}
