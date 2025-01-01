# ocelli: Camera-Based TRNG

**Ocelli** is a Rust library with FFI bindings that can be used to generate high-quality entropy using a camera feed. The application supports two entropy generation methods (`chop_and_tack` and `pick_and_flip`) and optionally applies Van Neumann whitening for enhanced randomness.

### Chop and Tack

(ported from NoiseBasedCamRng by Andika Wasisto https://github.com/awasisto/camrng)

The algorithm extracts entropy from two arrays with 8 bit integers that can be obtained by taking two consecutive frames from a camera feed and reading their brightness levels. It is required to cover the lens of the camera so it only sees uniform blackness. Due to thermal and quantum effects the image sensor will still sense fluctuations in brightness.

The outer 100 pixel wide edges of each frames are ignored since they can be prone to bias. Entropy is obtained by comparing the remaining pixel values in a grid pattern 30 pixels apart to avoid correlation.

### Pick and Flip

(inspired by R. Li, "A True Random Number Generator algorithm from digital camera image noise for varying lighting conditions," SoutheastCon 2015, Fort Lauderdale, FL, USA, 2015, pp. 1-8, doi: 10.1109/SECON.2015.7132901.)

The algorithm extracts entropy from an array of 8-bit values (camera frame pixel brightness) by analyzing the least significant bit (LSB) of each value. This process leverages the natural variability in pixel brightness across a frame. Entropy is derived by first examining whether the brightness value of a pixel falls within the range of 2 to 253, to avoid bias. The LSB of qualifying pixel values is then used to form a bitstream. To avoid correlations, the bits of every second array are flipped. The resulting bits are sequentially packed into bytes, forming the output entropy.

## Main Methods

* **`chop_and_tack`** takes two 8 bit integer arrays `current` and `previous` representing consecutive grayscale image frames, an usize `width` and an usize `height` of the original image frame dimensions, and a `minimum_distance` usize to define the grid distance between qualifying pixels. It returns an array of random 8 bit chunks.

* **`pick_and_flip`** takes an 8 bit integer array representing a grayscale image frame and an usize `current_frame_index` representing a frame count of which every even number triggers flipped bits for the frame for the output array of random 8 bit chunks.

## Helper Methods

* **`is_covered`** can be used to check if a camera lens is covered (a requirement for the Chop and Tack method to work properly). It takes an 8 bit integer array and an 8 bit integer `threshold` value. It checks how many unique values are present in the array. If the number of unique values lies under the threshold the method returns `true`.

* **`shannon`** can be used to calculate the Shannon Entropy value for an array of 8 bit integers.

* **`whiten`** applies Van Neumann whitening to an array of 8 bit values, halving its size but increasing the entropy amount by filtering out bias.

## Recommended Use

1. If *Chop and Tack* is to be used, utilize the `is_covered` method with a threshold of 50 to determine if the camera sensor is covered.
2. Read the desired amount of frames from the camera and extract the brightness levels as 8 bit integers into arrays.
3. Feed the arrays into one of the main methods and make sure to provide all required arguments.
4. Whitening can be applied using the `whiten` method to filter out bias and increase the entropy of the result
5. It is recommended to check the resulting entropy quality using the `shannon` method and drop the result if it falls below a threshold (e.g. 7.9).
6. Loop through the previous steps and accumulate the resulting entropy until the desired amount of random bytes is reached.

### Build

```bash
cargo build --release
```

Android
```bash
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 build --result
```

iOS
```bash
cargo build --release --target aarch64-apple-ios
cargo build --release --target x86_64-apple-ios
```