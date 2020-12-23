//! Wrappers around the `wgpu::Buffer` struct.

use std::collections::HashMap;
use std::hash::Hash;

use super::{ buffer_from_slice, to_u8_slice };

/// A buffer that will automatically resize itself when necessary
pub struct DynamicBuffer<T: Copy> {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsage,
    capacity: usize,
    len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T: Copy + 'static> DynamicBuffer<T> {
    /// Create a new `DynamicBuffer` with enough capacity for `initial_capacity` elements of type `T`
    pub fn with_capacity(
        device: &wgpu::Device,
        initial_capacity: usize,
        mut usage: wgpu::BufferUsage,
    ) -> Self {
        usage |= wgpu::BufferUsage::COPY_DST;
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                label: None,
                size: (initial_capacity * std::mem::size_of::<T>()) as u64,
                usage,
            }),
            usage,
            capacity: initial_capacity,
            len: 0,
            phantom: std::marker::PhantomData,
        }
    }

    /// Update the data of the buffer, resizing if needed
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        data: &[T],
    ) {
        if data.is_empty() {
            self.len = 0;
            return;
        }

        if data.len() > self.capacity {
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                label: None,
                size: (data.len() * std::mem::size_of::<T>()) as u64,
                usage: self.usage,
            });
            self.capacity = data.len();
        }

        let src_buffer = buffer_from_slice(
            device,
            wgpu::BufferUsage::COPY_SRC,
            to_u8_slice(data)
        );

        encoder.copy_buffer_to_buffer(
            &src_buffer,
            0,
            &self.buffer,
            0,
            (data.len() * std::mem::size_of::<T>()) as u64,
        );
        self.len = data.len();
    }

    /// Get the underlying buffer
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get the number of stored elements
    pub fn len(&self) -> usize {
        self.len
    }
}

/// A buffer that can contain multiple objects. Every object is of type `T` and can be accessed by a key of type `K`.
pub struct MultiBuffer<K: Hash + Eq + Clone, T: Copy + 'static> {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsage,
    objects: HashMap<K, usize>,
    segments: Vec<MultiBufferSegment>,
    len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<K: Hash + Eq + Clone + std::fmt::Debug, T: Copy + std::fmt::Debug + 'static>
    MultiBuffer<K, T>
{
    /// Create a new `MultiBuffer` with enough capacity for `initial_capacity` elements of type `T`
    pub fn with_capacity(
        device: &wgpu::Device,
        initial_capacity: usize,
        mut usage: wgpu::BufferUsage,
    ) -> Self {
        // We crash on Vulkan if buffer capacity is 0
        assert!(initial_capacity > 0);

        usage |= wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: (initial_capacity * std::mem::size_of::<T>()) as u64,
            usage,
        });
        let segments = vec![MultiBufferSegment {
            free: true,
            pos: 0,
            len: initial_capacity,
        }];

        Self {
            buffer,
            usage,
            objects: HashMap::new(),
            segments,
            len: initial_capacity,
            phantom: std::marker::PhantomData,
        }
    }

    /// Remove object `object` from the buffer
    pub fn remove(&mut self, object: &K) {
        if let Some(start_position) = self.objects.remove(object) {
            let mut segment_position = self
                .segments
                .iter_mut()
                .position(|seg| seg.pos == start_position)
                .expect("logic error!");
            assert_eq!(false, self.segments[segment_position].free, "logic error!");
            self.segments[segment_position].free = true;
            // Merge with the segment before if possible
            if segment_position > 0 {
                if self.segments[segment_position - 1].free {
                    self.segments[segment_position - 1].len += self.segments[segment_position].len;
                    self.segments.remove(segment_position);
                    segment_position -= 1;
                }
            }
            // Merge with the segment after if possible
            if segment_position < self.segments.len() - 1 {
                if self.segments[segment_position + 1].free {
                    self.segments[segment_position].len += self.segments[segment_position + 1].len;
                    self.segments.remove(segment_position + 1);
                }
            }
        }
    }

    /// Update the data for object `object` in the buffer
    ///
    /// # Panics
    /// Will panic if `data` is empty.
    // TODO: handle memory fragmentation
    pub fn update(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        object: K,
        data: &[T],
    ) {
        assert!(data.len() > 0, "cannot add an empty slice to a MultiBuffer");
        // Remove the object if it's already in the buffer
        self.remove(&object);
        // Try to find the position to insert
        let insert_position = self
            .segments
            .iter_mut()
            .position(|seg| seg.len >= data.len() && seg.free);
        let insert_position = insert_position.unwrap_or_else(|| {
            // Reallocate at least twice the size
            self.reallocate(device, encoder, (self.len + data.len()).max(2 * self.len));
            self.segments.len() - 1
        });
        // Copy data into the buffer
        let src_buffer = buffer_from_slice(
            device,
            wgpu::BufferUsage::COPY_SRC,
            to_u8_slice(data)
        );
        encoder.copy_buffer_to_buffer(
            &src_buffer,
            0,
            &self.buffer,
            (self.segments[insert_position].pos * std::mem::size_of::<T>()) as u64,
            (data.len() * std::mem::size_of::<T>()) as u64,
        );
        // Update current segment
        self.segments[insert_position].free = false;
        // Split the segment if necessary
        let extra_length = self.segments[insert_position].len - data.len();
        if extra_length > 0 {
            self.segments[insert_position].len -= extra_length;
            if insert_position < self.segments.len() - 1 && self.segments[insert_position + 1].free
            {
                self.segments[insert_position + 1].pos -= extra_length;
                self.segments[insert_position + 1].len += extra_length;
            } else {
                self.segments.insert(
                    insert_position + 1,
                    MultiBufferSegment {
                        free: true,
                        pos: self.segments[insert_position].pos + data.len(),
                        len: extra_length,
                    },
                );
            }
        }
        // Update the map
        self.objects
            .insert(object.clone(), self.segments[insert_position].pos);
    }

    fn reallocate(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        new_len: usize,
    ) {
        log::debug!(
            "Reallocating MultiBuffer<{}, {}> from length {} to length {}",
            std::any::type_name::<K>(),
            std::any::type_name::<T>(),
            self.len,
            new_len
        );
        // Create new buffer and copy data
        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            mapped_at_creation: false,
            size: (new_len * std::mem::size_of::<T>()) as u64,
            usage: self.usage,
        });
        encoder.copy_buffer_to_buffer(
            &self.buffer,
            0,
            &new_buffer,
            0,
            (self.len * std::mem::size_of::<T>()) as u64,
        );
        self.buffer = new_buffer;
        // Update segments and len
        let last_segment = self.segments.last_mut().expect("logic error!");
        if last_segment.free {
            last_segment.len += new_len - self.len;
        } else {
            self.segments.push(MultiBufferSegment {
                free: true,
                pos: self.len,
                len: new_len - self.len,
            });
        }
        self.len = new_len;
    }

    fn _assert_invariants(&self) {
        assert_eq!(self.segments.first().unwrap().pos, 0);
        assert_eq!(
            self.segments.last().unwrap().pos + self.segments.last().unwrap().len,
            self.len
        );
        for i in 0..(self.segments.len() - 1) {
            assert_eq!(
                self.segments[i].pos + self.segments[i].len,
                self.segments[i + 1].pos
            );
            assert!(!(self.segments[i].free && self.segments[i + 1].free));
        }
        for v in self.objects.values() {
            let segment_position = self
                .segments
                .iter()
                .enumerate()
                .find(|(_, seg)| seg.pos == *v)
                .expect("logic error!")
                .0;
            assert_eq!(false, self.segments[segment_position].free, "logic error!");
        }
        for s in self.segments.iter() {
            let pos_cnt = self.objects.values().filter(|v| **v == s.pos).count();
            if s.free {
                assert_eq!(pos_cnt, 0);
            } else {
                assert_eq!(pos_cnt, 1);
            }
        }
    }

    /// Get the position and the length of object `object` in the buffer
    pub fn get_pos_len(&self, object: &K) -> Option<(usize, usize)> {
        let pos = self.objects.get(object);
        match pos {
            None => None,
            Some(pos) => {
                for seg in self.segments.iter() {
                    if *pos == seg.pos {
                        return Some((seg.pos, seg.len));
                    }
                }
                None
            }
        }
    }

    /// Get the buffer. Please don't modify it.
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    /// Get all the keys, in no particular order
    pub fn keys(&self) -> impl Iterator<Item = K> {
        self.objects.keys().cloned().collect::<Vec<K>>().into_iter()
    }
}

#[derive(Debug, Clone, Copy)]
struct MultiBufferSegment {
    pub free: bool,
    pub pos: usize,
    pub len: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;

    // TODO: test on all backends
    #[test]
    fn test_multi_buffer() {
        use wgpu::*;

        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: None,
            power_preference: PowerPreference::HighPerformance,
        })).unwrap();
        let (device, _queue) = block_on(adapter.request_device(&DeviceDescriptor {
            features: wgpu::Features::empty(),
            limits: Limits::default(),
            shader_validation: true
        }, None))
        .expect("Failed to request device.");
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: None });

        // Create initial buffer
        let mut multi_buffer = MultiBuffer::with_capacity(&device, 10, BufferUsage::empty());

        let seg1 = [2u16, 3u16, 4u16];
        let seg2 = [5u16, 6u16, 7u16, 8u16];
        let seg3 = [9u16];

        // Single insert
        multi_buffer.update(&device, &mut encoder, 0u16, &seg1);
        multi_buffer.remove(&0u16);
        assert_eq!(multi_buffer.get_pos_len(&0), None);

        // Double insert
        multi_buffer.update(&device, &mut encoder, 1u16, &seg2);
        assert_eq!(multi_buffer.get_pos_len(&1), Some((0, 4)));
        multi_buffer.update(&device, &mut encoder, 2u16, &seg2);
        assert_eq!(multi_buffer.get_pos_len(&2), Some((4, 4)));
        multi_buffer.remove(&1u16);
        assert_eq!(multi_buffer.get_pos_len(&1), None);
        assert_eq!(multi_buffer.get_pos_len(&2), Some((4, 4)));

        // Triple insert
        multi_buffer.update(&device, &mut encoder, 0u16, &seg1);
        assert_eq!(multi_buffer.get_pos_len(&0), Some((0, 3)));
        multi_buffer.update(&device, &mut encoder, 1u16, &seg3);
        assert_eq!(multi_buffer.get_pos_len(&1), Some((3, 1)));
        // Now we have 8 items

        // Reallocate
        multi_buffer.update(&device, &mut encoder, 3u16, &seg2);
        assert_eq!(multi_buffer.get_pos_len(&3), Some((8, 4)));
    }
}
