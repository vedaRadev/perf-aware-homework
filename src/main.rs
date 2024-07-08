type Degrees = f64;
type Meters = f64;

fn reference_haversine(x0: Degrees, y0: Degrees, x1: Degrees, y1: Degrees, radius: Degrees) -> Meters {
    let lat_dist = (y1 - y0).to_radians();
    let lon_dist = (x1 - x0).to_radians();
    let lat1 = y0.to_radians();
    let lat2 = y1.to_radians();

    let a = (lat_dist / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (lon_dist / 2.0).sin().powi(2);
    let c = a.sqrt().asin() * 2.0;

    radius * c
}

fn main() {
    let haversine = reference_haversine(-0.116773, 51.510357, -77.009003, 38.889931, 6371000.0);
    println!("{}", haversine);
}
