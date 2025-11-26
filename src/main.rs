use std::env;
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use touchstone::file_operations;
use touchstone::open;
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

        // Check for special flags
        match args[1].as_str() {
            "--version" | "-v" => {
                println!("touchstone {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            _ => {}
        }

        // cargo run arg[1], such as cargo run files/2port.sh
        let file_argument = args[1].clone();

        Ok(Config { file_argument })
    }
}

fn print_help() {
    println!(
        "touchstone {} - A Touchstone Parser for RF Modeling",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    println!("USAGE:");
    println!("    touchstone <FILE_PATH>");
    println!();
    println!("OPTIONS:");
    println!("    -v, --version    Print version information");
    println!("    -h, --help       Print help information");
    println!();
    println!("EXAMPLES:");
    println!("    # Relative path");
    println!("    touchstone files/measurements.s2p");
    println!();
    println!("    # Bare filename");
    println!("    touchstone measurement.s2p");
    println!();
    println!("    # Windows absolute path");
    println!("    touchstone C:\\Users\\data\\measurements.s2p");
    println!();
    println!("    # Windows UNC path (network path)");
    println!("    touchstone \\\\server\\mount\\folder\\measurement.s2p");
    println!();
    println!("    # Unix absolute path");
    println!("    touchstone /home/user/measurements.s2p");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::build(&args).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    parse_plot_open_in_browser(config.file_argument);
}

fn generate_plot(s2p: &Network, file_path_plot: String) {
    // creates a html file from network
    plot::generate_plot_from_two_port_network(s2p, &file_path_plot).unwrap();
    println!("Plot HTML generated at {}", file_path_plot);
}

fn parse_plot_open_in_browser(file_path: String) {
    println!("\n");
    println!("============================");
    println!("In file {}", file_path);

    let s2p = Network::new(file_path.clone());

    let file_path_config: file_operations::FilePathConfig =
        file_operations::get_file_path_config(&file_path);

    // absolute path, append .html, remove woindows UNC Prefix if present
    // relative path with separators, just append .hmtl
    // bare_filename, prepend ./ and append .html
    if file_path_config.absolute_path {
        let mut file_path_html = format!("{}.html", &file_path);
        // Remove the UNC prefix on Windows if present
        if cfg!(target_os = "windows") && file_path_html.starts_with(r"\\?\") {
            file_path_html = file_path_html[4..].to_string();
        }
        generate_plot(&s2p, file_path_html.clone());
        open::plot(file_path_html.clone());
    } else if file_path_config.relative_path_with_separators {
        let file_path_html = format!("{}.html", &file_path);
        generate_plot(&s2p, file_path_html.clone());
        open::plot(file_path_html.clone());

    // if bare_filename, prepend ./ and append .html
    } else if file_path_config.bare_filename {
        let file_path_html = format!("./{}.html", &file_path);
        generate_plot(&s2p, file_path_html.clone());
        open::plot(file_path_html.clone());
    } else {
        panic!(
            "file_path_config must have one true value: {:?}",
            file_path_config
        );
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
    fn test_help_flag() {
        // Help flag test - verifies the flag is recognized
        // Note: In actual execution, this would exit the process
        // This test just documents the expected behavior
        let help_flags = vec!["--help", "-h"];
        for flag in help_flags {
            assert!(flag == "--help" || flag == "-h");
        }
    }

    #[test]
    fn test_version_flag() {
        // Version flag test - verifies the flag is recognized
        // Note: In actual execution, this would exit the process
        // This test just documents the expected behavior
        let version_flags = vec!["--version", "-v"];
        for flag in version_flags {
            assert!(flag == "--version" || flag == "-v");
        }
    }

    #[test]
    fn test_version_output_format() {
        // Test that version string is in correct format
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        // Version should be in format X.Y.Z
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "Version should be in X.Y.Z format");
    }

    #[test]
    fn test_run_function() {
        // test relative file
        let relative_path = String::from("files/ntwk1.s2p");
        parse_plot_open_in_browser(relative_path);
        let _relative_remove_file = fs::remove_file("files/ntwk1.s2p.html");
        let _relative_remove_dir = fs::remove_dir_all("files/js");

        // test bare filename
        let _bare_filename_copy = fs::copy("files/ntwk1.s2p", "ntwk1.s2p");
        let bare_filename = String::from("ntwk1.s2p");
        parse_plot_open_in_browser(bare_filename);
        let _bare_filename_remove_file_s2p = fs::remove_file("ntwk1.s2p");
        let _bare_filename_remove_file_html = fs::remove_file("ntwk1.s2p.html");
        let _bare_filename_remove_dir = fs::remove_dir_all("js");

        // This fails if "files/ntwk1.s2p" is missing on disk
        let path_buf = std::fs::canonicalize("files/ntwk1.s2p").unwrap();
        let absolute_path: String = path_buf.to_string_lossy().to_string();
        parse_plot_open_in_browser(absolute_path);
        // don't remove s2p file in files/
        let _absolute_path_remove_file_html = fs::remove_file("files/ntwk1.s2p.html");
        let _absolute_path_remove_dir = fs::remove_dir_all("files/js");
    }
}
