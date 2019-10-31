use simdnoise::*;

/// Generate a perlin noise at position x,y,z for a block of size (size, size, size)
/// Point in the block are distant of 1
/// Use the parameter factor to set the scale factor
pub fn perlin(
    mut x: f32,
    mut y: f32,
    mut z: f32,
    size : usize,
    mut factor : f32,
    octave: i32,
    persistance: f32,
    mut seed: i32,
) -> Vec<f32> {
    let mut result = vec![0.0; size*size*size];
    let mut p = 1.0;
    let mut div = 0.0; // normalization factor

    for _i in 0..octave {
        let noise = NoiseBuilder::gradient_3d_offset(x, size, y, size, z, size).with_seed(seed).with_freq(factor).generate_scaled(0.0, 1.0);
        for _j in 0..(size*size*size){
            result[_j] = result[_j] + p*noise[_j];
        }
        factor *= 2.0;
        seed += 1;
        div += p;
        p *= persistance;
    }

    for _j in 0..(size*size*size){
        result[_j] = result[_j]/div;
    }
    return result;
}

