use std::path::Path;

// The paths are relative to this .rs file
pub(crate) static PLOTLY_JS: &str = include_str!("assets/js/plotly-3.3.0.min.js");
// pub (crate) static PLOTLY_SRC_LINE: &str = "./js/plotly-3.3.0.min.js";
pub(crate) static TAILWIND_CSS: &str = include_str!("assets/js/tailwindcss-3.4.17.js");
// pub (crate) static TAILWIND_SRC_LINE: &str = "./js/tailwindcss-3.4.17.js";

pub(crate) static EXAMPLE_HTML: &str = include_str!("assets/example.html");

pub(crate) fn get_plotly_js() -> &'static str {
    PLOTLY_JS
}

pub(crate) fn get_tailwind_css() -> &'static str {
    TAILWIND_CSS
}

pub(crate) fn get_example_html() -> &'static str {
    EXAMPLE_HTML
}

pub(crate) fn write_plot_html(file_path: &str, html_content: &str) -> std::io::Result<()> {
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;

    let path = Path::new(file_path);
    let mut file = File::create(&path)?;
    file.write_all(html_content.as_bytes())?;
    Ok(())
}

pub fn generate_example_plot_html(output_path: &str) -> std::io::Result<()> {
    let folder_path = std::path::Path::new(output_path).parent().unwrap();
    std::fs::create_dir_all(folder_path)?;

    let html_content = get_example_html();
    write_plot_html(output_path, html_content)?;

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

pub fn generate_two_port_plot_html(
    output_path: &str,
    network_name: &str,
    frequency_data: &str,
    s11_data: &str,
    s21_data: &str,
    s12_data: &str,
    s22_data: &str,
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
    html_content = html_content.replace("{{network_name}}", network_name);
    html_content = html_content.replace("{{frequency_data}}", frequency_data);
    html_content = html_content.replace("{{s11_data}}", s11_data);
    html_content = html_content.replace("{{s21_data}}", s21_data);
    html_content = html_content.replace("{{s12_data}}", s12_data);
    html_content = html_content.replace("{{s22_data}}", s22_data);

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

pub fn generate_plot_from_two_port_network(
    network: &crate::Network,
    output_path: &str,
) -> std::io::Result<()> {
    let frequency_data = network
        .f
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let s11_data = network
        .s_db(1, 1)
        .iter()
        .map(|s| s.s_db.decibel().to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let s21_data = network
        .s_db(2, 1)
        .iter()
        .map(|s| s.s_db.decibel().to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let s12_data = network
        .s_db(1, 2)
        .iter()
        .map(|s| s.s_db.decibel().to_string())
        .collect::<Vec<String>>()
        .join(", ");
    let s22_data = network
        .s_db(2, 2)
        .iter()
        .map(|s| s.s_db.decibel().to_string())
        .collect::<Vec<String>>()
        .join(", ");

    // add brackets to make them valid JavaScript arrays
    let frequency_data = format!("[{}]", frequency_data);
    let s11_data = format!("[{}]", s11_data);
    let s21_data = format!("[{}]", s21_data);
    let s12_data = format!("[{}]", s12_data);
    let s22_data = format!("[{}]", s22_data);

    generate_two_port_plot_html(
        output_path,
        network.name.as_str(),
        &frequency_data,
        &s11_data,
        &s21_data,
        &s12_data,
        &s22_data,
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::Network;

    use super::*;
    #[test]
    fn test_generate_example_plot_html() {
        let output_path = "tests/output/example_plot.html";
        let result = generate_example_plot_html(output_path);
        assert!(result.is_ok());
        assert!(std::path::Path::new(output_path).exists());
        // clean up
        let _ = fs::remove_dir_all("tests/");
    }

    #[test]
    fn test_generate_two_port_plot_html() {
        let network = Network::new("files/ntwk1.s2p".to_string());

        let output_path = format!("{}.html", network.name.clone());

        let result = generate_plot_from_two_port_network(&network, &output_path);
        assert!(result.is_ok());

        assert!(std::path::Path::new(&output_path).exists());

        // clean up
        let _ = fs::remove_dir_all("tests/");
    }
}
