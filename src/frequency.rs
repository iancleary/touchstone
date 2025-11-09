pub(crate) fn thz_to_hz(thz: f32) -> f32 {
    thz * 1e12
}
pub(crate) fn hz_to_thz(hz: f32) -> f32 {
    hz / 1e12
}

pub(crate) fn ghz_to_hz(ghz: f32) -> f32 {
    ghz * 1e9
}
pub(crate) fn hz_to_ghz(hz: f32) -> f32 {
    hz / 1e9
}

pub(crate) fn mhz_to_hz(mhz: f32) -> f32 {
    mhz * 1e6
}
pub(crate) fn hz_to_mhz(hz: f32) -> f32 {
    hz / 1e6
}
pub(crate) fn khz_to_hz(khz: f32) -> f32 {
    khz * 1e3
}
pub(crate) fn hz_to_khz(hz: f32) -> f32 {
    hz / 1e3
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_frequency_conversions() {
        let thz = 1.0;
        let ghz = 1000.0;
        let mhz = 1_000_000.0;
        let khz = 1_000_000_000.0;
        let hz = 1_000_000_000_000.0;
        assert_eq!(thz_to_hz(thz), hz);
        assert_eq!(hz_to_thz(hz), thz);
        assert_eq!(ghz_to_hz(ghz), hz);
        assert_eq!(hz_to_ghz(hz), ghz);
        assert_eq!(mhz_to_hz(mhz), hz);
        assert_eq!(hz_to_mhz(hz), mhz);
        assert_eq!(khz_to_hz(khz), hz);
        assert_eq!(hz_to_khz(hz), khz);
    }
}
