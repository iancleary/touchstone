use std::env;
use std::fs;
use std::process;
use touchstone::Network;

struct Config {
    file_path: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }
        let file_path = args[1].clone();

        Ok(Config { file_path })
    }
}


fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    println!("In file {}", config.file_path);

    run(config.file_path);
}

fn run(file_path: String) {
    let s2p = Network::new(file_path);
    println!("Network created.");
    println!("Frequency Unit: {}", s2p.frequency_unit);
}
