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

/// Merge sorted arrays
pub fn merge_arrays<T: Ord + Copy>(output: &mut Vec<T>, input: &[Vec<T>]) {
    output.clear();
    let n = input.len();
    let mut indices = vec![0; n];
    loop {
        let mut lowest = None;
        let mut j = n;
        for i in 0..n {
            if indices[i] < input[i].len() {
                let el = input[i][indices[i]];
                match &mut lowest {
                    None => {
                        lowest = Some(el);
                        j = i;
                    },
                    Some(current) => {
                        if *current < el {
                            *current = el;
                            j = i
                        }
                    }
                }
            }
        }
        if j == n {
            break
        } else {
            output.push(lowest.unwrap());
            indices[j] += 1;
        }
    }
}
