use std::{fs, path::Path};

// The paths are relative to this .rs file
pub(crate) static PLOTLY_JS: &str = include_str!("assets/js/plotly-3.3.0.min.js");
// pub (crate) static PLOTLY_SRC_LINE: &str = "./js/plotly-3.3.0.min.js";
pub(crate) static TAILWIND_CSS: &str = include_str!("assets/js/tailwindcss-3.4.17.js");
// pub (crate) static TAILWIND_SRC_LINE: &str = "./js/tailwindcss-3.4.17.js";

pub(crate) fn get_plotly_js() -> &'static str {
    PLOTLY_JS
}

pub(crate) fn get_tailwind_css() -> &'static str {
    TAILWIND_CSS
}

pub(crate) fn write_plot_html(file_path: &str, html_content: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;

    let path = Path::new(file_path);

    // delete existing file
    if path.exists() {
        let _ = fs::remove_file(path);
    }

    // open file in write mode
    let mut file = File::create(path)?;
    file.write_all(html_content.as_bytes())?;
    Ok(())
}

pub fn generate_two_port_plot_html(
    output_path: &str,
    network_names: &[String],
    frequency_data: &[String],
    s11_data: &[String],
    s21_data: &[String],
    s12_data: &[String],
    s22_data: &[String],
) -> std::io::Result<()> {
    // this only works if a relative path or full path is given.
    // the unwrap fails if "ntwk1.s2p" is given instead of "./ntwk1.s2p"
    // this is handled befroby main.rs::get_file_path_config
    // Attempt to get parent; if None, default to "." (current dir)
    let folder_path = Path::new(output_path)
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                Path::new(".")
            } else {
                p
            }
        })
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder_path)?;

    let mut html_content = include_str!("assets/template_2port.html").to_string();

    // Format arrays for JS injection
    let format_js_string_array = |arr: &[String]| -> String {
        let items: Vec<String> = arr.iter().map(|s| format!("'{}'", s)).collect();
        format!("[{}]", items.join(", "))
    };

    let format_js_data_array = |arr: &[String]| -> String { format!("[{}]", arr.join(", ")) };

    html_content = html_content.replace(
        "{{ network_names }}",
        &format_js_string_array(network_names),
    );
    html_content = html_content.replace(
        "{{ frequency_data }}",
        &format_js_data_array(frequency_data),
    );
    html_content = html_content.replace("{{ s11_data }}", &format_js_data_array(s11_data));
    html_content = html_content.replace("{{ s21_data }}", &format_js_data_array(s21_data));
    html_content = html_content.replace("{{ s12_data }}", &format_js_data_array(s12_data));
    html_content = html_content.replace("{{ s22_data }}", &format_js_data_array(s22_data));

    write_plot_html(output_path, &html_content)?;

    let js_assets_path = format!(
        "{}/js",
        std::path::Path::new(output_path)
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
    );
    std::fs::create_dir_all(&js_assets_path)?;
    let plotly_js_path = format!("{}/plotly-3.3.0.min.js", js_assets_path);
    let tailwind_js_path = format!("{}/tailwindcss-3.4.17.js", js_assets_path);
    std::fs::write(plotly_js_path, get_plotly_js())?;
    std::fs::write(tailwind_js_path, get_tailwind_css())?;
    Ok(())
}

pub fn generate_one_port_plot_html(
    output_path: &str,
    network_names: &[String],
    frequency_data: &[String],
    s11_data: &[String],
) -> std::io::Result<()> {
    let folder_path = Path::new(output_path)
        .parent()
        .map(|p| {
            if p.as_os_str().is_empty() {
                Path::new(".")
            } else {
                p
            }
        })
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder_path)?;

    let mut html_content = include_str!("assets/template_1port.html").to_string();

    // Format arrays for JS injection
    let format_js_string_array = |arr: &[String]| -> String {
        let items: Vec<String> = arr.iter().map(|s| format!("'{}'", s)).collect();
        format!("[{}]", items.join(", "))
    };

    let format_js_data_array = |arr: &[String]| -> String { format!("[{}]", arr.join(", ")) };

    html_content = html_content.replace(
        "{{ network_names }}",
        &format_js_string_array(network_names),
    );
    html_content = html_content.replace(
        "{{ frequency_data }}",
        &format_js_data_array(frequency_data),
    );
    html_content = html_content.replace("{{ s11_data }}", &format_js_data_array(s11_data));

    write_plot_html(output_path, &html_content)?;

    let js_assets_path = format!(
        "{}/js",
        std::path::Path::new(output_path)
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
    );
    std::fs::create_dir_all(&js_assets_path)?;
    let plotly_js_path = format!("{}/plotly-3.3.0.min.js", js_assets_path);
    let tailwind_js_path = format!("{}/tailwindcss-3.4.17.js", js_assets_path);
    std::fs::write(plotly_js_path, get_plotly_js())?;
    std::fs::write(tailwind_js_path, get_tailwind_css())?;
    Ok(())
}

pub fn generate_plot_from_networks(
    networks: &[crate::Network],
    output_path: &str,
) -> std::io::Result<()> {
    // Check if all networks have the same rank
    if networks.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "No networks provided for plotting",
        ));
    }

    let rank = networks[0].rank;

    // Verify all networks have the same rank
    for network in networks {
        if network.rank != rank {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "All networks must have the same rank. Found {} and {}",
                    rank, network.rank
                ),
            ));
        }
    }

    // Handle different ranks
    match rank {
        1 => {
            // 1-port network plotting
            let mut network_names = Vec::new();
            let mut frequency_data_list = Vec::new();
            let mut s11_data_list = Vec::new();

            for network in networks {
                network_names.push(network.name.clone());

                let freq = network
                    .f
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                frequency_data_list.push(format!("[{}]", freq));

                let s11 = network
                    .s_db(1, 1)
                    .iter()
                    .map(|s| s.s_db.decibel().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                s11_data_list.push(format!("[{}]", s11));
            }

            generate_one_port_plot_html(
                output_path,
                &network_names,
                &frequency_data_list,
                &s11_data_list,
            )
        }
        2 => {
            // 2-port network plotting (existing code)
            let mut network_names = Vec::new();
            let mut frequency_data_list = Vec::new();
            let mut s11_data_list = Vec::new();
            let mut s21_data_list = Vec::new();
            let mut s12_data_list = Vec::new();
            let mut s22_data_list = Vec::new();

            for network in networks {
                network_names.push(network.name.clone());

                let freq = network
                    .f
                    .iter()
                    .map(|f| f.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                frequency_data_list.push(format!("[{}]", freq));

                let s11 = network
                    .s_db(1, 1)
                    .iter()
                    .map(|s| s.s_db.decibel().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                s11_data_list.push(format!("[{}]", s11));

                let s21 = network
                    .s_db(2, 1)
                    .iter()
                    .map(|s| s.s_db.decibel().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                s21_data_list.push(format!("[{}]", s21));

                let s12 = network
                    .s_db(1, 2)
                    .iter()
                    .map(|s| s.s_db.decibel().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                s12_data_list.push(format!("[{}]", s12));

                let s22 = network
                    .s_db(2, 2)
                    .iter()
                    .map(|s| s.s_db.decibel().to_string())
                    .collect::<Vec<String>>()
                    .join(", ");
                s22_data_list.push(format!("[{}]", s22));
            }

            generate_two_port_plot_html(
                output_path,
                &network_names,
                &frequency_data_list,
                &s11_data_list,
                &s21_data_list,
                &s12_data_list,
                &s22_data_list,
            )
        }
        _ => {
            // N-port where N > 2: Not yet supported for plotting
            Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!(
                    "Plotting for {}-port networks is not yet supported. \
                     Currently only 1-port and 2-port networks can be plotted. \
                     For {}-port networks, you can still parse and access S-parameters programmatically, \
                     but interactive HTML plots are not available yet.",
                    rank, rank
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::Network;

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
    fn test_generate_two_port_plot_html() {
        let test_dir = setup_test_dir("test_generate_two_port_plot_html");
        let s2p_path = test_dir.join("test_plot.s2p");
        fs::copy("files/test_plot/test_plot.s2p", &s2p_path).unwrap();

        let network = Network::new(s2p_path.to_str().unwrap().to_string());

        // network.name is derived from filename, so it will be "test_plot.s2p" (or similar depending on implementation)
        // Network::new uses parser::read_file which sets name.
        // If name is full path, this might be tricky.
        // Let's check Network::new implementation or parser.
        // Assuming name is just filename or derived from it.
        // But wait, output_path is constructed here.
        // If network.name is "test_plot.s2p", output_path is "test_plot.s2p.html".
        // But we want it in the test_dir.

        // Network::new takes a path.
        // parser::read_file probably sets name to filename.
        // Let's assume network.name is just the name.

        // We need to ensure output_path is in test_dir.
        // generate_plot_from_two_port_network takes output_path.
        // If output_path is relative, it puts it relative to CWD?
        // No, generate_two_port_plot_html uses output_path parent.

        // So we should construct output_path to be in test_dir.
        let output_path = s2p_path.with_extension("s2p.html");
        let output_path_str = output_path.to_str().unwrap().to_string();

        println!("{}", output_path_str);

        let output_path_as_path = Path::new(&output_path_str);

        // delete existing file
        if output_path_as_path.exists() {
            let _ = fs::remove_file(output_path_as_path);
        }

        let _ = generate_plot_from_networks(&[network], &output_path_str);

        assert!(std::path::Path::new(&output_path_str).exists());
        assert!(test_dir.join("js").exists());

        // clean up
        // let _remove_test_plot_file = fs::remove_file(output_path_as_path);
        // let _remove_tests_js_folder = fs::remove_dir_all(test_dir.join("js"));
    }
}
