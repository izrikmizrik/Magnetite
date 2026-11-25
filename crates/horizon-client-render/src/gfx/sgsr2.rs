//! SGSR 2 (`sgsr2.wgsl`, Snapdragon Game Super Resolution 2; BSD-3-Clause, THIRD_PARTY_NOTICES.md):
//! a temporal upscaler, the two-pass fragment variant. The world's jittered frame (TAA's jitter)
//! is turned into the camera's motion and a depth-clip weight at its own size, then upsampled
//! into a history at the output's size, which is what shows. At Native it is temporal
//! anti-aliasing. Built when the setting first asks for it, under error scopes: a device that
//! can't build it gets the reason in the settings (`graphics::antialiasing_failed`).

use super::smaa::caught;
use super::world_post::UpscaleFrame;

/// The history's and the motion's format: 8 bits would band as frames accumulate.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// `sgsr2.wgsl`'s `Params`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    clip_to_prev_clip: [[f32; 4]; 4],
    render_size: [f32; 2],
    output_size: [f32; 2],
    render_size_rcp: [f32; 2],
    output_size_rcp: [f32; 2],
    jitter_offset: [f32; 2],
    scale_ratio: [f32; 2],
    camera_fov_angle_hor: f32,
    min_lerp_contribution: f32,
    reset: f32,
    pad: f32,
}

pub(super) struct Sgsr2 {
    half: bool,
    convert_layout: wgpu::BindGroupLayout,
    upscale_layout: wgpu::BindGroupLayout,
    convert: wgpu::RenderPipeline,
    upscale: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    point: wgpu::Sampler,
    linear: wgpu::Sampler,
    targets: Option<Targets>,
    /// The last frame's reprojection: a camera that hasn't moved lets the history hold where
    /// it falls outside the box (`min_lerp_contribution`).
    last_reproject: std::cell::Cell<Option<[[f32; 4]; 4]>>,
}

/// The motion at the world's size and the two histories at the output's, written in turn:
/// `upscale[i]` writes `history[i]` from `history[1 - i]`; `show[i]` shows `history[i]`.
struct Targets {
    render: [u32; 2],
    output: [u32; 2],
    motion: wgpu::TextureView,
    history: [wgpu::TextureView; 2],
    convert: wgpu::BindGroup,
    upscale: [wgpu::BindGroup; 2],
    show: [wgpu::BindGroup; 2],
}

impl Sgsr2 {
    /// The pipelines, or the device's error.
    pub(super) fn new(device: &wgpu::Device, half: bool) -> Result<Self, String> {
        caught(device, || Self::build(device, half))
    }

    fn build(device: &wgpu::Device, half: bool) -> Self {
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let texture = |binding, filterable| {
            entry(
                binding,
                wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
            )
        };
        let uniform = entry(
            0,
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        );
        let convert_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SGSR 2 convert"),
            entries: &[
                uniform,
                // The depth as an unfilterable float, as TAA reads it: the GLSL backend can
                // gather it, which it can't from a depth binding.
                texture(1, false),
                entry(
                    2,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                ),
            ],
        });
        let upscale_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SGSR 2 upscale"),
            entries: &[
                uniform,
                texture(3, true),
                texture(4, true),
                texture(5, true),
                entry(
                    6,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
            ],
        });
        let (convert, upscale) = Self::pipelines(device, &convert_layout, &upscale_layout, half);
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SGSR 2"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = |label, filter| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                mag_filter: filter,
                min_filter: filter,
                ..Default::default()
            })
        };
        Self {
            half,
            convert_layout,
            upscale_layout,
            convert,
            upscale,
            uniform,
            point: sampler("SGSR 2 point", wgpu::FilterMode::Nearest),
            linear: sampler("SGSR 2 linear", wgpu::FilterMode::Linear),
            targets: None,
            last_reproject: std::cell::Cell::new(None),
        }
    }

    fn pipelines(
        device: &wgpu::Device,
        convert_layout: &wgpu::BindGroupLayout,
        upscale_layout: &wgpu::BindGroupLayout,
        half: bool,
    ) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Horizon SGSR 2"),
            source: wgpu::ShaderSource::Wgsl(
                super::half::shader(device, half, include_str!("sgsr2.wgsl")).into(),
            ),
        });
        let pipeline = |label, layout: &wgpu::BindGroupLayout, entry| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(layout)],
                ..Default::default()
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_sgsr2"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        (
            pipeline("SGSR 2 convert", convert_layout, "fs_convert"),
            pipeline("SGSR 2 upscale", upscale_layout, "fs_upscale"),
        )
    }

    /// The precision: the passes are built again when it changed.
    pub(super) fn set_half(&mut self, device: &wgpu::Device, half: bool) -> Result<(), String> {
        if half == self.half {
            return Ok(());
        }
        let (convert, upscale) = caught(device, || {
            Self::pipelines(device, &self.convert_layout, &self.upscale_layout, half)
        })?;
        (self.convert, self.upscale, self.half) = (convert, upscale, half);
        Ok(())
    }

    /// The targets for the world's `color` and `depth` at `render` and the output at `output`;
    /// `bind` makes the bindings that show a history. Made again when either size changed or
    /// the world's target was (`forget`): `Ok(true)`, and the new history holds nothing yet.
    pub(super) fn fit(
        &mut self,
        device: &wgpu::Device,
        (color, depth): (&wgpu::TextureView, &wgpu::TextureView),
        (render, output): ([u32; 2], [u32; 2]),
        bind: impl Fn(&wgpu::TextureView) -> wgpu::BindGroup,
    ) -> Result<bool, String> {
        if self
            .targets
            .as_ref()
            .is_some_and(|t| t.render == render && t.output == output)
        {
            return Ok(false);
        }
        self.targets = None;
        let targets = caught(device, || {
            self.targets_for(device, (color, depth), (render, output), &bind)
        })?;
        self.targets = Some(targets);
        Ok(true)
    }

    /// The world's target is new: the bindings that read the old one go.
    pub(super) fn forget(&mut self) {
        self.targets = None;
    }

    fn targets_for(
        &self,
        device: &wgpu::Device,
        (color, depth): (&wgpu::TextureView, &wgpu::TextureView),
        (render, output): ([u32; 2], [u32; 2]),
        bind: &impl Fn(&wgpu::TextureView) -> wgpu::BindGroup,
    ) -> Targets {
        let target = |label, size: [u32; 2]| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMAT,
                    // COPY_SRC: a native pack's output reads its copy of the history.
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let motion = target("SGSR 2 motion", render);
        let history = [0, 1].map(|_| target("SGSR 2 history", output));
        let view = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let convert = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SGSR 2 convert"),
            layout: &self.convert_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                view(1, depth),
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.point),
                },
            ],
        });
        let upscale = [0, 1].map(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("SGSR 2 upscale"),
                layout: &self.upscale_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform.as_entire_binding(),
                    },
                    view(3, color),
                    view(4, &motion),
                    view(5, &history[1 - i]),
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::Sampler(&self.linear),
                    },
                ],
            })
        });
        let show = [0, 1].map(|i| bind(&history[i]));
        Targets {
            render,
            output,
            motion,
            history,
            convert,
            upscale,
            show,
        }
    }

    /// The history this frame writes, for a native pack's output to read (`resolve_taa`).
    pub(super) fn history(&self, index: u32) -> Option<&wgpu::TextureView> {
        Some(&self.targets.as_ref()?.history[(index % 2) as usize])
    }

    /// The two passes for `frame`, and the bindings that show the history they wrote.
    /// `fov_hor`: tan(vertical fov / 2) * width / height.
    pub(super) fn resolve(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &UpscaleFrame<'_>,
        fov_hor: f32,
        mut timing: Option<&mut super::pack_timing::PassTiming>,
    ) -> Option<&wgpu::BindGroup> {
        let targets = self.targets.as_ref()?;
        let [rw, rh] = frame.render_size.map(|v| v as f32);
        let [ow, oh] = frame.output_size.map(|v| v as f32);
        let still = self.last_reproject.replace(Some(frame.reproject)) == Some(frame.reproject);
        let params = Params {
            clip_to_prev_clip: frame.reproject,
            render_size: [rw, rh],
            output_size: [ow, oh],
            render_size_rcp: [1.0 / rw, 1.0 / rh],
            output_size_rcp: [1.0 / ow, 1.0 / oh],
            jitter_offset: frame.jitter_pixels,
            scale_ratio: [
                (ow / rw).min(1.99),
                ((ow / rw) * (oh / rh)).powi(3).min(20.0),
            ],
            camera_fov_angle_hor: fov_hor,
            min_lerp_contribution: if still { 0.3 } else { 0.0 },
            reset: if frame.reset { 1.0 } else { 0.0 },
            pad: 0.0,
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&params));
        let i = (frame.index % 2) as usize;
        let mut pass = |label, view: &wgpu::TextureView, pipeline, group, timed: bool| {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let writes = if timed {
                timing.as_deref_mut().and_then(|t| t.writes("SGSR 2"))
            } else {
                None
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                timestamp_writes: writes,
                color_attachments: &attachments,
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        };
        pass(
            "SGSR 2 convert",
            &targets.motion,
            &self.convert,
            &targets.convert,
            false,
        );
        pass(
            "SGSR 2 upscale",
            &targets.history[i],
            &self.upscale,
            &targets.upscale[i],
            true,
        );
        Some(&targets.show[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::smaa::tests::{device, float_texture, read};

    /// A still picture, jittered as TAA jitters it: SGSR 2 settles on it at Native and at twice
    /// the size, the flat parts as drawn (light above paper white included), nothing outside
    /// the picture's range or half float's, and each pixel's alpha its own.
    #[test]
    fn sgsr2_settles_on_a_still_picture_at_native_and_upscaled() {
        let _gpu = crate::gfx::test_gpu::lock();
        let Some((device, queue, has_half)) = device() else {
            eprintln!("SGSR 2 GPU test skipped: no device");
            return;
        };
        const N: u32 = 24;
        let (bright, dark) = (3.0, half::f16::from_f32(0.2).to_f32());
        let color = float_texture(&device, &queue, [N, N], |x, _| {
            let c = if x < N / 2 { bright } else { dark };
            [c, c, c, 0.25]
        });
        // A wall halfway into the scene, reversed depth, as an unfilterable float.
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: N,
                height: N,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            depth.as_image_copy(),
            &bytemuck::cast_slice(&vec![0.5f32; (N * N) as usize]),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(N * 4),
                rows_per_image: None,
            },
            depth.size(),
        );
        let (color_view, depth_view) = (
            color.create_view(&Default::default()),
            depth.create_view(&Default::default()),
        );
        for half in [false, true].into_iter().filter(|h| !h || has_half) {
            for scale in [1, 2] {
                let output = [N * scale; 2];
                let post = super::super::world_post::Post::new(&device, FORMAT, half);
                let mut sgsr2 = Sgsr2::new(&device, half)
                    .unwrap_or_else(|e| panic!("SGSR 2 (half: {half}): {e}"));
                let bind = |view: &wgpu::TextureView| post.bind(&device, view);
                sgsr2
                    .fit(&device, (&color_view, &depth_view), ([N, N], output), bind)
                    .unwrap();
                let mut last = 0;
                for index in 0..24u32 {
                    let i = index % 8 + 1;
                    let jitter = [halton(i, 2) - 0.5, halton(i, 3) - 0.5];
                    let frame = UpscaleFrame {
                        color: &color_view,
                        depth: &depth_view,
                        render_size: [N, N],
                        output_size: output,
                        jitter_clip: [jitter[0] * 2.0 / N as f32, -jitter[1] * 2.0 / N as f32],
                        jitter_pixels: jitter,
                        reproject: glam::Mat4::IDENTITY.to_cols_array_2d(),
                        reset: index == 0,
                        index,
                    };
                    let mut encoder = device.create_command_encoder(&Default::default());
                    assert!(
                        sgsr2
                            .resolve(&queue, &mut encoder, &frame, 1.0, None)
                            .is_some()
                    );
                    queue.submit([encoder.finish()]);
                    last = index;
                }
                let history = sgsr2.history(last).unwrap().texture().clone();
                let encoder = device.create_command_encoder(&Default::default());
                let pixels = read(&device, &queue, encoder, &history);
                let width = output[0] as usize;
                let at = |x: usize, y: usize| pixels[y * width + x];
                let what = format!("half: {half}, scale {scale}");
                for p in &pixels {
                    assert!(p.iter().all(|c| c.is_finite()), "{what}: {p:?}");
                    assert!(
                        (dark - 0.01..=bright + 0.01).contains(&p[0]),
                        "{what}: {p:?}"
                    );
                    assert_eq!(p[3], 0.25, "{what}: the alpha is the pixel's own");
                }
                let near = |a: f32, b: f32| (a - b).abs() <= 0.01 * b.max(1.0);
                assert!(
                    near(at(2, width / 2)[0], bright),
                    "{what}: {:?}",
                    at(2, width / 2)
                );
                assert!(
                    near(at(width - 3, width / 2)[0], dark),
                    "{what}: {:?}",
                    at(width - 3, width / 2)
                );
            }
        }
    }

    fn halton(mut i: u32, base: u32) -> f32 {
        let (mut f, mut r) = (1.0, 0.0);
        while i > 0 {
            f /= base as f32;
            r += f * (i % base) as f32;
            i /= base;
        }
        r
    }
}
