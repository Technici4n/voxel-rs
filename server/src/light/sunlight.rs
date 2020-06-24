use voxel_rs_common::world::{Chunk, CHUNK_SIZE};
use super::HighestOpaqueBlock;
use std::sync::Arc;

// TODO : Add block that are source of light

pub struct LightData {
    pub light_level: [u8; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
}

impl LightData {
    pub fn new() -> Self {
        Self {
            light_level: [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }
}

/// Take a 3x3x3 chunks bloc and 3x3 HighestOpaqueBlock and compute the light by using a BFS
pub fn compute_light(
    chunks: Vec<Option<Arc<Chunk>>>,
    highest_opaque_blocks: Vec<Arc<HighestOpaqueBlock>>,
    queue: &mut FastBFSQueue,
    light_data: &mut [u8],
    opaque: &mut [bool],
) -> LightData {
    assert!(light_data.len() >= (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize);
    assert!(opaque.len() >= (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize);
    let mut res = LightData::new();
    queue.clear();

    const MAX_LIGHT: u32 = 15;

    //let mut light_data = [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize];
    //let mut opaque = [false; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize];
    let csize = CHUNK_SIZE as usize;

    let mut transparent_count = 0;
    let c = chunks[9 + 3 + 1].clone().unwrap();
    unsafe {
        let y0 = c.pos.py; // Center chunk height

        'triple_loop: for cx in [1, 0, 2].iter() {
            for cy in [1, 0, 2].iter() {
                for cz in [1, 0, 2].iter() {
                    if *cx != 1 && *cy != 1 && *cz != 1 && transparent_count == 0 {
                        break 'triple_loop;
                    }

                    let chunk = chunks[*cx * 9 + *cy * 3 + *cz].clone();
                    let highest_opaque_block = &highest_opaque_blocks[*cx * 3 + *cz];
                    // First we compute the range of the blocks we have to check in the chunk.
                    let mut i_range = 0..CHUNK_SIZE;
                    let mut j_range = 0..CHUNK_SIZE;
                    let mut k_range = 0..CHUNK_SIZE;
                    if *cx == 0 {
                        i_range = (CHUNK_SIZE - MAX_LIGHT + 1)..CHUNK_SIZE;
                    } else if *cx == 2 {
                        i_range = 0..(MAX_LIGHT - 1);
                    }
                    if *cy == 0 {
                        j_range = (CHUNK_SIZE - MAX_LIGHT + 1)..CHUNK_SIZE;
                    } else if *cy == 2 {
                        j_range = 0..(MAX_LIGHT - 1);
                    }
                    if *cz == 0 {
                        k_range = (CHUNK_SIZE - MAX_LIGHT + 1)..CHUNK_SIZE;
                    } else if *cz == 2 {
                        k_range = 0..(MAX_LIGHT - 1);
                    }
                    // Then we fill the BFS queue
                    match chunk {
                        None => {
                            for i in i_range {
                                for k in j_range.clone() {
                                    for j in k_range.clone() {
                                        let s = (*cx * csize + i as usize) * csize * csize * 9
                                            + (*cy * csize + j as usize) * csize * 3
                                            + (*cz * csize + k as usize);
                                        *opaque.get_unchecked_mut(s) = false;
                                        if (y0 + *cy as i64 - 1) * CHUNK_SIZE as i64 + j as i64
                                            > *highest_opaque_block
                                                .y
                                                .get_unchecked((i * CHUNK_SIZE + k) as usize)
                                        {
                                            *light_data.get_unchecked_mut(s) = 15;
                                            queue.push((
                                                *cx * csize + i as usize,
                                                *cy * csize + j as usize,
                                                *cz * csize + k as usize,
                                                15,
                                            ));
                                        } else {
                                            *light_data.get_unchecked_mut(s) = 0;
                                        }
                                    }
                                }
                            }
                        }
                        Some(c) => {
                            for i in i_range {
                                for j in j_range.clone() {
                                    for k in k_range.clone() {
                                        let s = (*cx * csize + i as usize) * csize * csize * 9
                                            + (*cy * csize + j as usize) * csize * 3
                                            + (*cz * csize + k as usize);
                                        if c.get_block_at_unsafe((i, j, k)) != 0 {
                                            // TODO : replace by is opaque
                                            *opaque.get_unchecked_mut(s) = true;
                                        } else {
                                            *opaque.get_unchecked_mut(s) = false;
                                            if c.pos.py * CHUNK_SIZE as i64 + j as i64
                                                > *highest_opaque_block
                                                    .y
                                                    .get_unchecked((i * CHUNK_SIZE + k) as usize)
                                            {
                                                *light_data.get_unchecked_mut(s) = 15;
                                                queue.push((
                                                    *cx * csize + i as usize,
                                                    *cy * csize + j as usize,
                                                    *cz * csize + k as usize,
                                                    15,
                                                ));
                                            } else {
                                                *light_data.get_unchecked_mut(s) = 0;
                                                if *cx == 1 && *cy == 1 && *cz == 1 {
                                                    transparent_count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        const MIN_VAL: isize = CHUNK_SIZE as isize - MAX_LIGHT as isize + 1;
        const MAX_VAL: isize = 2 * CHUNK_SIZE as isize + MAX_LIGHT as isize;
        const DX: [isize; 6] = [1, -1, 0, 0, 0, 0];
        const DY: [isize; 6] = [0, 0, 1, -1, 0, 0];
        const DZ: [isize; 6] = [0, 0, 0, 0, 1, -1];

        while !queue.is_empty() && transparent_count > 0 {
            let (x, y, z, ll) = *queue.pop();
            for i in 0..6 {
                let (nx, ny, nz) = (x as isize + DX[i], y as isize + DY[i], z as isize + DZ[i]);
                if MIN_VAL <= nx
                    && nx < MAX_VAL
                    && MIN_VAL <= ny
                    && ny < MAX_VAL
                    && MIN_VAL <= nz
                    && nz < MAX_VAL
                {
                    let s = (nx as usize) * csize * csize * 9 + (ny as usize) * csize * 3 + (nz as usize);
                    if *opaque.get_unchecked(s as usize) { continue; }
                    let ref_light = light_data.get_unchecked_mut(s as usize);
                    if *ref_light < ll - 1 {
                        *ref_light = ll - 1;
                        if ll > 1 {
                            queue.push((nx as usize, ny as usize, nz as usize, ll - 1));
                        }
                        if nx as usize / csize == 1
                            && ny as usize / csize == 1
                            && nz as usize / csize == 1
                            && !*opaque.get_unchecked(
                                nx as usize * csize * csize * 9
                                    + ny as usize * csize * 3
                                    + nz as usize,
                            )
                        {
                            transparent_count -= 1;
                        }
                    }
                }
            }
        }

        for i in 0..csize {
            for j in 0..csize {
                for k in 0..csize {
                    res.light_level[i * csize * csize + j * csize + k] = *light_data.get_unchecked(
                        (i + csize) * csize * csize * 9 + (j + csize) * 3 * csize + (k + csize),
                    );
                }
            }
        }
    }

    return res;
}

/// A structure to fasten the light computation
/// Extremely unsafe
pub struct FastBFSQueue {
    data: Vec<(usize, usize, usize, u8)>,
    pop_index: usize,
    push_index: usize,
}

impl FastBFSQueue {
    pub fn new() -> Self {
        let mut data = Vec::new();
        for _ in 0..(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) {
            data.push((0, 0, 0, 0));
        }

        Self {
            data,
            pop_index: 0,
            push_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.pop_index == self.push_index
    }

    #[inline(always)]
    pub unsafe fn pop(&mut self) -> &(usize, usize, usize, u8) {
        let res = self.data.get_unchecked(self.pop_index);
        self.pop_index =
            (self.pop_index + 1) % (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize;
        //assert_ne!(self.pop_index, self.push_index);
        return res;
    }

    #[inline(always)]
    pub unsafe fn push(&mut self, to_push: (usize, usize, usize, u8)) {
        *self.data.get_unchecked_mut(self.push_index) = to_push;
        self.push_index =
            (self.push_index + 1) % (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize;
        //assert_ne!(self.pop_index, self.push_index);
    }

    pub fn clear(&mut self) {
        self.pop_index = 0;
        self.push_index = 0;
    }
}
