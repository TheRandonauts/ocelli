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
    fn get_entropy(&self, current: &Vec<u8>, previous: &Vec<u8>, width: usize, minimum_distance: usize) -> Vec<u8> {
        let mut entropy = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;
    
        // Calculate the height of the frame
        let height = current.len() / width;
    
        // Skip the first and last 100 rows (width * 100 pixels)
        let start_row = 100;
        let end_row = height - 100;
    
        if start_row >= end_row || width <= 200 {
            panic!("Resolution is too small to apply the grid selection with the given offset.");
        }
    
        // Iterate through rows, skipping the top and bottom 100 rows
        for row in start_row..end_row {
            // Skip the first 100 pixels in the row
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
    fn shannon(&self, data: &Vec<u8>) -> f64 {
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
    fn is_covered(&self, grayscale: &Vec<u8>, threshold: usize) -> bool {
        let unique_values: HashSet<_> = grayscale.iter().copied().collect();
        unique_values.len() < threshold
    }
}

fn main() -> opencv::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <camera index> <entropy length in bytes> [-w]", args[0]);
        std::process::exit(1);
    }

    let camera_index: i32 = args[1].parse().expect("Failed to parse camera index as a number");
    let length: usize = args[2].parse().expect("Failed to parse entropy length as a number");
    let whiten_flag = args.contains(&String::from("-w"));

    let mut cam = VideoCapture::new(camera_index, CAP_V4L)?;
    if !cam.is_opened()? {
        panic!("Failed to open the camera with index {}", camera_index);
    }

    // Capture a single frame to determine resolution
    let mut frame = core::Mat::default();
    cam.read(&mut frame)?;
    if frame.empty() {
        panic!("Failed to capture a frame.");
    }

    let width = frame.cols() as usize;
    let height = frame.rows() as usize;

    println!(
        "Camera index: {}\nCamera resolution detected: {}x{}",
        camera_index, width, height
    );

    let ocelli = Ocelli;

    // Convert the frame to grayscale and check if the camera is covered
    let mut gray_frame = core::Mat::default();
    imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0)?;
    let grayscale_data = gray_frame.data_bytes().expect("Failed to get grayscale data").to_vec();

    let uncovered = !ocelli.is_covered(&grayscale_data, 50);

    println!(
        "Camera is {}covered.",
        if uncovered {
            "un"
        } else {
            ""
        }
    );

    let mut total_entropy = Vec::new();
    let shannon_threshold = 4.5;
    let mut previous_frame_data = grayscale_data.clone();

    let start_time = Instant::now();

    while total_entropy.len() < length {
        cam.read(&mut frame)?;
        let mut gray_frame = core::Mat::default();
        imgproc::cvt_color(&frame, &mut gray_frame, imgproc::COLOR_BGR2GRAY, 0)?;
        let current_frame_data = gray_frame.data_bytes().expect("Failed to get grayscale data").to_vec();

        let mut entropy: Vec<u8> = Vec::new();

        entropy = ocelli.get_entropy(&current_frame_data, &previous_frame_data, width, 20);
        previous_frame_data = current_frame_data;

        if whiten_flag {
            entropy = ocelli.whiten(&entropy);
        }

        let shannon_entropy = ocelli.shannon(&entropy);

        if shannon_entropy >= shannon_threshold {
            total_entropy.extend(entropy);
        } else {
            println!(
                "Rejected entropy array (Shannon entropy: {:.3}). Retrying...",
                shannon_entropy
            );
        }

        println!(
            "Collected {} of {} bytes of entropy...",
            total_entropy.len(),
            length
        );
    }

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
