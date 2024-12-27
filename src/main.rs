use opencv::prelude::*;
use opencv::videoio::{VideoCapture, CAP_V4L};
use opencv::imgproc;
use opencv::core;
use std::collections::HashSet;
use std::collections::HashMap;
use std::env;
use std::time::Instant;

struct Ocelli;

impl Ocelli {
    /// Calculates the entropy bits based on two arrays of grayscale values
    fn get_entropy(&self, current: &[u8], previous: &[u8]) -> Vec<u8> {
        let mut entropy = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for (&c, &p) in current.iter().zip(previous.iter()) {
            if c > p {
                current_byte = (current_byte << 1) | 1; // Append '1' to the byte
            } else if c < p {
                current_byte = current_byte << 1; // Append '0' to the byte
            } else {
                continue; // Skip if values are equal
            }

            bit_count += 1;

            // Push the byte once we have 8 bits
            if bit_count == 8 {
                entropy.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }

        entropy
    }

    /// Apply Van Neumann whitening to a vector of entropy bits
    fn whiten(&self, entropy: &Vec<u8>) -> Vec<u8> {
        let mut whitened_entropy = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        // Iterate through the entropy bytes and process bits in pairs
        for byte in entropy {
            for i in (0..8).step_by(2) {
                let bit1 = (byte >> (7 - i)) & 1;
                let bit2 = (byte >> (6 - i)) & 1;

                // Apply Van Neumann rules
                match (bit1, bit2) {
                    (0, 1) => {
                        current_byte = (current_byte << 1) | 0; // Append '0'
                        bit_count += 1;
                    }
                    (1, 0) => {
                        current_byte = (current_byte << 1) | 1; // Append '1'
                        bit_count += 1;
                    }
                    _ => {} // Discard (0,0) and (1,1) pairs
                }

                // If we have a full byte, push it to the output
                if bit_count == 8 {
                    whitened_entropy.push(current_byte);
                    current_byte = 0;
                    bit_count = 0;
                }
            }
        }

        whitened_entropy
    }

    /// Calculates the Shannon entropy of binary data
    fn shannon(&self, data: &[u8]) -> f64 {
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

    /// Calculates the required number of frames for the desired entropy bytes
    fn required_frames(&self, bytes: usize, width: usize, height: usize) -> usize {
        let max_bytes = width * height / 8;

        if max_bytes == 0 {
            panic!("Invalid resolution: too few pixels to generate entropy.");
        }

        (bytes + max_bytes - 1) / max_bytes + 1
    }

    /// Determines if the camera is covered based on the unique grayscale values
    fn is_covered(&self, grayscale: &[u8], threshold: usize) -> bool {
        let unique_values: HashSet<_> = grayscale.iter().copied().collect();
        unique_values.len() < threshold
    }
}

fn main() -> opencv::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <entropy length in bytes> <resolution width> <resolution height>", args[0]);
        std::process::exit(1);
    }

    // Check if the whitening flag is set
    let whiten_flag = args.contains(&String::from("-w"));

    let length: usize = args[1].parse().expect("Failed to parse entropy length as a number");
    let width: usize = args[2].parse().expect("Failed to parse resolution width as a number");
    let height: usize = args[3].parse().expect("Failed to parse resolution height as a number");

    let mut cam = VideoCapture::new(1, CAP_V4L)?;
    if !cam.is_opened()? {
        panic!("Failed to open the camera");
    }

    // Set camera resolution
    cam.set(opencv::videoio::CAP_PROP_FRAME_WIDTH, width as f64)?;
    cam.set(opencv::videoio::CAP_PROP_FRAME_HEIGHT, height as f64)?;

    println!("Camera resolution set to {}x{}", width, height);

    let ocelli = Ocelli;

    // Capture a single frame to check if the camera is covered
    let mut frame = core::Mat::default();
    cam.read(&mut frame)?;
    let mut gray_frame = core::Mat::default();
    imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0)?;
    let grayscale_data = gray_frame.data_bytes().expect("Failed to get grayscale data");

    if !ocelli.is_covered(grayscale_data, 50) {
        println!("Camera is not covered. Stopping...");
        // return Ok(());
    }

    // Start timing
    let start_time = Instant::now();

    // Calculate required frames
    let required_frames = ocelli.required_frames(length, width, height);
    println!("Capturing {} frames to generate {} bytes of entropy...", required_frames, length);

    // Generate entropy
    let mut total_entropy = Vec::new();
    let shannon_threshold = 4.0;

    let mut previous_frame_data = grayscale_data.to_vec();

    while total_entropy.len() < length {
        cam.read(&mut frame)?;
        let mut gray_frame = core::Mat::default();
        imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0)?;
        let current_frame_data = gray_frame.data_bytes().expect("Failed to get grayscale data").to_vec();

        let entropy = ocelli.get_entropy(&current_frame_data, &previous_frame_data);
        let shannon_entropy = ocelli.shannon(&entropy);

        if shannon_entropy >= shannon_threshold {
            total_entropy.extend(entropy);
        } else {
            println!(
                "Rejected entropy array (Shannon entropy: {:.3}). Retrying...",
                shannon_entropy
            );
        }

        previous_frame_data = current_frame_data;
    }

    if whiten_flag {
        total_entropy = ocelli.whiten(&total_entropy);
    }

    // Convert entropy to hex string
    let entropy_hex = total_entropy
        .iter()
        .take(length)
        .map(|byte| format!("{:02x}", byte))
        .collect::<String>();

    println!("Generated entropy (hex): {}", entropy_hex);
    
    let total_shannon_entropy = ocelli.shannon(&total_entropy);
    
    // End timing
    let elapsed_time = start_time.elapsed();
    println!(
        "Process completed in {:.3} seconds.\nShannon Entropy {:.3}.",
        elapsed_time.as_secs_f64(), total_shannon_entropy
    );

    Ok(())
}