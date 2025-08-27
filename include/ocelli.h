#pragma once
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

void chop_and_tack(const uint8_t* current, size_t current_len,
                   const uint8_t* previous, size_t previous_len,
                   size_t width, size_t minimum_distance,
                   uint8_t* result_ptr, size_t* result_len);

void pick_and_flip(const uint8_t* data, size_t data_len,
                   uint8_t low, uint8_t high, size_t current_frame_index,
                   uint8_t* result_ptr, size_t* result_len);

double shannon(const uint8_t* data, size_t data_len);

void whiten(const uint8_t* entropy, size_t entropy_len,
            uint8_t* result_ptr, size_t* result_len);

bool is_covered(const uint8_t* grayscale, size_t grayscale_len, size_t threshold);

#ifdef __cplusplus
}
#endif
