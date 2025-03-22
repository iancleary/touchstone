use std::fs;

pub fn read_file(file_path: String) -> String {
  let contents = fs::read_to_string(file_path)
      .expect("Should have been able to read the file");
  contents
}

#[cfg(test)]
mod tests {

    use super::read_file;
    #[test]
    fn parse_2port() {
        let contents = read_file("files/2port.s2p".to_string());
    }
}