use std::collections::{HashMap, HashSet};
use std::slice;

pub struct Ocelli;

impl Ocelli {

    fn bits_to_bytes(&self, bits: &[u8]) -> Vec<u8> {
        bits.chunks(8).filter_map(|chunk| {
            if chunk.len() == 8 {
                Some(chunk.iter().fold(0, |byte, &bit| (byte << 1) | bit))
            } else {
                None
            }
        }).collect()
    }

    pub fn chop_and_tack(&self, current: &Vec<u8>, previous: &Vec<u8>, width: usize, minimum_distance: usize) -> Vec<u8> {
        // Extracts entropy from two frames by comparing pixel values in a specific grid pattern.
        // The resulting entropy is constructed by appending 1s or 0s based on pixel differences.
        // Algorithm ported from NoiseBasedCamRng by Andika Wasisto https://github.com/awasisto/camrng

        let mut entropy = Vec::new();
        let height = current.len() / width;

        // Define bounds for the grid
        let start_row = 100;
        let end_row = height - 100;

        if start_row >= end_row || width <= 200 {
            panic!("Resolution is too small to apply the grid selection with the given offset.");
        }

        // Process the grid within the defined bounds
        for row in start_row..end_row {
            let row_start = row * width + 100;
            let row_end = (row + 1) * width - 100;

            entropy.extend((row_start..row_end).step_by(minimum_distance)
                .filter(|&pixel_index| pixel_index < current.len() && pixel_index < previous.len())
                .map(|pixel_index| {
                    let c = current[pixel_index];
                    let p = previous[pixel_index];
                    (c > p) as u8 - (c < p) as u8 // 1 for c > p, 0 for c < p, skips if equal
                })
                .filter(|&bit| bit <= 1));
        }

        self.bits_to_bytes(&entropy)

    }

    pub fn pick_and_flip(&self, data: &[u8], low: u8, high: u8, current_frame_index: usize) -> Vec<u8> {
        // Extracts the least significant bit (LSB) of each pixel brightness, flipping it based on the frame index.
        // Generates entropy by combining these bits into bytes.
        // Algorithm is a simplified version of R. Li, "A True Random Number Generator algorithm from 
        // digital camera image noise for varying lighting conditions," doi: 10.1109/SECON.2015.7132901.

        let mut entropy = Vec::new();

        for &pixel in data {
            if (low..=high).contains(&pixel) { // filter bias
                let mut lsb = pixel & 1;
                if current_frame_index % 2 == 0 { // flip bits
                    lsb ^= 1;
                }
                entropy.push(lsb);
            }
        }

        self.bits_to_bytes(&entropy)
    }

    pub fn shannon(&self, data: &Vec<u8>) -> f64 {
        // Calculates the Shannon entropy of a given byte vector to measure its randomness.
        // Uses a frequency map to compute probabilities and their contributions to entropy.

        let mut frequency_map = HashMap::new();
        let data_len = data.len();

        for &byte in data {
            *frequency_map.entry(byte).or_insert(0) += 1;
        }

        frequency_map.values().fold(0.0, |entropy, &count| {
            let probability = count as f64 / data_len as f64;
            entropy - probability * probability.log2()
        })
    }

    pub fn whiten(&self, entropy: &[u8]) -> Vec<u8> {
        // Applies von Neumann whitening to reduce bias in the input entropy.
        // Pairs of bits are analyzed, and only unbiased pairs are used to construct the output.

        let mut whitened_entropy = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for byte in entropy {
            for i in (0..8).step_by(2) {
                let bit1 = (byte >> (7 - i)) & 1;
                let bit2 = (byte >> (6 - i)) & 1;

                match (bit1, bit2) {
                    (0, 1) => {
                        current_byte = (current_byte << 1) | 0;
                        bit_count += 1;
                    }
                    (1, 0) => {
                        current_byte = (current_byte << 1) | 1;
                        bit_count += 1;
                    }
                    _ => {}
                }

                if bit_count == 8 {
                    whitened_entropy.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }

        whitened_entropy
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

#[no_mangle]
pub extern "C" fn chop_and_tack(
    current_ptr: *const u8,
    current_len: usize,
    previous_ptr: *const u8,
    previous_len: usize,
    width: usize,
    minimum_distance: usize,
    result_ptr: *mut u8,
    result_len: &mut usize,
) {
    let current = unsafe { slice::from_raw_parts(current_ptr, current_len) };
    let previous = unsafe { slice::from_raw_parts(previous_ptr, previous_len) };

    let ocelli = Ocelli;
    let result = ocelli.chop_and_tack(&current.to_vec(), &previous.to_vec(), width, minimum_distance);

    unsafe {
        let result_slice = slice::from_raw_parts_mut(result_ptr, result.len());
        result_slice.copy_from_slice(&result);
        *result_len = result.len();
    }
}

#[no_mangle]
pub extern "C" fn pick_and_flip(
    data_ptr: *const u8,
    data_len: usize,
    low: u8,
    high: u8,
    current_frame_index: usize,
    result_ptr: *mut u8,
    result_len: &mut usize,
) {
    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };

    let ocelli = Ocelli;
    let result = ocelli.pick_and_flip(data, low, high, current_frame_index);

    unsafe {
        let result_slice = slice::from_raw_parts_mut(result_ptr, result.len());
        result_slice.copy_from_slice(&result);
        *result_len = result.len();
    }
}


#[no_mangle]
pub extern "C" fn shannon(data_ptr: *const u8, data_len: usize) -> f64 {
    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };

    let ocelli = Ocelli;
    ocelli.shannon(&data.to_vec())
}

#[no_mangle]
pub extern "C" fn whiten(
    entropy_ptr: *const u8,
    entropy_len: usize,
    result_ptr: *mut u8,
    result_len: &mut usize,
) {
    let entropy = unsafe { slice::from_raw_parts(entropy_ptr, entropy_len) };

    let ocelli = Ocelli;
    let result = ocelli.whiten(entropy);

    unsafe {
        let result_slice = slice::from_raw_parts_mut(result_ptr, result.len());
        result_slice.copy_from_slice(&result);
        *result_len = result.len();
    }
}

#[no_mangle]
pub extern "C" fn is_covered(grayscale_ptr: *const u8, grayscale_len: usize, threshold: usize) -> bool {
    let grayscale = unsafe { slice::from_raw_parts(grayscale_ptr, grayscale_len) };

    let ocelli = Ocelli;
    ocelli.is_covered(grayscale, threshold)
}