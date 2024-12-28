use opencv::prelude::*;
use opencv::videoio::{VideoCapture, CAP_V4L};
use opencv::imgproc;
use opencv::core;
use std::collections::HashSet;
use std::collections::HashMap;
use std::env;
use std::time::Instant;
use std::fs::File;
use std::io::Write;
use chrono::Local;

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

    /// Use the chop & stack method on an array of grayscale values
    fn chop_and_stack(&self, mut data: Vec<u8>) -> Vec<u8> {
        // Ensure the input length is divisible by 4 by trimming excess bytes
        let remainder = data.len() % 4;
        if remainder != 0 {
            data.truncate(data.len() - remainder);
        }
    
        // Determine the length of each fold
        let fold_len = data.len() / 4;
    
        // Split the data into four folds
        let mut folds: Vec<Vec<u8>> = data
        .chunks(fold_len)
        .map(|chunk| chunk.to_vec())
        .collect();

        // Reverse the second and fourth folds
        if folds.len() > 1 {
            folds[1].reverse(); // Reverse the second row
        }
        if folds.len() > 3 {
            folds[3].reverse(); // Reverse the fourth row
        }
        
        // Combine the folds by summing corresponding elements and applying modulo 256
        let mut combined: Vec<u8> = vec![0u8; fold_len];
        for i in 0..fold_len {
            combined[i] = ((folds[0][i] as u16
                + folds[1][i] as u16
                + folds[2][i] as u16
                + folds[3][i] as u16) % 256) as u8;
        }
    
        combined
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

    let whiten_flag = args.contains(&String::from("-w"));

    let length: usize = args[1].parse().expect("Failed to parse entropy length as a number");
    let width: usize = args[2].parse().expect("Failed to parse resolution width as a number");
    let height: usize = args[3].parse().expect("Failed to parse resolution height as a number");

    let mut cam = VideoCapture::new(1, CAP_V4L)?;
    if !cam.is_opened()? {
        panic!("Failed to open the camera");
    }

    cam.set(opencv::videoio::CAP_PROP_FRAME_WIDTH, width as f64)?;
    cam.set(opencv::videoio::CAP_PROP_FRAME_HEIGHT, height as f64)?;

    println!("Camera resolution set to {}x{}", width, height);

    let ocelli = Ocelli;

    // Capture a single frame to check if the camera is covered
    let mut frame = core::Mat::default();
    cam.read(&mut frame)?;
    let mut gray_frame = core::Mat::default();
    imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0)?;
    let grayscale_data = gray_frame.data_bytes().expect("Failed to get grayscale data").to_vec();

    let uncovered = !ocelli.is_covered(&grayscale_data, 50);

    println!(
        "Starting entropy generation... Using {}",
        if uncovered {
            "chop_and_stack"
        } else {
            "get_entropy"
        }
    );

    let mut total_entropy = Vec::new();
    let shannon_threshold = 4.0;
    let mut previous_frame_data = grayscale_data.clone();

    let start_time = Instant::now();

    while total_entropy.len() < length {
        cam.read(&mut frame)?;
        let mut gray_frame = core::Mat::default();
        imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0)?;
        let current_frame_data = gray_frame.data_bytes().expect("Failed to get grayscale data").to_vec();

        let mut entropy: Vec<u8> = Vec::new();
        let mut shannon_entropy = 0.0;

        if uncovered {
            // Process with chop_and_stack
            entropy = ocelli.chop_and_stack(current_frame_data.clone());

        } else {
            // Process with get_entropy
            entropy = ocelli.get_entropy(&current_frame_data, &previous_frame_data);

            previous_frame_data = current_frame_data;
        }

        if whiten_flag {
            entropy = ocelli.whiten(&entropy);
        }

        shannon_entropy = ocelli.shannon(&entropy);

        if shannon_entropy >= shannon_threshold {
            
            total_entropy.extend(entropy);
        
        } else {
            println!(
                "Rejected stacked entropy array (Shannon entropy: {:.3}). Retrying...",
                shannon_entropy
            );
        }

        println!(
            "Collected {} of {} bytes of entropy...",
            total_entropy.len(),
            length
        );
    }

    // Convert entropy to hex string
    // let entropy_hex = total_entropy
    //     .iter()
    //     .take(length)
    //     .map(|byte| format!("{:02x}", byte))
    //     .collect::<String>();
    // println!("Generated entropy (hex): {}", entropy_hex);

    let total_shannon_entropy = ocelli.shannon(&total_entropy);

    let elapsed_time = start_time.elapsed();
    println!(
        "Process completed in {:.3} seconds.\nShannon Entropy {:.3}.",
        elapsed_time.as_secs_f64(),
        total_shannon_entropy
    );

    // Save the generated entropy to a binary file
    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let method = if uncovered { "chop_and_stack" } else { "get_entropy" };
    let whitened = if whiten_flag { "_whitened" } else { "" };
    let filename = format!("{}{}_{}.bin", method, whitened, timestamp);

    let mut file = File::create(&filename).expect("Failed to create file");
    file.write_all(&total_entropy).expect("Failed to write data to file");

    println!("Entropy saved to file: {}", filename);

    Ok(())
}