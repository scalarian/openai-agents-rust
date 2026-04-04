pub fn calculate_audio_length_ms(sample_count: usize, sample_rate_hz: u32) -> u64 {
    if sample_rate_hz == 0 {
        return 0;
    }
    ((sample_count as f64 / sample_rate_hz as f64) * 1000.0).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_audio_length_ms() {
        assert_eq!(calculate_audio_length_ms(1600, 16000), 100);
    }
}
