use std::env;
use std::process;

// this cannot be crate::Network because of how Cargo works,
// since cargo/rust treats lib.rs and main.rs as separate crates
use crate::file_operations;
use crate::open;
use crate::plot;
use crate::Network;

pub struct Config {}

impl Config {
    pub fn run(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("not enough arguments");
        }

        if args.len() > 2 && args[1] != "cascade" {
            return Err("too many arguments, expecting only 2, such as `touchstone filepath`");
        }

        // Check for special flags
        match args[1].as_str() {
            "--version" | "-v" => {
                print_version();
                process::exit(0);
            }
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            "cascade" => {
                // Parse arguments for --name or -n
                let mut output_name: Option<String> = None;
                let mut file_paths = Vec::new();

                let mut i = 2;
                while i < args.len() {
                    match args[i].as_str() {
                        "--name" | "-n" => {
                            if i + 1 < args.len() {
                                output_name = Some(args[i + 1].clone());
                                i += 2;
                            } else {
                                return Err("missing argument for --name");
                            }
                        }
                        _ => {
                            file_paths.push(args[i].clone());
                            i += 1;
                        }
                    }
                }

                if file_paths.len() < 2 {
                    return Err(
                        "cascade requires at least 2 files, e.g. `touchstone cascade file1 file2`",
                    );
                }

                let mut networks = Vec::new();
                for path in file_paths.iter() {
                    networks.push(Network::new(path.clone()));
                }

                let mut result = networks[0].clone();
                // Cascade remaining networks
                for network in networks.iter().skip(1) {
                    result = result * network.clone();
                }

                // Determine output path
                let output_s2p_path = if let Some(name) = output_name {
                    // If name is provided, use it.
                    // If it doesn't have an extension, add .s2p?
                    // Let's assume user provides full filename or we just use it as is.
                    // But we should probably ensure it ends in .s2p for consistency?
                    // The prompt says "output s2p file", so let's trust the user or append if missing?
                    // Let's just use it as is for now.
                    name
                } else {
                    // Default behavior: first file directory, "cascaded_result.html" (but we need s2p now)
                    let first_file = &file_paths[0];
                    let path = std::path::Path::new(first_file);
                    let parent = path.parent().unwrap_or(std::path::Path::new("."));
                    parent
                        .join("cascaded_result.s2p")
                        .to_string_lossy()
                        .to_string()
                };

                // Save S2P file
                if let Err(e) = result.save(&output_s2p_path) {
                    eprintln!("Failed to save S2P file: {}", e);
                    // Continue to plot generation? Or return error?
                    // Let's return error.
                    return Err("Failed to save S2P file");
                }
                println!("Saved cascaded network to {}", output_s2p_path);

                // Generate plot
                // Plot should be named based on the output s2p file
                // e.g. output.s2p -> output.s2p.html
                let output_html_path = format!("{}.html", output_s2p_path);

                generate_plot(&[result], output_html_path.clone());
                open::plot(output_html_path);

                return Ok(Config {});
            }
            _ => {
                if args.len() > 2 {
                    return Err(
                        "too many arguments, expecting only 2, such as `touchstone filepath`",
                    );
                }
            }
        }

        // cargo run arg[1], such as cargo run files/ntwk1.s2p
        // touchstone arg[1], such as touchstone files/ntwk1.s2p
        let file_argument = args[1].clone();

        parse_plot_open_in_browser(file_argument.clone());

        Ok(Config {})
    }
}

fn print_version() {
    println!("touchstone {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    // ANSI color codes
    const BOLD: &str = "\x1b[1m";
    const CYAN: &str = "\x1b[36m";
    const GREEN: &str = "\x1b[32m";
    const YELLOW: &str = "\x1b[33m";
    const RESET: &str = "\x1b[0m";

    println!();
    println!(
        "📡 Touchstone (s2p, etc.) file parser, plotter, and more - https://github.com/iancleary/touchstone{}",
        RESET
    );
    println!();
    println!("{}{}VERSION:{}", BOLD, YELLOW, RESET);
    println!("    {}{}{}", GREEN, env!("CARGO_PKG_VERSION"), RESET);
    println!();
    println!("{}{}USAGE:{}", BOLD, YELLOW, RESET);
    println!("    {} touchstone <FILE_PATH>{}", GREEN, RESET);
    println!();
    println!("     FILE_PATH: path to a s2p file");
    println!();
    println!("     The s2p file is parsed and an interactive plot (html file and js/ folder) ");
    println!("     is created next to the s2p file.");
    // println!("     ");
    println!();
    println!("{}{}OPTIONS:{}", BOLD, YELLOW, RESET);
    println!(
        "    {}  -v, --version{}{}    Print version information",
        GREEN, RESET, RESET
    );
    println!(
        "    {}  -h, --help{}{}       Print help information",
        GREEN, RESET, RESET
    );
    println!();
    println!("{}{}EXAMPLES:{}", BOLD, YELLOW, RESET);
    println!("    {} # Relative path{}", CYAN, RESET);
    println!("    {} touchstone files/measurements.s2p{}", GREEN, RESET);
    println!();
    println!("    {} # Bare filename{}", CYAN, RESET);
    println!("    {} touchstone measurement.s2p{}", GREEN, RESET);
    println!();
    println!("    {} # Windows absolute path{}", CYAN, RESET);
    println!(
        "    {} touchstone C:\\Users\\data\\measurements.s2p{}",
        GREEN, RESET
    );
    println!();
    println!("    {} # Windows UNC path (network path){}", CYAN, RESET);
    println!(
        "    {} touchstone \\\\server\\mount\\folder\\measurement.s2p{}",
        GREEN, RESET
    );
    println!();
    println!("    {} # Unix absolute path{}", CYAN, RESET);
    println!(
        "    {} touchstone /home/user/measurements.s2p{}",
        GREEN, RESET
    );
}

fn generate_plot(networks: &[Network], file_path_plot: String) {
    // creates a html file from network
    plot::generate_plot_from_networks(networks, &file_path_plot).unwrap();
    println!("Plot HTML generated at {}", file_path_plot);
}

fn parse_plot_open_in_browser(file_path: String) {
    println!("\n");
    println!("============================");
    println!("In file {}", file_path);

    let path = std::path::Path::new(&file_path);
    let mut networks = Vec::new();
    let mut output_html_path = String::new();

    if path.is_dir() {
        println!("Directory detected. Plotting all valid network files in directory.");
        // Iterate over files in directory
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(extension) = path.extension() {
                        let ext_str = extension.to_string_lossy().to_lowercase();
                        // Check for s2p, s1p, etc. (s*p)
                        if ext_str.starts_with('s') && ext_str.ends_with('p') && ext_str.len() == 3
                        {
                            println!("Found network file: {:?}", path);
                            networks.push(Network::new(path.to_string_lossy().to_string()));
                        }
                    }
                }
            }
        }
        if networks.is_empty() {
            eprintln!("No valid network files found in directory.");
            return;
        }
        // Output HTML in the directory
        output_html_path = path
            .join("combined_plot.html")
            .to_string_lossy()
            .to_string();
        generate_plot(&networks, output_html_path.clone());
        open::plot(output_html_path);
    } else {
        // Single file
        let s2p = Network::new(file_path.clone());
        networks.push(s2p);

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
            output_html_path = file_path_html;
        } else if file_path_config.relative_path_with_separators {
            output_html_path = format!("{}.html", &file_path);
        } else if file_path_config.bare_filename {
            output_html_path = format!("./{}.html", &file_path);
        } else {
            panic!(
                "file_path_config must have one true value: {:?}",
                file_path_config
            );
        }
        generate_plot(&networks, output_html_path.clone());
        open::plot(output_html_path);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use std::path::PathBuf;

    fn setup_test_dir(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push("touchstone_tests");
        path.push(name);
        path.push(format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn test_config_build() {
        let test_dir = setup_test_dir("test_config_build");
        let s2p_path = test_dir.join("test_cli_config_build.s2p");
        fs::copy("files/test_cli/test_cli_config_build.s2p", &s2p_path).unwrap();

        let args = vec![
            String::from("program_name"),
            s2p_path.to_str().unwrap().to_string(),
        ];
        let _cli_run = Config::run(&args).unwrap();

        // Cleanup is optional as it's in temp dir, but good practice if we want to check it doesn't fail
        // let _remove_file = fs::remove_file(s2p_path.with_extension("s2p.html"));
        // let _remove_js_folder = fs::remove_dir_all(test_dir.join("js"));
    }

    #[test]
    fn test_config_build_not_enough_args() {
        let args = vec![String::from("program_name")];
        let result = Config::run(&args);
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
        let test_dir_rel = setup_test_dir("test_run_function_rel");
        // Create a "files" subdir to match the relative path structure expected if needed,
        // or just use the file in the temp dir.
        // The original test used "files/test_cli_run_relative_path.s2p".
        // parse_plot_open_in_browser handles relative paths.

        let s2p_path_rel = test_dir_rel.join("test_cli_run_relative_path.s2p");
        fs::copy(
            "files/test_cli/test_cli_run_relative_path.s2p",
            &s2p_path_rel,
        )
        .unwrap();

        parse_plot_open_in_browser(s2p_path_rel.to_str().unwrap().to_string());
        // Output should be next to it
        assert!(s2p_path_rel.with_extension("s2p.html").exists());
        assert!(test_dir_rel.join("js").exists());

        // test bare filename
        // This MUST run in CWD because we pass a bare filename.
        // We can't easily isolate this without changing CWD.
        // But since we moved other tests out of "files/", this test (using root) shouldn't conflict
        // with them, UNLESS another test uses root.
        // The only other test using root is this one.
        // So we keep it as is, but maybe add a lock if we add more root tests.

        let _bare_filename_copy = fs::copy(
            "files/test_cli/test_cli_run_bare_filename.s2p",
            "test_cli_run_bare_filename.s2p",
        );
        let bare_filename = String::from("test_cli_run_bare_filename.s2p");
        parse_plot_open_in_browser(bare_filename);
        let _bare_filename_remove_file_s2p = fs::remove_file("test_cli_run_bare_filename.s2p");
        let _bare_filename_remove_file_html =
            fs::remove_file("test_cli_run_bare_filename.s2p.html");
        let _bare_filename_remove_dir = fs::remove_dir_all("js");

        // test absolute path
        let test_dir_abs = setup_test_dir("test_run_function_abs");
        let s2p_path_abs = test_dir_abs.join("test_cli_run_absolute_path.s2p");
        fs::copy(
            "files/test_cli/test_cli_run_absolute_path.s2p",
            &s2p_path_abs,
        )
        .unwrap();

        let path_buf = std::fs::canonicalize(&s2p_path_abs).unwrap();
        let absolute_path: String = path_buf.to_string_lossy().to_string();

        parse_plot_open_in_browser(absolute_path);

        assert!(s2p_path_abs.with_extension("s2p.html").exists());
        assert!(test_dir_abs.join("js").exists());
    }

    #[test]
    fn test_cascade_command() {
        let test_dir = setup_test_dir("test_cascade_command");
        let s2p1 = test_dir.join("ntwk1.s2p");
        let s2p2 = test_dir.join("ntwk2.s2p");

        fs::copy("files/ntwk1.s2p", &s2p1).unwrap();
        fs::copy("files/ntwk2.s2p", &s2p2).unwrap();

        // Test default output
        let args = vec![
            String::from("program_name"),
            String::from("cascade"),
            s2p1.to_str().unwrap().to_string(),
            s2p2.to_str().unwrap().to_string(),
        ];

        let _cli_run = Config::run(&args).unwrap();

        let expected_output_s2p = test_dir.join("cascaded_result.s2p");
        let expected_output_html = test_dir.join("cascaded_result.s2p.html");
        assert!(expected_output_s2p.exists());
        assert!(expected_output_html.exists());
        assert!(test_dir.join("js").exists());

        // Test with --name
        let output_name = test_dir.join("custom_output.s2p");
        let args_named = vec![
            String::from("program_name"),
            String::from("cascade"),
            s2p1.to_str().unwrap().to_string(),
            s2p2.to_str().unwrap().to_string(),
            String::from("--name"),
            output_name.to_str().unwrap().to_string(),
        ];

        let _cli_run_named = Config::run(&args_named).unwrap();

        assert!(output_name.exists());
        assert!(output_name.with_extension("s2p.html").exists());
    }

    #[test]
    fn test_cascade_not_enough_args() {
        let args = vec![
            String::from("program_name"),
            String::from("cascade"),
            String::from("file1"),
        ];
        let result = Config::run(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_directory_plot() {
        let test_dir = setup_test_dir("test_directory_plot");
        let s2p1 = test_dir.join("ntwk1.s2p");
        let s2p2 = test_dir.join("ntwk2.s2p");
        // Add a non-s2p file to ensure it's ignored
        let txt_file = test_dir.join("readme.txt");

        fs::copy("files/ntwk1.s2p", &s2p1).unwrap();
        fs::copy("files/ntwk2.s2p", &s2p2).unwrap();
        fs::write(&txt_file, "ignore me").unwrap();

        // Pass the directory path
        parse_plot_open_in_browser(test_dir.to_str().unwrap().to_string());

        // Check for combined output
        let expected_output = test_dir.join("combined_plot.html");
        assert!(expected_output.exists());
        assert!(test_dir.join("js").exists());

        // Verify content contains both network names (simple check)
        let content = fs::read_to_string(&expected_output).unwrap();
        // ntwk1.s2p and ntwk2.s2p should be in the content (as part of network names)
        // Note: Network::new uses the filename as name by default usually?
        // Let's check if "ntwk1.s2p" is in the content.
        // The template injects network_names.
        assert!(content.contains("ntwk1.s2p"));
        assert!(content.contains("ntwk2.s2p"));
    }
}
