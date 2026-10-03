// Adapted from gpui-fusion crates/gpui_wgpu/src/offscreen_renderer.rs,
// revision 27d0daf84e4956f93aac222985e91181ff3d43b5, Apache-2.0.
// Copyright 2022 - 2025 Zed Industries, Inc. See licenses/third-party/gpui_wgpu-0.1.0.
// Modified by Audorchid contributors, 2026: use an UNORM output target. GPUI's
// color atlas and UI shaders carry display-encoded colors; an sRGB target would
// apply a second transfer curve and wash out both images and primitives.
use anyhow::{Context, Result, ensure};
use gpui::{DevicePixels, PlatformAtlas, Scene, Size};
use gpui_wgpu::{WgpuContext, WgpuExternalRendererConfig, WgpuRenderer, wgpu};
use std::sync::{Arc, mpsc};

pub(crate) struct GpuRenderer {
    context: WgpuContext,
    renderer: WgpuRenderer,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
    size: Size<DevicePixels>,
    stride: u32,
}
impl GpuRenderer {
    pub fn new(size: Size<DevicePixels>) -> Result<Self> {
        let context = WgpuContext::new_headless()?;
        let renderer = WgpuRenderer::new_external(
            &context,
            WgpuExternalRendererConfig {
                size,
                format: wgpu::TextureFormat::Rgba8Unorm,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                target_usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
            },
        )?;
        let (texture, view, readback, stride) = targets(&context, size);
        Ok(Self {
            context,
            renderer,
            texture,
            view,
            readback,
            size,
            stride,
        })
    }
    pub fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.renderer.sprite_atlas().clone()
    }
    pub fn resize(&mut self, size: Size<DevicePixels>) {
        if size == self.size {
            return;
        }
        self.renderer.update_drawable_size(size);
        (self.texture, self.view, self.readback, self.stride) = targets(&self.context, size);
        self.size = size;
    }
    pub fn render_rgba(&mut self, scene: &Scene) -> Result<Vec<u8>> {
        ensure!(
            self.renderer
                .draw_external(scene, &self.texture, &self.view, wgpu::Color::BLACK),
            "GPUI renderer requested a larger instance buffer; retry the frame"
        );
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("audorchid.readback"),
                });
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.stride),
                    rows_per_image: Some(self.size.height.0 as u32),
                },
            },
            wgpu::Extent3d {
                width: self.size.width.0 as u32,
                height: self.size.height.0 as u32,
                depth_or_array_layers: 1,
            },
        );
        self.context.queue.submit([encoder.finish()]);
        let (tx, rx) = mpsc::sync_channel(1);
        self.readback
            .map_async(wgpu::MapMode::Read, .., move |result| {
                let _ = tx.send(result);
            });
        self.context.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        })?;
        rx.recv().context("GPU readback callback dropped")??;
        let mapped = self.readback.get_mapped_range(..)?;
        let row = self.size.width.0 as usize * 4;
        let mut rgba = Vec::with_capacity(row * self.size.height.0 as usize);
        for source in mapped
            .chunks_exact(self.stride as usize)
            .take(self.size.height.0 as usize)
        {
            rgba.extend_from_slice(&source[..row]);
        }
        drop(mapped);
        self.readback.unmap();
        Ok(rgba)
    }
}
fn targets(
    context: &WgpuContext,
    size: Size<DevicePixels>,
) -> (wgpu::Texture, wgpu::TextureView, wgpu::Buffer, u32) {
    let width = size.width.0 as u32;
    let height = size.height.0 as u32;
    let texture = context.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("audorchid.offscreen"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let stride = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let readback = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("audorchid.readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    (texture, view, readback, stride)
}
