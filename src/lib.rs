use std::slice;

mod core;
use crate::core::Ocelli;

#[no_mangle]
pub extern "C" fn chop_and_tack(
    current_ptr: *const u8,
    current_len: usize,
    previous_ptr: *const u8,
    previous_len: usize,
    width: usize,
    minimum_distance: usize,
    result_ptr: *mut u8,   // out buffer (caller-allocated)
    result_len: *mut usize // out: actual bytes written
) {
    if current_ptr.is_null() || previous_ptr.is_null() || result_ptr.is_null() || result_len.is_null() {
        return;
    }

    let current = unsafe { slice::from_raw_parts(current_ptr, current_len) };
    let previous = unsafe { slice::from_raw_parts(previous_ptr, previous_len) };

    let ocelli = Ocelli;
    let result = match ocelli.chop_and_tack(current, previous, width, minimum_distance) {
        Some(v) => v,
        None => {
            unsafe { *result_len = 0; }
            return;
        }
    };

    unsafe {
        let out = slice::from_raw_parts_mut(result_ptr, result.len());
        out.copy_from_slice(&result);
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
    result_ptr: *mut u8,   // out buffer (caller-allocated)
    result_len: *mut usize // out: actual bytes written
) {
    if data_ptr.is_null() || result_ptr.is_null() || result_len.is_null() {
        return;
    }
    if low >= high {
        unsafe { *result_len = 0; }
        return;
    }

    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };
    let ocelli = Ocelli;
    let result = ocelli.pick_and_flip(data, low, high, current_frame_index);

    unsafe {
        let out = slice::from_raw_parts_mut(result_ptr, result.len());
        out.copy_from_slice(&result);
        *result_len = result.len();
    }
}

#[no_mangle]
pub extern "C" fn shannon(
    data_ptr: *const u8,
    data_len: usize
) -> f64 {
    if data_ptr.is_null() {
        return 0.0;
    }
    let data = unsafe { slice::from_raw_parts(data_ptr, data_len) };
    let ocelli = Ocelli;
    ocelli.shannon(data)
}

#[no_mangle]
pub extern "C" fn whiten(
    entropy_ptr: *const u8,
    entropy_len: usize,
    result_ptr: *mut u8,   // out buffer (caller-allocated)
    result_len: *mut usize // out: actual bytes written
) {
    if entropy_ptr.is_null() || result_ptr.is_null() || result_len.is_null() {
        return;
    }

    let entropy = unsafe { slice::from_raw_parts(entropy_ptr, entropy_len) };
    let ocelli = Ocelli;
    let result = ocelli.whiten(entropy);

    unsafe {
        let out = slice::from_raw_parts_mut(result_ptr, result.len());
        out.copy_from_slice(&result);
        *result_len = result.len();
    }
}

#[no_mangle]
pub extern "C" fn is_covered(
    grayscale_ptr: *const u8,
    grayscale_len: usize,
    threshold: usize
) -> bool {
    if grayscale_ptr.is_null() {
        return false;
    }
    let grayscale = unsafe { slice::from_raw_parts(grayscale_ptr, grayscale_len) };
    let ocelli = Ocelli;
    ocelli.is_covered(grayscale, threshold)
}
