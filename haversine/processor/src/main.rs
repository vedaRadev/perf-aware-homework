mod json;

use std::{
    env,
    process,
    fs,
    mem,
};

#[cfg(feature = "profiling")]
use std::os::windows::fs::MetadataExt;

use json::JsonParser;
use performance_metrics::{ init_profiler, profile, end_and_print_profile_info };

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

type HaversinePair = ((f64, f64), (f64, f64));

fn main() {
    init_profiler!();

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

    profile! { "file read" [ fs::metadata(&haversine_json_filename).expect("no metadata for file").file_size() ];
        let haversine_json = fs::read(&haversine_json_filename)
            .unwrap_or_else(|err| panic!("failed to read {}: {}", haversine_json_filename, err));
    }

    let mut haversine_validation = haversine_validation_filename.map(|filename| {
        fs::read(&filename)
            .unwrap_or_else(|err| panic!("failed to read {}: {}", err, filename))
    });


    profile! { "parse haversine pairs";
        let object = JsonParser::new(&haversine_json).parse().unwrap_or_else(|err| panic!("{}", err));

        profile! { "convert values";
            let haversine_pairs: Vec<HaversinePair> = object
                .get_element("pairs").expect("expected top-level \"pairs\" object")
                .iter()
                .map(|pair| {
                    let x0 = pair.get_element_value_as::<f64>("x0").unwrap().unwrap();
                    let y0 = pair.get_element_value_as::<f64>("y0").unwrap().unwrap();
                    let x1 = pair.get_element_value_as::<f64>("x1").unwrap().unwrap();
                    let y1 = pair.get_element_value_as::<f64>("y1").unwrap().unwrap();
                    ((x0, y0), (x1, y1))
                })
                .collect();
        }

        profile! { "free json";
            drop(object);
        }
    }

    let mut total_haversine: f64 = 0.0;
    profile! { "sums" [ (mem::size_of::<HaversinePair>() * haversine_pairs.len()) as u64 ];
        for((x0, y0), (x1, y1)) in haversine_pairs.iter() {
            let haversine_distance = calculate_haversine_distance(*x0, *y0, *x1, *y1, EARTH_RADIUS);
            total_haversine += haversine_distance;
        }
    }

    let average_haversine = total_haversine / (haversine_pairs.len() as f64);

    println!("input size: {}", haversine_json.len());
    println!("pair count: {}", haversine_pairs.len());
    println!("average haversine: {}", average_haversine);
    if let Some(haversine_validation) = &mut haversine_validation {
        let expected_average_haversine = unsafe {
            let slice: [u8; 8] = haversine_validation[haversine_validation.len() - 8 ..]
                .try_into().expect("failed to grab expected haversine average");
            mem::transmute::<[u8; 8], f64>(slice)
        };

        println!("\texpected: {}", expected_average_haversine);
        println!("\tdiff: {}", expected_average_haversine - average_haversine);
    }

    end_and_print_profile_info!(cpu_frequency_sample_millis);
}
