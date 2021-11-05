use std::env;
use xpljn::Config;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap();

    xpljn::run(config);
}
