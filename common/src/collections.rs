/// Create a new zero-initialized vector
pub unsafe fn zero_initialized_vec<T>(size: usize) -> Vec<T> {
    let mut v: Vec<T> = Vec::with_capacity(size);
    std::ptr::write_bytes(v.as_mut_ptr(), 0u8, size);
    v.set_len(size);
    v
}

/// Fill a vector with zeroes
pub unsafe fn zero_vec<T>(v: &mut Vec<T>) {
    std::ptr::write_bytes(v.as_mut_ptr(), 0u8, v.len());
}
