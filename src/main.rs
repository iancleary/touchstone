use std::env;
use std::path::Path;
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use touchstone::plot;
use touchstone::Network;

struct Config {
    file_argument: String,
}

impl Config {
    fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }
        // cargo run arg[1], such as cargo run files/2port.sh
        let file_argument = args[1].clone();

        Ok(Config { file_argument })
    }
}

#[derive(Debug)]
struct FilePathConfig {
    absolute_path: bool,
    relative_path_with_separators: bool,
    bare_filename: bool,
}

// prevents downstream problems with path.parent() when passing
// in a bare filename, such as measured.s2p
// this function help us adjust to ./measured.s2p so logic is easier later
fn get_file_path_config(path_str: &str) -> FilePathConfig {
    let path = Path::new(path_str);

    if path.is_absolute() {
        // /home/user/files/measured.s2p, etc.
        println!("'{}' is an Absolute path.", path_str);
        return FilePathConfig {
            absolute_path: true,
            relative_path_with_separators: false,
            bare_filename: false,
        };
    }
    // If it's not absolute, we check the number of parts
    else if path.components().count() > 1 {
        // files/measured.s2p, etc.
        println!(
            "'{}' is a Relative path with separators (nested).",
            path_str
        );
        return FilePathConfig {
            absolute_path: false,
            relative_path_with_separators: true,
            bare_filename: false,
        };
    } else {
        // measured.s2p, etc.
        println!("'{}' is a Bare filename (no separators).", path_str);
        return FilePathConfig {
            absolute_path: false,
            relative_path_with_separators: false,
            bare_filename: true,
        };
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    run(config.file_argument);
}

fn run(file_path: String) {
    println!("\n");
    println!("============================");
    println!("In file {}", file_path);

    let s2p = Network::new(file_path.clone());

    println!("Network created.");

    let length_of_data = s2p.f.len();

    let mut head_count = 5;
    let mut tail_count = 5;
    if length_of_data < 5 {
        println!("Warning: less than 5 data lines in file.");
        head_count = length_of_data;
        tail_count = 0;
    }

    println!("============================");
    s2p.print_summary();
    println!("============================");

    println!("\nFirst {:?} S-parameters:\n", head_count);
    for i in 0..head_count {
        println!("{:?}", s2p.f[i]);
        println!("{:?}", s2p.s[i]);
    }

    if tail_count != 0 {
        println!("\nLast 5 S-parameters:\n");
        for i in length_of_data - 5..length_of_data {
            println!("{:?}", s2p.f[i]);
            println!("{:?}", s2p.s[i]);
        }
    }
    println!("============================");

    let file_path_config: FilePathConfig = get_file_path_config(&file_path);
    let mut file_path_plot = String::new();

    // ensures file_path_plot is not a bare_filename
    if file_path_config.absolute_path {
        file_path_plot = format!("{}.html", &file_path);
    } else if file_path_config.relative_path_with_separators {
        file_path_plot = format!("{}.html", &file_path);
    } else if file_path_config.bare_filename {
        file_path_plot = format!("./{}.html", &file_path);
    } else {
        panic!(
            "file_path_config must have one true value: {:?}",
            file_path_config
        );
    }

    // ensuring file_path_plot is not a bare_file name allows
    // the creation of the js folder to be simpler, as it doesn't have to handle .parent() path existence concerns
    plot::generate_plot_from_two_port_network(&s2p, &file_path_plot).unwrap();
    println!("Plot HTML generated at {}", file_path_plot);

    // Note: This does NOT handle space encoding (spaces remain spaces),
    // which most modern browsers can handle, but strictly speaking is invalid URI syntax.
    let file_path_file_url = format!(
        "file://{}",
        std::fs::canonicalize(&file_path_plot)
            .unwrap()
            .to_str()
            .unwrap()
    );

    println!(
        "You can open the plot in your browser at:\n{}",
        file_path_file_url
    );

    // if not part of cargo test, open the created file
    if cfg!(test) {
        // pass
    } else {
        // 1. Ensure we have the absolute path to the file
        // canonicalize() returns a PathBuf and handles relative paths
        println!("Attempting to open plot in your default browser...");
        // 2. Use the open crate to launch the file, if not testing
        // open::that() works with paths or URL strings
        match open::that(&file_path_file_url) {
            Ok(()) => {
                println!("Success! Check your browser.");
            }
            Err(e) => {
                // 3. Graceful Fallback
                // If it fails (e.g. headless environment), print the path for manual opening
                eprintln!("Could not open the file automatically: {}", e);
                println!("You can manually open this file:\n{}", file_path_file_url);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    #[test]
    fn test_config_build() {
        let args = vec![String::from("program_name"), String::from("files/2port.sh")];
        let config = Config::build(&args).unwrap();
        assert_eq!(config.file_argument, "files/2port.sh");
    }

    #[test]
    fn test_config_build_not_enough_args() {
        let args = vec![String::from("program_name")];
        let result = Config::build(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_function() {
        // test relative file
        let relative_path = String::from("files/ntwk1.s2p");
        run(relative_path);
        let _ = fs::remove_file("files/ntwk1.s2p.html");
        let _2 = fs::remove_dir_all("files/js");

        // test bare filename
        let _3 = fs::copy("files/ntwk1.s2p", "ntwk1.s2p");
        let bare_filename = String::from("ntwk1.s2p");
        run(bare_filename);
        let _4 = fs::remove_file("ntwk1.s2p");
        let _5 = fs::remove_file("ntwk1.s2p.html");
        let _6 = fs::remove_dir_all("js");

        // This fails if "files/ntwk1.s2p" is missing on disk
        let path_buf = std::fs::canonicalize("files/ntwk1.s2p").unwrap();
        let absolute_path: String = path_buf.to_string_lossy().to_string();
        run(absolute_path);
        let _7 = fs::remove_file("files/ntwk1.s2p.html");
        let _8 = fs::remove_dir_all("files/js");
    }
}
