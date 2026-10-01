use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    println!(
        "polymarket-execution-verifier library — run `cargo run --example basic_verification` for the demo"
    );
}
