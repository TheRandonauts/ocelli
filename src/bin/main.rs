use ocelli::Ocelli;
use v4l::prelude::*;
use v4l::video::Capture;
use v4l::buffer::Type;
use v4l::format::FourCC;
use v4l::io::traits::CaptureStream;
use image::ImageReader;
use std::fs::File;
use std::io::{stdin, Write};
use chrono::Local;
use std::time::Instant;

fn frame_to_grayscale(data: &[u8]) -> Vec<u8> {
    let img = ImageReader::new(std::io::Cursor::new(data))
        .with_guessed_format()
        .expect("Failed to guess format")
        .decode()
        .expect("Failed to decode image");
    let gray = img.into_luma8(); // Convert to grayscale
    gray.into_raw() // Return raw pixel data as Vec<u8>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <camera index> <entropy length in bytes>", args[0]);
        std::process::exit(1);
    }

    // Check if the quick flag is set
    let quick = args.contains(&String::from("-q"));

    let camera_index: usize = args[1].parse().expect("Failed to parse camera index as a number");
    let length: usize = args[2].parse().expect("Failed to parse entropy length as a number");

    let dev = Device::new(camera_index).expect("Failed to open camera");

    // Set the desired format and resolution
    let mut format = dev.format().expect("Failed to get camera format");
    format.fourcc = FourCC::new(b"MJPG"); // Use MJPG for higher resolutions
    format.width = 1920;
    format.height = 1080;

    if let Err(_) = dev.set_format(&format) {
        println!("Failed to set resolution to 1920x1080. Falling back to 1280x720.");
        format.width = 1280;
        format.height = 720;
        dev.set_format(&format)
            .expect("Failed to set resolution to 1280x720");
    }

    println!(
        "Using resolution: {}x{} (FourCC: {})",
        format.width, format.height, format.fourcc
    );

    let mut stream = MmapStream::with_buffers(&dev, Type::VideoCapture, 4)
        .expect("Failed to create stream");

    let ocelli = Ocelli;
    let mut total_entropy = Vec::new();
    let start_time = Instant::now();
    let shannon_threshold = 7.9;
    let mut frame_count = 0;

    while total_entropy.len() < length {
        // Capture first frame
        let (data1, _) = stream.next().expect("Failed to capture frame");

        // Skip the first 30 frames
        if frame_count <= 30 {
            frame_count += 1;
        } else {

            let grayscale_data1 = frame_to_grayscale(&data1);

            let mut entropy: Vec<u8> = [0].to_vec();
            
            if quick {
                // Quicker capture using Pick and Flip
                entropy = ocelli.whiten(&ocelli.pick_and_flip(&grayscale_data1, frame_count as usize));
            } else {
                if ocelli.is_covered(&grayscale_data1, 50) {
                    // Capture second frame
                    let (data2, _) = stream.next().expect("Failed to capture second frame");
                    let grayscale_data2 = frame_to_grayscale(&data2);

                    // Generate entropy using chop_and_tack
                    entropy = ocelli.chop_and_tack(&grayscale_data1, &grayscale_data2, format.width as usize, 30);
                } else {
                    println!("Camera is not covered. Please cover the camera.");
                    frame_count = 0;
                }
            }

            let shannon_entropy = ocelli.shannon(&entropy);

            if shannon_entropy >= shannon_threshold {
                total_entropy.extend(entropy);
                println!(
                    "Collected {} of {} bytes of entropy (Shannon entropy: {:.3})",
                    total_entropy.len(),
                    length,
                    shannon_entropy
                );
            } else {
                println!("Rejected entropy for frame {} (Shannon entropy: {:.3})", frame_count, shannon_entropy);
            }
            
        }
    }

    // Trim total_entropy to the exact length
    total_entropy.truncate(length);

    // Print elapsed time
    let elapsed_time = start_time.elapsed();
    println!(
        "Process completed in {:.3} seconds.",
        elapsed_time.as_secs_f64()
    );

    // Final Shannon entropy test
    let final_shannon_entropy = ocelli.shannon(&total_entropy);
    println!("Final Shannon entropy: {:.3}", final_shannon_entropy);

    // Ask if the result should be saved
    println!("Save result to a file or print as hex string? (file/print):");
    let mut input = String::new();
    stdin().read_line(&mut input).expect("Failed to read input");

    if input.trim().eq_ignore_ascii_case("file") {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("entropy_{}.bin", timestamp);

        let mut file = File::create(&filename).expect("Failed to create file");
        file.write_all(&total_entropy).expect("Failed to write data to file");

        println!("Entropy saved to file: {}", filename);
    } else if input.trim().eq_ignore_ascii_case("print") {
        let entropy_hex: String = total_entropy.iter().map(|b| format!("{:02x}", b)).collect();
        println!("Generated entropy (hex): {}", entropy_hex);
    }

    Ok(())
}
