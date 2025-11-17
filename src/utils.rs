pub(crate) fn str_to_f64(x: &str) -> f64 {
    x.parse::<f64>().expect("Failed to parse {x} into f64")
}

// Rule 5. All angles are measured in degrees.
pub(crate) fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
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

    #[test]
    fn degrees_to_radians_90() {
        let degrees: f64 = 90.0;
        let radians = super::degrees_to_radians(degrees);
        assert_eq!(radians, std::f64::consts::PI / 2.0);
    }

    #[test]
    fn degrees_to_radians_180() {
        let degrees: f64 = 180.0;
        let radians = super::degrees_to_radians(degrees);
        assert_eq!(radians, std::f64::consts::PI);
    }

    #[test]
    fn degrees_to_radians_n90() {
        let degrees: f64 = -90.0;
        let radians = super::degrees_to_radians(degrees);
        assert_eq!(radians, -std::f64::consts::PI / 2.0);
    }
}
