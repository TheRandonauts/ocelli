# ocelli: Camera-Based TRNG

**Ocelli** is a Rust application that generates high-quality entropy using a camera feed. The application supports two entropy generation methods (`get_entropy` and `chop_and_stack`) and optionally applies Van Neumann whitening for enhanced randomness. Generated entropy is saved as a binary file.

## Features

- **Entropy Methods**:
  - `get_entropy`: Compares pixel values between frames.
  - `chop_and_stack`: Combines and processes pixel data with reversed rows.
- **Van Neumann Whitening**: Optional, enabled via the `-w` flag.
- **Shannon Entropy Test**: Ensures the randomness quality of generated entropy.

## Usage

```bash
cargo run --release -- <entropy_length_in_bytes> <resolution_width> <resolution_height> [-w]
```

### Example

```bash
cargo run --release -- 1024 640 480 -w
```

This generates 1024 bytes of whitened entropy using a 640x480 resolution.

## Requirements

- OpenCV 4.x

## Installation

1. Install Rust: https://www.rust-lang.org/tools/install
2. Install OpenCV: Follow the [official guide](https://docs.opencv.org/).
3. Clone the repository:
   ```bash
   git clone <repository_url>
   cd ocelli-entropy-generator
   ```
4. Run the application:
   ```bash
   cargo run --release -- <arguments>
   ```

## Output

Generated entropy files are saved in the current directory with a name format:
```
<method>[_whitened]_YYYYMMDD_HHMMSS.bin
```