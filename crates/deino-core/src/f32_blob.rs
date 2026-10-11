//! Embeddings as SQLite BLOBs: little-endian `f32`s, four bytes each.
//!
//! The one encoding of `speakers.centroid` (CAP-03) and
//! `voiceprints.embedding` (CAP-04), so a centroid enrolled as a voiceprint
//! round-trips bit for bit.

/// `values` as little-endian bytes.
pub(crate) fn encode(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// Little-endian bytes back into values (a trailing partial value is
/// dropped; both schemas allow none).
pub(crate) fn decode(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .filter_map(|chunk| chunk.try_into().ok().map(f32::from_le_bytes))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_round_trip_bit_for_bit() {
        let values = [0.25_f32, -1.5, f32::MIN_POSITIVE, 1e-7];
        let bytes = encode(&values);
        assert_eq!(bytes.len(), 16);
        assert_eq!(
            decode(&bytes)
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            values.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        );
        assert_eq!(decode(&[0, 0, 128]), Vec::<f32>::new());
    }
}
