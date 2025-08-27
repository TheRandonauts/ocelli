use std::collections::{HashMap, HashSet};

pub struct Ocelli;

impl Ocelli {
    fn bits_to_bytes(&self, bits: &[u8]) -> Vec<u8> {
        bits.chunks(8)
            .filter_map(|chunk| {
                if chunk.len() == 8 {
                    Some(chunk.iter().fold(0, |byte, &bit| (byte << 1) | bit))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn chop_and_tack(
    // Extracts entropy from two frames by comparing pixel values in a specific grid pattern.
    // The resulting entropy is constructed by appending 1s or 0s based on pixel differences.
    // Algorithm ported from NoiseBasedCamRng by Andika Wasisto https://github.com/awasisto/camrng

        &self,
        current: &[u8],
        previous: &[u8],
        width: usize,
        minimum_distance: usize,
    ) -> Option<Vec<u8>> {
        let height = current.len() / width;
        if width <= 200 || height <= 200 || current.len() != previous.len() {
            return None; // invalid geometry
        }

        let mut entropy = Vec::new();

        // Define bounds for the grid
        let start_row = 100;
        let end_row = height - 100;

        // Process the grid within the defined bounds
        for row in start_row..end_row {
            let row_start = row * width + 100;
            let row_end = (row + 1) * width - 100;

            entropy.extend(
                (row_start..row_end)
                    .step_by(minimum_distance)
                    .filter(|&idx| idx < current.len())
                    .map(|idx| {
                        let c = current[idx];
                        let p = previous[idx];
                        (c > p) as u8 - (c < p) as u8 // 1 for c > p, 0 for c < p, skips if equal
                    })
                    .filter(|&bit| bit <= 1),
            );
        }

        Some(self.bits_to_bytes(&entropy))
    }

    pub fn pick_and_flip(&self, data: &[u8], low: u8, high: u8, current_frame_index: usize) -> Vec<u8> {
    // Extracts the least significant bit (LSB) of each pixel brightness, flipping it based on the frame index.
    // Generates entropy by combining these bits into bytes.
    // Algorithm is a simplified version of R. Li, "A True Random Number Generator algorithm from 
    // digital camera image noise for varying lighting conditions," doi: 10.1109/SECON.2015.7132901.

        let mut bits = Vec::with_capacity(data.len());
        for &pixel in data {
            if (low..=high).contains(&pixel) { // filter bias
                let mut lsb = pixel & 1;
                if current_frame_index % 2 == 0 { // flip bits
                    lsb ^= 1;
                }
                bits.push(lsb);
            }
        }
        self.bits_to_bytes(&bits)
    }

    pub fn shannon(&self, data: &[u8]) -> f64 {
    // Calculates the Shannon entropy of a given byte vector to measure its randomness.
    // Uses a frequency map to compute probabilities and their contributions to entropy.

        let mut frequency_map = HashMap::new();
        let n = data.len();
        if n == 0 { return 0.0; }

        for &byte in data {
            *frequency_map.entry(byte).or_insert(0usize) += 1;
        }

        frequency_map.values().fold(0.0, |entropy, &count| {
            let p = count as f64 / n as f64;
            entropy - p * p.log2()
        })
    }

    pub fn whiten(&self, entropy: &[u8]) -> Vec<u8> {
    // Applies von Neumann whitening to reduce bias in the input entropy.
    // Pairs of bits are analyzed, and only unbiased pairs are used to construct the output.

        let mut out = Vec::with_capacity(entropy.len() / 2);
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for &byte in entropy {
            for i in (0..8).step_by(2) {
                let bit1 = (byte >> (7 - i)) & 1;
                let bit2 = (byte >> (6 - i)) & 1;
                match (bit1, bit2) {
                    (0, 1) => { current_byte = (current_byte << 1) | 0; bit_count += 1; }
                    (1, 0) => { current_byte = (current_byte << 1) | 1; bit_count += 1; }
                    _ => {}
                }
                if bit_count == 8 {
                    out.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }
        out
    }

    pub fn is_covered(&self, grayscale: &[u8], threshold: usize) -> bool {
    // Checks if the grayscale image contains fewer unique values than the specified threshold.
    // Useful for ensuring the chop and tack method only sees noise and no image data, resulting
    // in higher quality entropy.
    // Recommended default threshold is 50

        let unique_values: HashSet<_> = grayscale.iter().copied().collect();
        // println!("UV: {:?}", unique_values);
        unique_values.len() < threshold
    }
}
