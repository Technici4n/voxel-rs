//! Helpers for renderer passes

use wgpu::util::{BufferInitDescriptor, DeviceExt};
use crate::window::WindowBuffers;

/// Create an attachment for the depth buffer that doesn't clear it.
pub fn create_default_depth_stencil_attachment(
    depth_buffer: &wgpu::TextureView,
) -> wgpu::RenderPassDepthStencilAttachmentDescriptor {
    wgpu::RenderPassDepthStencilAttachmentDescriptor {
        attachment: depth_buffer,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: true
        }),
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: true
        })
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
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true
            },
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
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: true
            },
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
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(crate::window::CLEAR_COLOR),
            store: true
        },
    }]
}

fn create_clear_depth_attachment(
    buffers: WindowBuffers,
) -> wgpu::RenderPassDepthStencilAttachmentDescriptor {
    wgpu::RenderPassDepthStencilAttachmentDescriptor {
        attachment: buffers.depth_buffer,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(crate::window::CLEAR_DEPTH),
            store: true
        }),
        stencil_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: true
        })
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
    device.create_buffer_init(&BufferInitDescriptor {
        label: None,
        usage,
        contents: &data
    })
}
