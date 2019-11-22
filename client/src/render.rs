use gfx::traits::Factory;
use gfx_device_gl::Resources as R;

/// Make sure that the given buffer has a sufficient size.
pub fn ensure_buffer_capacity<T>(
    buffer: &mut gfx::handle::Buffer<R, T>,
    min_num: usize,
    factory: &mut gfx_device_gl::Factory,
) -> Result<(), gfx::buffer::CreationError> {
    let info = buffer.get_info().clone();
    let buffer_num = info.size / std::mem::size_of::<T>();
    if buffer_num < min_num {
        let new_buffer = factory.create_buffer(min_num, info.role, info.usage, info.bind)?;
        *buffer = new_buffer;
    }
    Ok(())
}
