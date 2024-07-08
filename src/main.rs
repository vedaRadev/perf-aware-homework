use std::{ cmp, env, process, fs, io::Write };
use rand::prelude::*;

type Degrees = f64;
fn reference_haversine(x0: Degrees, y0: Degrees, x1: Degrees, y1: Degrees, radius: Degrees) -> f64 {
    let lat_dist = (y1 - y0).to_radians();
    let lon_dist = (x1 - x0).to_radians();
    let lat1 = y0.to_radians();
    let lat2 = y1.to_radians();

    let a = (lat_dist / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (lon_dist / 2.0).sin().powi(2);
    let c = a.sqrt().asin() * 2.0;

    radius * c
}

struct PolarPair((Degrees, Degrees), (Degrees, Degrees));

const EARTH_RADIUS: f64 = 6372.8;
const CLUSTER_X_RADIUS: f64 = 30.0;
const CLUSTER_Y_RADIUS: f64 = 30.0;
fn main() {
    // skip executable name
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        println!("usage: [num_pairs_to_generate: int] [max_clusters: int] [seed: int]");
        process::exit(1);
    }

    let num_pairs = args[0].parse::<usize>().unwrap_or_else(|_| panic!("Failed to parse num pairs: {}", args[0]));
    let max_clusters = args[1].parse::<usize>().unwrap_or_else(|_| panic!("Failed to parse max clusters: {}", args[1]));
    let seed = args[2].parse::<i64>().unwrap_or_else(|_| panic!("Failed to parse seed: {}", args[2]));

    let mut rng = StdRng::seed_from_u64(seed as u64);

    // Because we're going to be generating a large number of pairs of points, we know that
    // if we just generated a uniform set of points, it'll always converge toward a single average
    // value given a large number of iterations. Thank probability and statistics for that.
    // So to get just a bit of variation in what our average expected haversine is, we're going to
    // actually generate a random cluster point on the sphere, then generate points in a radius
    // around the center of that cluster.
    // This is going to help us determine if our haversine algorithm is wrong when compared to the
    // reference function included in this file. Since this course is about performance, I'm
    // assuming that, even though we're going to be using the same general algorithm, it's going to
    // look different when stuff like SIMD is introduced. We may also do the loop unrolling thing
    // from the prologue of the course.
    let max_pairs_per_cluster = cmp::max((num_pairs as f64 / max_clusters as f64).ceil() as usize, 1);
    let mut pairs_generated: usize = 0;
    let mut total_haversine: f64 = 0.0;
    let mut polar_pairs: Vec<PolarPair> = Vec::with_capacity(num_pairs);
    'outer: for _cluster_index in 0 .. max_clusters {
        let cluster_x: Degrees = rng.gen_range(-180.0 .. 180.0);
        let cluster_y: Degrees = rng.gen_range(-90.0 .. 90.0);

        let cluster_x_start = cluster_x - CLUSTER_X_RADIUS;
        let cluster_x_end = cluster_x + CLUSTER_X_RADIUS;
        let cluster_y_start = cluster_y - CLUSTER_Y_RADIUS;
        let cluster_y_end = cluster_y + CLUSTER_Y_RADIUS;
        // println!("\nNEW CLUSTER ({}): ({}, {})", _cluster_index + 1, cluster_x, cluster_y);

        loop {
            let x0 = rng.gen_range(cluster_x_start .. cluster_x_end);
            let y0 = rng.gen_range(cluster_y_start .. cluster_y_end);
            let x1 = rng.gen_range(cluster_x_start .. cluster_x_end);
            let y1 = rng.gen_range(cluster_y_start .. cluster_y_end);
            polar_pairs.push(PolarPair((x0, y0), (x1, y1)));

            let haversine_distance = reference_haversine(x0, y0, x1, y1, EARTH_RADIUS);
            total_haversine += haversine_distance;

            pairs_generated += 1;
            println!("{}: ({}, {}) ({}, {}) -> {}", pairs_generated, x0, y0, x1, y1, haversine_distance);

            if pairs_generated == num_pairs { break 'outer; }
            if pairs_generated % max_pairs_per_cluster == 0 { break; }
        }
    }

    let mut file = fs::File::create("haversine_pairs.json").expect("Failed to open JSON output file point_pairs.json");
    let _ = file.write(b"{\n\t\"pairs\": [\n").expect("failed to write to JSON output file point_pairs.json");
    for (idx, PolarPair((x0, y0), (x1, y1))) in polar_pairs.iter().enumerate() {
        // { "x0":<x0>, "y0":<y0>, "x1":<x1>, "y1":<y1> }
        let string = format!(
            "\t\t{{\"x0\":{}, \"y0\":{}, \"x1\":{}, \"y1\":{}}}{}\n",
            x0,
            y0,
            x1,
            y1,
            if idx == polar_pairs.len() - 1 { "" } else { "," }
        );
        let _ = file.write(string.as_bytes()).expect("failed to write to JSON output file point_pairs.json");
    }
    let _ = file.write(b"\t]\n}");

    let average_haversine: f64 = total_haversine / pairs_generated as f64;
    println!("expected haversine average: {}", average_haversine);
}
