//! Arm ASR (`arm_asr.wgsl`, Arm Accuracy Super Resolution; MIT, after AMD FidelityFX Super
//! Resolution 2; THIRD_PARTY_NOTICES.md): a temporal upscaler in TAA's place, on TAA's jitter,
//! with Arm's Ultra Performance shader options. Four passes a frame: reconstruct and dilate,
//! depth clip, locks (compute), accumulate; the history, the locks and the dilated motion are
//! kept in pairs written in turn. Built when the setting first asks for it, under error scopes:
//! a device that can't build it gets the reason in the settings (`graphics::antialiasing_failed`).

use super::smaa::caught;
use super::world_post::UpscaleFrame;

const HISTORY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const LOCK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
const MOTION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
const LUMA_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;
const MASKS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg16Float;

/// `arm_asr.wgsl`'s `Params`.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    clip_to_prev_clip: [[f32; 4]; 4],
    device_to_view_depth: [f32; 4],
    render_size: [f32; 2],
    display_size: [f32; 2],
    jitter: [f32; 2],
    downscale_factor: [f32; 2],
    jitter_sequence_length: f32,
    reset: f32,
    pad: [f32; 2],
}

/// The camera ASR's depth clip reads: the vertical field, the aspect and the planes of the
/// world's reversed, finite projection.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Camera {
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    /// ASR's `DeviceToViewSpaceTransformFactors` for inverted, finite depth
    /// (`setupDeviceDepthToViewSpaceDepthParams`).
    fn device_to_view_depth(self) -> [f32; 4] {
        // Inverted: the far plane takes the near's place.
        let (min, max) = (self.far, self.near);
        let q = max / (min - max);
        let tan_half = (self.fov_y * 0.5).tan();
        [-q, q * min, tan_half * self.aspect, tan_half]
    }
}

pub(super) struct Arm {
    layouts: [wgpu::BindGroupLayout; 4],
    reconstruct: wgpu::RenderPipeline,
    depth_clip: wgpu::RenderPipeline,
    lock: wgpu::ComputePipeline,
    accumulate: wgpu::RenderPipeline,
    uniform: wgpu::Buffer,
    linear: wgpu::Sampler,
    targets: Option<Targets>,
}

/// One world target's and one output size's resources: `[i]` is written on frames whose index
/// is `i` mod 2, from `[1 - i]`.
struct Targets {
    render: [u32; 2],
    output: [u32; 2],
    dilated_depth: wgpu::TextureView,
    dilated_motion: [wgpu::TextureView; 2],
    lock_luma: wgpu::TextureView,
    masks: wgpu::TextureView,
    history: [wgpu::TextureView; 2],
    lock_status: [wgpu::TextureView; 2],
    color: wgpu::TextureView,
    reconstruct: wgpu::BindGroup,
    depth_clip: [wgpu::BindGroup; 2],
    lock: wgpu::BindGroup,
    accumulate: [wgpu::BindGroup; 2],
    show: wgpu::BindGroup,
}

impl Arm {
    /// The pipelines, or the device's error.
    pub(super) fn new(device: &wgpu::Device) -> Result<Self, String> {
        caught(device, || Self::build(device))
    }

    fn build(device: &wgpu::Device) -> Self {
        let stages = wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE;
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: stages,
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
        let storage = |binding| {
            entry(
                binding,
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
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
        let sampler = entry(
            3,
            wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        );
        let layout = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            })
        };
        // The depth reads as an unfilterable float, as TAA's does: the GLSL backend can't load a
        // depth binding. The dilated depth is 32-bit float, unfilterable without a feature.
        let layouts = [
            layout(
                "Arm ASR reconstruct",
                &[uniform, texture(1, true), texture(2, false), storage(4)],
            ),
            layout(
                "Arm ASR depth clip",
                &[
                    uniform,
                    texture(2, false),
                    sampler,
                    storage(4),
                    texture(5, false),
                    texture(6, false),
                    texture(7, true),
                ],
            ),
            layout(
                "Arm ASR lock",
                &[uniform, storage(4), texture(8, false), storage(9)],
            ),
            layout(
                "Arm ASR accumulate",
                &[
                    uniform,
                    texture(1, true),
                    sampler,
                    texture(6, false),
                    storage(9),
                    texture(10, true),
                    texture(11, true),
                    texture(12, true),
                ],
            ),
        ];
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Horizon Arm ASR"),
            source: wgpu::ShaderSource::Wgsl(include_str!("arm_asr.wgsl").into()),
        });
        let pipeline_layout = |layout: &wgpu::BindGroupLayout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Arm ASR"),
                bind_group_layouts: &[Some(layout)],
                ..Default::default()
            })
        };
        let render = |label, layout, entry, formats: &[wgpu::TextureFormat]| {
            let targets: Vec<_> = formats
                .iter()
                .map(|&format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                })
                .collect();
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout(layout)),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_asr"),
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
                    targets: &targets,
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let reconstruct = render(
            "Arm ASR reconstruct",
            &layouts[0],
            "fs_reconstruct",
            &[DEPTH_FORMAT, MOTION_FORMAT, LUMA_FORMAT],
        );
        let depth_clip = render(
            "Arm ASR depth clip",
            &layouts[1],
            "fs_depth_clip",
            &[MASKS_FORMAT],
        );
        let accumulate = render(
            "Arm ASR accumulate",
            &layouts[3],
            "fs_accumulate",
            &[HISTORY_FORMAT, LOCK_FORMAT, HISTORY_FORMAT],
        );
        let lock = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Arm ASR lock"),
            layout: Some(&pipeline_layout(&layouts[2])),
            module: &module,
            entry_point: Some("cs_lock"),
            compilation_options: Default::default(),
            cache: None,
        });
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Arm ASR"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Arm ASR"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            layouts,
            reconstruct,
            depth_clip,
            lock,
            accumulate,
            uniform,
            linear,
            targets: None,
        }
    }

    /// The resources for the world's `color` and `depth` at `render` and the output at
    /// `output`; `bind` makes the bindings that show the output. `Ok(true)` when they were made
    /// again (a size changed, or the world's target did: `forget`): their history is empty.
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
        let texture = |label, size: [u32; 2], format| {
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
                    format,
                    // COPY_SRC: a native pack's output reads its copy of the picture.
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let buffer = |label, size: [u32; 2]| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: u64::from(size[0]) * u64::from(size[1]) * 4,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let dilated_depth = texture("Arm ASR dilated depth", render, DEPTH_FORMAT);
        let dilated_motion =
            [0, 1].map(|_| texture("Arm ASR dilated motion", render, MOTION_FORMAT));
        let lock_luma = texture("Arm ASR lock luma", render, LUMA_FORMAT);
        let masks = texture("Arm ASR masks", render, MASKS_FORMAT);
        let history = [0, 1].map(|_| texture("Arm ASR history", output, HISTORY_FORMAT));
        let lock_status = [0, 1].map(|_| texture("Arm ASR lock status", output, LOCK_FORMAT));
        let shown = texture("Arm ASR output", output, HISTORY_FORMAT);
        let reconstructed = buffer("Arm ASR reconstructed depth", render);
        let new_locks = buffer("Arm ASR new locks", output);
        let view = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let uniform = wgpu::BindGroupEntry {
            binding: 0,
            resource: self.uniform.as_entire_binding(),
        };
        let sampler = wgpu::BindGroupEntry {
            binding: 3,
            resource: wgpu::BindingResource::Sampler(&self.linear),
        };
        let reconstructed_entry = wgpu::BindGroupEntry {
            binding: 4,
            resource: reconstructed.as_entire_binding(),
        };
        let locks_entry = wgpu::BindGroupEntry {
            binding: 9,
            resource: new_locks.as_entire_binding(),
        };
        let group = |label, layout, entries: &[wgpu::BindGroupEntry]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries,
            })
        };
        let reconstruct = group(
            "Arm ASR reconstruct",
            &self.layouts[0],
            &[
                uniform.clone(),
                view(1, color),
                view(2, depth),
                reconstructed_entry.clone(),
            ],
        );
        let depth_clip = [0, 1].map(|i| {
            group(
                "Arm ASR depth clip",
                &self.layouts[1],
                &[
                    uniform.clone(),
                    view(2, depth),
                    sampler.clone(),
                    reconstructed_entry.clone(),
                    view(5, &dilated_depth),
                    view(6, &dilated_motion[i]),
                    view(7, &dilated_motion[1 - i]),
                ],
            )
        });
        let lock = group(
            "Arm ASR lock",
            &self.layouts[2],
            &[
                uniform.clone(),
                reconstructed_entry,
                view(8, &lock_luma),
                locks_entry.clone(),
            ],
        );
        let accumulate = [0, 1].map(|i| {
            group(
                "Arm ASR accumulate",
                &self.layouts[3],
                &[
                    uniform.clone(),
                    view(1, color),
                    sampler.clone(),
                    view(6, &dilated_motion[i]),
                    locks_entry.clone(),
                    view(10, &masks),
                    view(11, &history[1 - i]),
                    view(12, &lock_status[1 - i]),
                ],
            )
        });
        let show = bind(&shown);
        Targets {
            render,
            output,
            dilated_depth,
            dilated_motion,
            lock_luma,
            masks,
            history,
            lock_status,
            color: shown,
            reconstruct,
            depth_clip,
            lock,
            accumulate,
            show,
        }
    }

    /// The picture this frame wrote, for a native pack's output to read (`resolve_taa`).
    pub(super) fn output(&self) -> Option<&wgpu::TextureView> {
        Some(&self.targets.as_ref()?.color)
    }

    /// The four passes for `frame`, and the bindings that show the picture they wrote.
    /// `phases`: the jitter's sequence length (TAA's Halton phases).
    pub(super) fn resolve(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &UpscaleFrame<'_>,
        (camera, phases): (Camera, u32),
        mut timing: Option<&mut super::pack_timing::PassTiming>,
    ) -> Option<&wgpu::BindGroup> {
        let t = self.targets.as_ref()?;
        let [rw, rh] = frame.render_size.map(|v| v as f32);
        let [ow, oh] = frame.output_size.map(|v| v as f32);
        let params = Params {
            clip_to_prev_clip: frame.reproject,
            device_to_view_depth: camera.device_to_view_depth(),
            render_size: [rw, rh],
            display_size: [ow, oh],
            jitter: frame.jitter_pixels,
            downscale_factor: [rw / ow, rh / oh],
            jitter_sequence_length: phases.max(1) as f32,
            reset: if frame.reset { 1.0 } else { 0.0 },
            pad: [0.0; 2],
        };
        queue.write_buffer(&self.uniform, 0, bytemuck::bytes_of(&params));
        let i = (frame.index % 2) as usize;
        draw(
            encoder,
            "Arm ASR reconstruct",
            &[&t.dilated_depth, &t.dilated_motion[i], &t.lock_luma],
            (&self.reconstruct, &t.reconstruct),
            None,
        );
        draw(
            encoder,
            "Arm ASR depth clip",
            &[&t.masks],
            (&self.depth_clip, &t.depth_clip[i]),
            None,
        );
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Arm ASR lock"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.lock);
            pass.set_bind_group(0, &t.lock, &[]);
            pass.dispatch_workgroups(t.render[0].div_ceil(8), t.render[1].div_ceil(8), 1);
        }
        draw(
            encoder,
            "Arm ASR accumulate",
            &[&t.history[i], &t.lock_status[i], &t.color],
            (&self.accumulate, &t.accumulate[i]),
            timing.as_deref_mut().and_then(|t| t.writes("Arm ASR")),
        );
        Some(&t.show)
    }
}

/// One full-screen pass of `pipeline` with `group`, onto `views`, cleared first.
fn draw(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    views: &[&wgpu::TextureView],
    (pipeline, group): (&wgpu::RenderPipeline, &wgpu::BindGroup),
    timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'_>>,
) {
    debug_assert!(views.len() <= 3);
    let mut attachments: [Option<wgpu::RenderPassColorAttachment<'_>>; 3] = [None, None, None];
    for (slot, &view) in attachments.iter_mut().zip(views.iter()) {
        *slot = Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        });
    }
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        timestamp_writes,
        color_attachments: &attachments[..views.len()],
        ..Default::default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::smaa::tests::{device, float_texture, read};

    /// A still picture, jittered as TAA jitters it: Arm ASR settles on it at Native and at twice
    /// the size, the flat parts as drawn (the frame's gamma light decoded and encoded again, light
    /// above paper white included), nothing outside the picture's range or half float's, and each
    /// pixel's alpha its own.
    #[test]
    fn arm_asr_settles_on_a_still_picture_at_native_and_upscaled() {
        let _gpu = crate::gfx::test_gpu::lock();
        let Some((device, queue, _)) = device() else {
            eprintln!("Arm ASR GPU test skipped: no device");
            return;
        };
        const N: u32 = 24;
        let (bright, dark) = (3.0, half::f16::from_f32(0.2).to_f32());
        let color = float_texture(&device, &queue, [N, N], |x, _| {
            let c = if x < N / 2 { bright } else { dark };
            [c, c, c, 0.25]
        });
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
            bytemuck::cast_slice(&vec![0.01f32; (N * N) as usize]),
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
        let camera = Camera {
            fov_y: 1.2,
            aspect: 1.0,
            near: 0.05,
            far: 512.0,
        };
        for scale in [1, 2] {
            let output = [N * scale; 2];
            let post = super::super::world_post::Post::new(&device, HISTORY_FORMAT, false);
            let mut arm = Arm::new(&device).unwrap_or_else(|e| panic!("Arm ASR: {e}"));
            let bind = |view: &wgpu::TextureView| post.bind(&device, view);
            arm.fit(&device, (&color_view, &depth_view), ([N, N], output), bind)
                .unwrap();
            let phases = 8 * scale * scale;
            for index in 0..32u32 {
                let i = index % phases + 1;
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
                    arm.resolve(&queue, &mut encoder, &frame, (camera, phases), None)
                        .is_some()
                );
                queue.submit([encoder.finish()]);
            }
            let shown = arm.output().unwrap().texture().clone();
            let encoder = device.create_command_encoder(&Default::default());
            let pixels = read(&device, &queue, encoder, &shown);
            let width = output[0] as usize;
            let at = |x: usize, y: usize| pixels[y * width + x];
            let what = format!("scale {scale}");
            for p in &pixels {
                assert!(p.iter().all(|c| c.is_finite()), "{what}: {p:?}");
                assert!(
                    (dark - 0.02..=bright + 0.02).contains(&p[0]),
                    "{what}: {p:?}"
                );
                assert_eq!(p[3], 0.25, "{what}: the alpha is the pixel's own");
            }
            let near = |a: f32, b: f32| (a - b).abs() <= 0.02 * b.max(1.0);
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

    #[test]
    fn device_depth_reads_back_as_the_distance_along_the_view() {
        let camera = Camera {
            fov_y: 1.2,
            aspect: 16.0 / 9.0,
            near: 0.05,
            far: 1024.0,
        };
        let f = camera.device_to_view_depth();
        let view = |d: f32| f[1] / (d - f[0]);
        // Reversed depth: 1 at the near plane, 0 at the far one.
        assert!((view(1.0) - 0.05).abs() < 1e-5);
        assert!((view(0.0) - 1024.0).abs() < 0.1);
        // A point 10 blocks away, as the world's projection writes its depth.
        let projection = glam::Mat4::perspective_rh(1.2, 16.0 / 9.0, 1024.0, 0.05);
        let clip = projection * glam::Vec4::new(0.0, 0.0, -10.0, 1.0);
        assert!((view(clip.z / clip.w) - 10.0).abs() < 1e-2);
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
