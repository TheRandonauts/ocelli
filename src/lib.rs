use std::collections::{HashMap, HashSet};
use std::slice;

pub struct Ocelli;

impl Ocelli {

    pub fn chop_and_tack(&self, current: &Vec<u8>, previous: &Vec<u8>, width: usize, minimum_distance: usize) -> Vec<u8> {
        let mut entropy = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        // Calculate the height of the frame
        let height = current.len() / width;

        // Skip 100 pixels at the top and bottom of the frame
        let start_row = 100;
        let end_row = height - 100;

        if start_row >= end_row || width <= 200 {
            panic!("Resolution is too small to apply the grid selection with the given offset.");
        }

        for row in start_row..end_row {
            // Skip 100 pixels at the left and right of the frame
            let row_start = row * width + 100;
            let row_end = (row + 1) * width - 100;

            // Select pixels in the row based on the step size
            for pixel_index in (row_start..row_end).step_by(minimum_distance) {
                if pixel_index >= current.len() || pixel_index >= previous.len() {
                    continue;
                }

                let c = current[pixel_index];
                let p = previous[pixel_index];

                if c > p {
                    current_byte = (current_byte << 1) | 1; // Append '1'
                } else if c < p {
                    current_byte = current_byte << 1; // Append '0'
                } else {
                    continue; // Skip if equal
                }

                bit_count += 1;

                if bit_count == 8 {
                    entropy.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }

        entropy
    }

    pub fn pick_and_flip(&self, data: &[u8], current_frame_index: usize) -> Vec<u8> {
        let mut entropy = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for &pixel_brightness in data {
            if (2..=253).contains(&pixel_brightness) {
                let mut lsb = pixel_brightness & 1;

                if current_frame_index % 2 == 0 {
                    lsb ^= 1; // Flip the bit
                }

                current_byte = (current_byte << 1) | lsb;
                bit_count += 1;

                if bit_count == 8 {
                    entropy.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }

        entropy
    }

    pub fn shannon(&self, data: &Vec<u8>) -> f64 {
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
    current_frame_index: usize,
    result_ptr: *mut u8,
    result_len: &mut usize,
) {
    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };

    let ocelli = Ocelli;
    let result = ocelli.pick_and_flip(data, current_frame_index);

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
