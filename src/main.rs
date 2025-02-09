use cashkit;

fn main() {
    println!("Hello, world!");
    println!("{}", cashkit::exchange("EUR", "CAD", f64::from(1)));
}
