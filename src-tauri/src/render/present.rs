//! Draws the preview straight into the editor window (Windows): the compositor's frame goes to
//! a swapchain on the window, under the webview, which is transparent over the preview. Nothing
//! is read back, encoded or sent to the page, and menus and dialogs the page draws over the
//! preview stay on top of it.
//!
//! The page gives up its backgrounds over the preview to make the hole, so the presenter
//! paints them again (the [`Fill`]s) before the frame.
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

const SHADER: &str = include_str!("present.wgsl");
/// Uniform slots: the page's backgrounds, then the frame.
pub const MAX_FILLS: usize = 24;
const SLOT_BYTES: u64 = 256;

/// A rectangle in window pixels: x, y, width, height.
pub type Rect = [f32; 4];

/// A background the page no longer paints over the preview, in window pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub rect: Rect,
    /// Straight (not premultiplied) RGBA, 0 to 1, sRGB encoded as the page gives it.
    pub color: [f32; 4],
    pub radius: f32,
}

/// Where the preview goes in the window.
#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    /// Drawn first, in order (outermost first).
    pub fills: Vec<Fill>,
    /// The frame is fitted inside this, keeping its aspect ratio, and centred.
    pub frame: Rect,
    pub frame_radius: f32,
    /// The part of the window the frame may show in (scrolled or clipped by the page).
    pub clip: Option<Rect>,
}

impl Placement {
    /// Where a `width` x `height` frame lands inside [`Placement::frame`].
    pub fn fitted(&self, width: u32, height: u32) -> Rect {
        let [x, y, w, h] = self.frame;
        if width == 0 || height == 0 || w <= 0.0 || h <= 0.0 {
            return [x, y, 0.0, 0.0];
        }
        let scale = (w / width as f32).min(h / height as f32);
        let (fw, fh) = (width as f32 * scale, height as f32 * scale);
        [x + (w - fw) / 2.0, y + (h - fh) / 2.0, fw, fh]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Item {
    rect: [f32; 4],
    color: [f32; 4],
    surface: [f32; 2],
    radius: f32,
    textured: f32,
}

/// The pipeline that draws a [`Placement`] into a target of one format.
pub struct PresentPipeline {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
}

impl PresentPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aeroedits-present"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let texture = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("aeroedits-present"),
            entries: &[
                texture(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Item>() as u64),
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aeroedits-present-layout"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aeroedits-present-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("aeroedits-present-sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aeroedits-present-items"),
            size: SLOT_BYTES * (MAX_FILLS as u64 + 1),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            pipeline,
            layout,
            sampler,
            uniform,
        }
    }

    /// Records drawing `placement` into `target` (`surface` pixels) with `frame` (its size
    /// `frame_size`). Everything outside the fills and the frame is cleared to black.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        surface: (u32, u32),
        frame: &wgpu::TextureView,
        frame_size: (u32, u32),
        placement: &Placement,
    ) {
        let size = [surface.0 as f32, surface.1 as f32];
        let mut items: Vec<Item> = placement
            .fills
            .iter()
            .take(MAX_FILLS)
            .map(|fill| {
                let [r, g, b, a] = fill.color.map(|c| c.clamp(0.0, 1.0));
                Item {
                    rect: fill.rect,
                    color: [r * a, g * a, b * a, a],
                    surface: size,
                    radius: fill.radius.max(0.0),
                    textured: 0.0,
                }
            })
            .collect();
        let fills = items.len();
        items.push(Item {
            rect: placement.fitted(frame_size.0, frame_size.1),
            color: [0.0; 4],
            surface: size,
            radius: placement.frame_radius.max(0.0),
            textured: 1.0,
        });
        let mut bytes = vec![0u8; SLOT_BYTES as usize * items.len()];
        for (index, item) in items.iter().enumerate() {
            let at = index * SLOT_BYTES as usize;
            bytes[at..at + std::mem::size_of::<Item>()].copy_from_slice(bytemuck::bytes_of(item));
        }
        queue.write_buffer(&self.uniform, 0, &bytes);
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("aeroedits-present-bind"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(frame),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &self.uniform,
                        offset: 0,
                        size: wgpu::BufferSize::new(std::mem::size_of::<Item>() as u64),
                    }),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("aeroedits-present"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        for index in 0..items.len() {
            if index == fills {
                let Some([x, y, w, h]) = scissor(placement.clip, surface) else {
                    break;
                };
                pass.set_scissor_rect(x, y, w, h);
            }
            pass.set_bind_group(0, &bind, &[(index as u64 * SLOT_BYTES) as u32]);
            pass.draw(0..6, 0..1);
        }
    }
}

/// The scissor rectangle for `clip` inside a `surface`, or `None` when nothing of it shows.
fn scissor(clip: Option<Rect>, surface: (u32, u32)) -> Option<[u32; 4]> {
    let [x, y, w, h] = clip.unwrap_or([0.0, 0.0, surface.0 as f32, surface.1 as f32]);
    let x0 = x.floor().clamp(0.0, surface.0 as f32) as u32;
    let y0 = y.floor().clamp(0.0, surface.1 as f32) as u32;
    let x1 = (x + w).ceil().clamp(0.0, surface.0 as f32) as u32;
    let y1 = (y + h).ceil().clamp(0.0, surface.1 as f32) as u32;
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

/// The window a [`WindowPresenter`] draws into.
pub type WindowTarget = Arc<dyn wgpu::WindowHandle>;

/// Lends a shared window to the surface, which wants to own its handle source.
struct SharedWindow(WindowTarget);

impl wgpu::rwh::HasWindowHandle for SharedWindow {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        self.0.window_handle()
    }
}

impl wgpu::rwh::HasDisplayHandle for SharedWindow {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        self.0.display_handle()
    }
}

/// A swapchain on the editor window and the pipeline that draws the preview into it.
pub struct WindowPresenter {
    surface: wgpu::Surface<'static>,
    window: WindowTarget,
    format: wgpu::TextureFormat,
    alpha: wgpu::CompositeAlphaMode,
    configured: Option<(u32, u32)>,
    pipeline: PresentPipeline,
    /// The compositor whose device this was made on.
    pub compositor_id: u64,
}

impl WindowPresenter {
    pub(crate) fn new(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        window: WindowTarget,
        compositor_id: u64,
    ) -> Result<Self, String> {
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Window(Box::new(SharedWindow(
                window.clone(),
            ))))
            .map_err(|e| format!("Could not draw into the window: {e}"))?;
        let caps = surface.get_capabilities(adapter);
        // The compositor's frames hold sRGB-encoded values: a plain UNORM target keeps them.
        let format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
        ]
        .into_iter()
        .find(|f| caps.formats.contains(f))
        .ok_or("The window cannot show 8-bit frames")?;
        let alpha = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            *caps
                .alpha_modes
                .first()
                .ok_or("The window has no alpha mode")?
        };
        Ok(Self {
            surface,
            window,
            format,
            alpha,
            configured: None,
            pipeline: PresentPipeline::new(device, format),
            compositor_id,
        })
    }

    /// The window's client area in pixels.
    pub fn client_size(&self) -> Result<(u32, u32), String> {
        client_size(&self.window)
    }

    /// Draws `placement` with `frame` and shows it. Returns false when there was nothing to
    /// draw into (a minimised window, a swapchain being recreated).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn present(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
        frame: &wgpu::TextureView,
        frame_size: (u32, u32),
        placement: &Placement,
    ) -> Result<bool, String> {
        let size = self.client_size()?;
        if size.0 == 0 || size.1 == 0 {
            queue.submit(Some(encoder.finish()));
            return Ok(false);
        }
        if self.configured != Some(size) {
            self.configure(device, size);
        }
        let texture = match self.surface.get_current_texture() {
            Ok(texture) => texture,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.configure(device, size);
                match self.surface.get_current_texture() {
                    Ok(texture) => texture,
                    Err(e) => {
                        queue.submit(Some(encoder.finish()));
                        return Err(format!("The window's swapchain failed: {e}"));
                    }
                }
            }
            Err(wgpu::SurfaceError::Timeout) => {
                queue.submit(Some(encoder.finish()));
                return Ok(false);
            }
            Err(e) => {
                queue.submit(Some(encoder.finish()));
                return Err(format!("The window's swapchain failed: {e}"));
            }
        };
        let view = texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.pipeline.draw(
            device,
            queue,
            &mut encoder,
            &view,
            size,
            frame,
            frame_size,
            placement,
        );
        queue.submit(Some(encoder.finish()));
        texture.present();
        Ok(true)
    }

    fn configure(&mut self, device: &wgpu::Device, (width, height): (u32, u32)) {
        self.surface.configure(
            device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: self.alpha,
                view_formats: vec![],
            },
        );
        self.configured = Some((width, height));
    }
}

#[cfg(windows)]
#[link(name = "user32")]
extern "system" {
    fn GetClientRect(hwnd: isize, rect: *mut [i32; 4]) -> i32;
}

/// The size of a window's client area in pixels.
#[cfg(windows)]
fn client_size(window: &WindowTarget) -> Result<(u32, u32), String> {
    use wgpu::rwh::HasWindowHandle;
    let handle = window
        .window_handle()
        .map_err(|e| format!("The window has no handle: {e}"))?;
    let wgpu::rwh::RawWindowHandle::Win32(win32) = handle.as_raw() else {
        return Err("Not a Windows window".into());
    };
    let mut rect = [0i32; 4];
    // SAFETY: the handle is a live window (the presenter holds it) and `rect` is a RECT.
    if unsafe { GetClientRect(win32.hwnd.get(), &mut rect) } == 0 {
        return Err("Could not measure the window".into());
    }
    let [left, top, right, bottom] = rect;
    Ok(((right - left).max(0) as u32, (bottom - top).max(0) as u32))
}

#[cfg(not(windows))]
fn client_size(_window: &WindowTarget) -> Result<(u32, u32), String> {
    Err("The window preview is only drawn this way on Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement() -> Placement {
        Placement {
            fills: vec![],
            frame: [100.0, 50.0, 400.0, 400.0],
            frame_radius: 0.0,
            clip: None,
        }
    }

    #[test]
    fn frames_fit_inside_their_box_centred() {
        let p = placement();
        assert_eq!(p.fitted(1920, 1080), [100.0, 137.5, 400.0, 225.0]);
        assert_eq!(p.fitted(1080, 1920), [187.5, 50.0, 225.0, 400.0]);
        assert_eq!(p.fitted(0, 10), [100.0, 50.0, 0.0, 0.0]);
    }

    /// Draws a placement into an offscreen target and reads it back: a fill, a half-transparent
    /// fill over it, and a red 2:1 frame letterboxed in a square box with rounded corners.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter; run with --ignored on a machine that has one"
    )]
    fn gpu_placements_draw_fills_then_the_frame_fitted() {
        let compositor = crate::render::Compositor::new().unwrap();
        let (device, queue) = (&compositor.device, &compositor.queue);
        let texture = |width, height, usage| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Bgra8Unorm,
                usage,
                view_formats: &[],
            })
        };
        // The frame: 64x32 of pure red.
        let frame = texture(
            64,
            32,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        let red: Vec<u8> = [0u8, 0, 255, 255].repeat(64 * 32);
        queue.write_texture(
            frame.as_image_copy(),
            &red,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(64 * 4),
                rows_per_image: Some(32),
            },
            wgpu::Extent3d {
                width: 64,
                height: 32,
                depth_or_array_layers: 1,
            },
        );
        let target = texture(
            128,
            128,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        );
        let placement = Placement {
            fills: vec![
                Fill {
                    rect: [0.0, 0.0, 128.0, 128.0],
                    color: [0.0, 0.0, 1.0, 1.0],
                    radius: 0.0,
                },
                Fill {
                    rect: [0.0, 0.0, 64.0, 128.0],
                    color: [1.0, 1.0, 1.0, 0.5],
                    radius: 0.0,
                },
            ],
            frame: [16.0, 16.0, 96.0, 96.0],
            frame_radius: 8.0,
            clip: None,
        };
        let pipeline = PresentPipeline::new(device, wgpu::TextureFormat::Bgra8Unorm);
        let mut encoder = device.create_command_encoder(&Default::default());
        pipeline.draw(
            device,
            queue,
            &mut encoder,
            &target.create_view(&Default::default()),
            (128, 128),
            &frame.create_view(&Default::default()),
            (64, 32),
            &placement,
        );
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 512 * 128,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            target.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(512),
                    rows_per_image: Some(128),
                },
            },
            wgpu::Extent3d {
                width: 128,
                height: 128,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(Some(encoder.finish()));
        readback.slice(..).map_async(wgpu::MapMode::Read, |_| ());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let data = readback.slice(..).get_mapped_range();
        // BGRA at (x, y).
        let px = |x: usize, y: usize| -> [u8; 3] {
            let at = y * 512 + x * 4;
            [data[at], data[at + 1], data[at + 2]]
        };
        let near =
            |got: [u8; 3], want: [u8; 3]| got.iter().zip(want).all(|(g, w)| g.abs_diff(w) <= 3);
        // Blue everywhere, half white over the left half, the red frame fitted to 96x48
        // in the middle (rows 40 to 88), and the fills showing above and below it.
        assert!(near(px(100, 5), [255, 0, 0]), "{:?}", px(100, 5));
        assert!(near(px(10, 5), [255, 128, 128]), "{:?}", px(10, 5));
        assert!(near(px(64, 64), [0, 0, 255]), "{:?}", px(64, 64));
        assert!(near(px(100, 30), [255, 0, 0]), "{:?}", px(100, 30));
        assert!(near(px(100, 86), [0, 0, 255]), "{:?}", px(100, 86));
        // The frame's rounded corner shows the fill behind it.
        assert!(near(px(16, 40), [255, 128, 128]), "{:?}", px(16, 40));
    }

    #[test]
    fn scissors_stay_inside_the_window() {
        assert_eq!(scissor(None, (800, 600)), Some([0, 0, 800, 600]));
        assert_eq!(
            scissor(Some([-10.0, 20.5, 100.0, 1000.0]), (800, 600)),
            Some([0, 20, 90, 580])
        );
        assert_eq!(scissor(Some([900.0, 0.0, 10.0, 10.0]), (800, 600)), None);
    }
}
