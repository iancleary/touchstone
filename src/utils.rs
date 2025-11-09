pub(crate) fn str_to_f32(x: &str) -> f32 {
    x.parse::<f32>().expect("Failed to parse {x} into f32")
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_str_to_f32() {
        let x = "3.14";
        let y = super::str_to_f32(x);
        assert_eq!(y, 3.14);
    }

    #[test]
    fn test_str_to_f32_invalid() {
        let x = "abc";
        let result = std::panic::catch_unwind(|| {
            super::str_to_f32(x);
        });
        assert!(result.is_err());
    }
}
