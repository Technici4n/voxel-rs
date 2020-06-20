//! Helpers for renderer passes

use crate::window::WindowBuffers;

/// Create an attachment for the depth buffer that doesn't clear it.
pub fn create_default_depth_stencil_attachment(
    depth_buffer: &wgpu::TextureView,
) -> wgpu_types::RenderPassDepthStencilAttachmentDescriptorBase<&wgpu::TextureView> {
    wgpu_types::RenderPassDepthStencilAttachmentDescriptorBase {
        attachment: depth_buffer,
        depth_load_op: wgpu::LoadOp::Load,
        depth_store_op: wgpu::StoreOp::Store,
        clear_depth: 0.0, // TODO: use debugging depth ?
        stencil_load_op: wgpu::LoadOp::Load,
        stencil_store_op: wgpu::StoreOp::Store,
        clear_stencil: 0,
    }
}

/// Create a render pass that renders to the multisampled frame buffer without resolving and without clearing.
pub fn create_default_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    buffers: WindowBuffers<'a>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: buffers.multisampled_texture_buffer,
            resolve_target: None,
            load_op: wgpu::LoadOp::Load,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color::GREEN, // TODO: use debugging color ?
        }],
        depth_stencil_attachment: Some(create_default_depth_stencil_attachment(
            buffers.depth_buffer,
        )),
    })
}

/// Encode a render pass to resolve the multisampled frame buffer to the window frame buffer
pub fn encode_resolve_render_pass<'a>(encoder: &mut wgpu::CommandEncoder, buffers: WindowBuffers) {
    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: buffers.multisampled_texture_buffer,
            resolve_target: Some(buffers.texture_buffer),
            load_op: wgpu::LoadOp::Load,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color::GREEN, // TODO: use debugging color ?
        }],
        depth_stencil_attachment: None,
    });
}

fn create_clear_color_attachment(
    buffers: WindowBuffers,
) -> [wgpu::RenderPassColorAttachmentDescriptor; 1] {
    [wgpu::RenderPassColorAttachmentDescriptor {
        attachment: buffers.multisampled_texture_buffer,
        resolve_target: None,
        load_op: wgpu::LoadOp::Clear,
        store_op: wgpu::StoreOp::Store,
        clear_color: crate::window::CLEAR_COLOR,
    }]
}

fn create_clear_depth_attachment(
    buffers: WindowBuffers,
) -> wgpu_types::RenderPassDepthStencilAttachmentDescriptorBase<&wgpu::TextureView> {
    wgpu_types::RenderPassDepthStencilAttachmentDescriptorBase {
        attachment: buffers.depth_buffer,
        depth_load_op: wgpu::LoadOp::Clear,
        depth_store_op: wgpu::StoreOp::Store,
        clear_depth: crate::window::CLEAR_DEPTH,
        stencil_load_op: wgpu::LoadOp::Load,
        stencil_store_op: wgpu::StoreOp::Store,
        clear_stencil: 0,
    }
}

/// Clear the multisampled color buffer and the depth buffer
pub fn clear_color_and_depth(encoder: &mut wgpu::CommandEncoder, buffers: WindowBuffers) {
    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &create_clear_color_attachment(buffers),
        depth_stencil_attachment: Some(create_clear_depth_attachment(buffers)),
    });
}

/// Clear the depth buffer
pub fn clear_depth(encoder: &mut wgpu::CommandEncoder, buffers: WindowBuffers) {
    let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[],
        depth_stencil_attachment: Some(create_clear_depth_attachment(buffers)),
    });
}

/// Convert a vector to a buffer compatible slice of u8
pub fn to_u8_slice<T: Copy>(v: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len() * std::mem::size_of::<T>()) }
}

/// Helper to create a buffer from an existing slice.
pub fn buffer_from_slice(device: &wgpu::Device, usage: wgpu::BufferUsage, data: &[u8]) -> wgpu::Buffer {
    let buffer_mapped = device.create_buffer_mapped(&wgpu::BufferDescriptor {
        label: None,
        size: data.len() as u64,
        usage
    });
    buffer_mapped.data.copy_from_slice(&data);
    buffer_mapped.finish()
}
