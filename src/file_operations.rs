use std::path::Path;

#[derive(Debug)]
pub struct FilePathConfig {
    pub absolute_path: bool,
    pub relative_path_with_separators: bool,
    pub bare_filename: bool,
}

// prevents downstream problems with path.parent() when passing
// in a bare filename, such as measured.s2p
// this function help us adjust to ./measured.s2p so logic is easier later
pub fn get_file_path_config(path_str: &str) -> FilePathConfig {
    let path = Path::new(path_str);

    if path.is_absolute() {
        // /home/user/files/measured.s2p, etc.
        println!("'{}' is an Absolute path.", path_str);
        FilePathConfig {
            absolute_path: true,
            relative_path_with_separators: false,
            bare_filename: false,
        }
    }
    // If it's not absolute, we check the number of parts
    else if path.components().count() > 1 {
        // files/measured.s2p, etc.
        println!(
            "'{}' is a Relative path with separators (nested).",
            path_str
        );
        FilePathConfig {
            absolute_path: false,
            relative_path_with_separators: true,
            bare_filename: false,
        }
    } else {
        // measured.s2p, etc.
        println!("'{}' is a Bare filename (no separators).", path_str);
        FilePathConfig {
            absolute_path: false,
            relative_path_with_separators: false,
            bare_filename: true,
        }
    }
}

fn path_to_url_manual(path_str: &str) -> String {
    // 1. Unify separators to '/'
    let cleaned_path = path_str.replace('\\', "/");

    // 2. Handle Windows vs Unix prefix
    if cleaned_path.starts_with('/') {
        // Unix: just add file://
        format!("file://{}", cleaned_path)
    } else {
        // Windows: C:/Path -> file:///C:/Path (needs 3 slashes)
        format!("file:///{}", cleaned_path)
    }
}

pub fn get_file_url(file_path: &String) -> String {
    let mut path_str: String = std::fs::canonicalize(file_path)
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Remove the UNC prefix on Windows if present
    if cfg!(target_os = "windows") && path_str.starts_with(r"\\?\") {
        path_str = path_str[4..].to_string();
    }

    // add file_prefix
    path_to_url_manual(&path_str)
}
