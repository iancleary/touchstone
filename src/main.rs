use std::env;
use std::process;

use touchstone::cli;

fn main() {
    let args: Vec<String> = env::args().collect();

    let _ = cli::Config::run(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });
}
