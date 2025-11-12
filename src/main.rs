use std::env;
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use touchstone::Network;

struct Config {
    file_path: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }
        // cargo run arg[1], such as cargo run files/2port.sh
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

    println!("First 5 S-parameters:\n");
    for i in 0..5 {
        println!("{:?}", s2p.data_lines[i]);
        println!("{:?}", s2p.s[i]);
    }

    println!("Last 5 S-parameters:\n");
    let n = s2p.data_lines.len();
    for i in n - 5..n {
        println!("{:?}", s2p.data_lines[i]);
        println!("{:?}", s2p.s[i]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config_build() {
        let args = vec![
            String::from("program_name"),
            String::from("files/2port.sh"),
        ];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.file_path, "files/2port.sh");
    }   

    #[test]
    fn test_config_build_not_enough_args() {
        let args = vec![String::from("program_name")];
        let result = Config::build(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_function() {
        let file_path = String::from("files/2port.s2p");
        run(file_path);
    }
}