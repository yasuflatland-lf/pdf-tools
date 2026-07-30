#[allow(dead_code)]
#[path = "../tests/support/fixtures.rs"]
mod fixtures;

fn main() {
    fixtures::ensure_generated();
    println!(
        "Generated PDF test fixtures in {}",
        fixtures::fixtures_dir().display()
    );
}
