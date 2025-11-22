
// The path is relative to this .rs file
pub(crate) static PLOTLY_JS: &str = include_str!("assets/js/plotly-3.3.0.min.js");
pub (crate) static PLOTLY_SRC_LINE: &str = "./js/plotly-3.3.0.min.js";
pub(crate) static TAILWIND_CSS: &str = include_str!("assets/js/tailwindcss-3.4.17.js");
pub (crate) static TAILWIND_SRC_LINE: &str = "./js/tailwindcss-3.4.17.js";

pub(crate) static EXAMPLE_HTML: &str = include_str!("assets/example.html");

pub (crate) fn get_plotly_js() -> &'static str {
    PLOTLY_JS
}

pub (crate) fn get_tailwind_css() -> &'static str {
    TAILWIND_CSS
}

pub (crate) fn get_example_html() -> &'static str {
    EXAMPLE_HTML
}

pub (crate) fn write_plot_html(file_path: &str, html_content: &str) -> std::io::Result<()> {
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

    let js_assets_path = format!("{}/js", std::path::Path::new(output_path).parent().unwrap().to_str().unwrap());
    std::fs::create_dir_all(&js_assets_path)?;
    let plotly_js_path = format!("{}/plotly-3.3.0.min.js", js_assets_path);
    let tailwind_js_path = format!("{}/tailwindcss-3.4.17.js", js_assets_path);
    std::fs::write(plotly_js_path, get_plotly_js())?;
    std::fs::write(tailwind_js_path, get_tailwind_css())?;
    Ok(())
}