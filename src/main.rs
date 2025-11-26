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
        // cargo run arg[1], such as cargo run files/2port.sh
        let file_argument = args[1].clone();

        Ok(Config { file_argument })
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

fn generate_plot(s2p: &Network, file_path_plot: String) {
    // creates a html file from network
    plot::generate_plot_from_two_port_network(s2p, &file_path_plot).unwrap();
    println!("Plot HTML generated at {}", file_path_plot);
}

fn run(file_path: String) {
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
    fn test_run_function() {
        // test relative file
        let relative_path = String::from("files/ntwk1.s2p");
        run(relative_path);
        let _relative_remove_file = fs::remove_file("files/ntwk1.s2p.html");
        let _relative_remove_dir = fs::remove_dir_all("files/js");

        // test bare filename
        let _bare_filename_copy = fs::copy("files/ntwk1.s2p", "ntwk1.s2p");
        let bare_filename = String::from("ntwk1.s2p");
        run(bare_filename);
        let _bare_filename_remove_file_s2p = fs::remove_file("ntwk1.s2p");
        let _bare_filename_remove_file_html = fs::remove_file("ntwk1.s2p.html");
        let _bare_filename_remove_dir = fs::remove_dir_all("js");

        // This fails if "files/ntwk1.s2p" is missing on disk
        let path_buf = std::fs::canonicalize("files/ntwk1.s2p").unwrap();
        let absolute_path: String = path_buf.to_string_lossy().to_string();
        run(absolute_path);
        // don't remove s2p file in files/
        let _absolute_path_remove_file_html = fs::remove_file("files/ntwk1.s2p.html");
        let _absolute_path_remove_dir = fs::remove_dir_all("files/js");
    }
}
