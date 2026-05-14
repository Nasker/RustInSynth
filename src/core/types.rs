/// A single audio sample value, typically in range [-1.0, 1.0]
pub type Sample = f32;

/// Sample rate in Hz (e.g., 44100, 48000)
pub type SampleRate = u32;

/// Frequency in Hz
pub type Frequency = f32;

/// Amplitude/volume level, typically in range [0.0, 1.0]
pub type Amplitude = f32;

/// MIDI note number (0-127)
pub type MidiNote = u8;

/// Converts a MIDI note number to frequency in Hz
/// A4 (note 69) = 440 Hz
pub fn midi_to_frequency(note: MidiNote) -> Frequency {
    440.0 * 2.0_f32.powf((note as f32 - 69.0) / 12.0)
}

/// Converts frequency in Hz to the nearest MIDI note number
pub fn frequency_to_midi(freq: Frequency) -> MidiNote {
    (69.0 + 12.0 * (freq / 440.0).log2()).round() as MidiNote
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a4_conversion() {
        let freq = midi_to_frequency(69);
        assert!((freq - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_frequency_to_midi() {
        assert_eq!(frequency_to_midi(440.0), 69);
    }
}
