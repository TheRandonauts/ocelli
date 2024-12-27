# ocelli: Camera-Based TRNG

**Ocelli** is a Rust application that uses a camera to generate high-quality random entropy by analyzing pixel intensity differences between consecutive frames.

## Features

- **Camera-Based Entropy**: Leverages grayscale pixel intensity differences to produce entropy bits.
- **Shannon Entropy Validation**: Filters entropy data to ensure high randomness quality.
- **Van Neumann Whitening**: Optional bias removal for unbiased entropy output.
- **Dynamic Frame Handling**: Automatically captures frames based on resolution and entropy requirements.

## Requirements

- OpenCV (with Rust bindings)

## Usage

Run the program with the desired parameters:

```bash
cargo run -- <entropy_length> <resolution_width> <resolution_height> [-w]
```
### Parameters:
- `<entropy_length>`: Number of bytes of entropy to generate.
- `<resolution_width>`: Camera resolution width.
- `<resolution_height>`: Camera resolution height.
- `-w`: (Optional) Enable Van Neumann whitening for unbiased entropy.

### Example:
Generate 1000 bytes of entropy at 1920x1080 resolution with whitening enabled:
```bash
cargo run -- 1000 1920 1080 -w
```

---

## Output

- **Hexadecimal Entropy**: Generated entropy is output as a hexadecimal string.
- **Processing Time**: Reports how long the entropy generation took.
- **Shannon Entropy**: Displays the Shannon entropy of the final output.
