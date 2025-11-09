pub(crate) fn str_to_f64(x: &str) -> f64 {
    x.parse::<f64>().expect("Failed to parse {x} into f64")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_str_to_f64() {
        let x = "3.14";
        let y = super::str_to_f64(x);
        assert_eq!(y, 3.14);
    }

    #[test]
    fn test_str_to_f64_invalid() {
        let x = "abc";
        let result = std::panic::catch_unwind(|| {
            super::str_to_f64(x);
        });
        assert!(result.is_err());
    }
}
