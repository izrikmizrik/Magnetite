//! SMAA 1x over the world (`smaa.wgsl`, after Jimenez et al.; MIT, THIRD_PARTY_NOTICES.md): edge
//! detection, the blending weights, the blend, at the world's size. Built when the setting first
//! asks for it, in its preset (`SmaaQuality`) and precision (`half`); every pipeline and target
//! under error scopes, so that a device that can't run it gets a message in the settings
//! (`graphics::antialiasing_failed`) and the world drawn without it, not a crash.

use crate::graphics::SmaaQuality;

/// The edges, the weights and SMAA's lookup textures' formats.
const EDGES_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg8Unorm;
const WEIGHTS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// SMAA's area texture (AreaTex.h, 160×560, two channels) and search texture (SearchTex.h,
/// 64×16, one), as the reference implementation ships them.
const AREA: &[u8] = include_bytes!("smaa/area.bin");
const SEARCH: &[u8] = include_bytes!("smaa/search.bin");

/// The pipelines and lookup textures, and the targets at the world's size.
pub(super) struct Smaa {
    quality: SmaaQuality,
    half: bool,
    /// The format the blend writes: the world's.
    format: wgpu::TextureFormat,
    edges_layout: wgpu::BindGroupLayout,
    weights_layout: wgpu::BindGroupLayout,
    blend_layout: wgpu::BindGroupLayout,
    edges: wgpu::RenderPipeline,
    weights: wgpu::RenderPipeline,
    blend: wgpu::RenderPipeline,
    area: wgpu::TextureView,
    search: wgpu::TextureView,
    sampler: wgpu::Sampler,
    targets: Option<Targets>,
}

/// The edges and weights of one world target, and the bindings that read it.
struct Targets {
    size: [u32; 2],
    edges: wgpu::TextureView,
    weights: wgpu::TextureView,
    edges_group: wgpu::BindGroup,
    weights_group: wgpu::BindGroup,
    blend_group: wgpu::BindGroup,
}

/// Runs `make` under validation, internal and out-of-memory scopes: what the device raised, as
/// text, instead of the panic an uncaught error would be.
pub(super) fn caught<T>(device: &wgpu::Device, make: impl FnOnce() -> T) -> Result<T, String> {
    let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let internal = device.push_error_scope(wgpu::ErrorFilter::Internal);
    let memory = device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
    let made = make();
    let error = pollster::block_on(memory.pop())
        .or(pollster::block_on(internal.pop()))
        .or(pollster::block_on(validation.pop()));
    match error {
        None => Ok(made),
        Some(e) => Err(e.to_string()),
    }
}

impl SmaaQuality {
    /// SMAA.hlsl's preset values: threshold, search steps, diagonal steps, corner rounding, and
    /// whether diagonals and corners are handled.
    fn constants(self) -> [(&'static str, f64); 6] {
        let (threshold, steps, diag_steps, diag, corners) = match self {
            SmaaQuality::Low => (0.15, 4.0, 8.0, false, false),
            SmaaQuality::Medium => (0.1, 8.0, 8.0, false, false),
            SmaaQuality::High => (0.1, 16.0, 8.0, true, true),
            SmaaQuality::Ultra => (0.05, 32.0, 16.0, true, true),
        };
        [
            ("THRESHOLD", threshold),
            ("MAX_SEARCH_STEPS", steps),
            ("MAX_SEARCH_STEPS_DIAG", diag_steps),
            ("DIAG", f64::from(u8::from(diag))),
            ("CORNER_ROUNDING", 25.0),
            ("CORNERS", f64::from(u8::from(corners))),
        ]
    }
}

impl Smaa {
    /// The pipelines and lookup textures, or the device's error.
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        quality: SmaaQuality,
        half: bool,
    ) -> Result<Self, String> {
        caught(device, || Self::build(device, queue, format, quality, half))?
    }

    fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        quality: SmaaQuality,
        half: bool,
    ) -> Result<Self, String> {
        use wgpu::util::DeviceExt;
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
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let layout = |label, entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries,
            })
        };
        let edges_layout = layout("SMAA edges", &[texture(0)]);
        let weights_layout = layout(
            "SMAA weights",
            &[sampler_entry, texture(2), texture(3), texture(4)],
        );
        let blend_layout = layout("SMAA blend", &[texture(0), sampler_entry, texture(5)]);
        let lookup = |label, width, height, format, data: &[u8]| {
            device
                .create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some(label),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[],
                    },
                    Default::default(),
                    data,
                )
                .create_view(&Default::default())
        };
        let area = lookup("SMAA area", 160, 560, wgpu::TextureFormat::Rg8Unorm, AREA);
        let search = lookup("SMAA search", 64, 16, wgpu::TextureFormat::R8Unorm, SEARCH);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SMAA"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let (edges, weights, blend) = Self::pipelines(
            device,
            [&edges_layout, &weights_layout, &blend_layout],
            format,
            quality,
            half,
        );
        Ok(Self {
            quality,
            half,
            format,
            edges_layout,
            weights_layout,
            blend_layout,
            edges,
            weights,
            blend,
            area,
            search,
            sampler,
            targets: None,
        })
    }

    /// The three passes in `quality`'s preset and `half`'s precision.
    fn pipelines(
        device: &wgpu::Device,
        [edges, weights, blend]: [&wgpu::BindGroupLayout; 3],
        format: wgpu::TextureFormat,
        quality: SmaaQuality,
        half: bool,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
        wgpu::RenderPipeline,
    ) {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Horizon SMAA"),
            source: wgpu::ShaderSource::Wgsl(
                super::half::shader(device, half, include_str!("smaa.wgsl")).into(),
            ),
        });
        let constants = quality.constants();
        let pipeline = |label, layout: &wgpu::BindGroupLayout, entry, target| {
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
                    entry_point: Some("vs_smaa"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: wgpu::PipelineCompilationOptions {
                        constants: &constants,
                        ..Default::default()
                    },
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        (
            pipeline("SMAA edges", edges, "fs_edges", EDGES_FORMAT),
            pipeline("SMAA weights", weights, "fs_weights", WEIGHTS_FORMAT),
            pipeline("SMAA blend", blend, "fs_blend", format),
        )
    }

    /// The preset and precision: the passes are built again when either changed.
    pub(super) fn set(
        &mut self,
        device: &wgpu::Device,
        quality: SmaaQuality,
        half: bool,
    ) -> Result<(), String> {
        if (quality, half) == (self.quality, self.half) {
            return Ok(());
        }
        let layouts = [&self.edges_layout, &self.weights_layout, &self.blend_layout];
        let (edges, weights, blend) = caught(device, || {
            Self::pipelines(device, layouts, self.format, quality, half)
        })?;
        (self.edges, self.weights, self.blend) = (edges, weights, blend);
        (self.quality, self.half) = (quality, half);
        Ok(())
    }

    /// The edges and weights for the world's colour `color` at `size`, made again when the
    /// world's target was (`forget`) or its size changed.
    pub(super) fn fit(
        &mut self,
        device: &wgpu::Device,
        color: &wgpu::TextureView,
        size: [u32; 2],
    ) -> Result<(), String> {
        if self.targets.as_ref().is_some_and(|t| t.size == size) {
            return Ok(());
        }
        self.targets = None;
        let targets = caught(device, || self.targets_for(device, color, size))?;
        self.targets = Some(targets);
        Ok(())
    }

    /// The world's target is new: the bindings that read the old one go.
    pub(super) fn forget(&mut self) {
        self.targets = None;
    }

    fn targets_for(
        &self,
        device: &wgpu::Device,
        color: &wgpu::TextureView,
        size: [u32; 2],
    ) -> Targets {
        let target = |label, format| {
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
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let edges = target("SMAA edges", EDGES_FORMAT);
        let weights = target("SMAA weights", WEIGHTS_FORMAT);
        let view = |binding, view| wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(view),
        };
        let sampler = wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        };
        let group = |label, layout, entries: &[wgpu::BindGroupEntry]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout,
                entries,
            })
        };
        let edges_group = group("SMAA edges", &self.edges_layout, &[view(0, color)]);
        let weights_group = group(
            "SMAA weights",
            &self.weights_layout,
            &[
                sampler.clone(),
                view(2, &edges),
                view(3, &self.area),
                view(4, &self.search),
            ],
        );
        let blend_group = group(
            "SMAA blend",
            &self.blend_layout,
            &[view(0, color), sampler, view(5, &weights)],
        );
        Targets {
            size,
            edges,
            weights,
            edges_group,
            weights_group,
            blend_group,
        }
    }

    /// Whether `fit` has the targets for this frame.
    pub(super) fn ready(&self) -> bool {
        self.targets.is_some()
    }

    /// The three passes over the world, blending onto `out`, a target of the world's size.
    pub(super) fn run(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        out: &wgpu::TextureView,
        mut timing: Option<&mut super::pack_timing::PassTiming>,
    ) {
        let Some(t) = &self.targets else { return };
        let mut pass = |label, view: &wgpu::TextureView, pipeline, group, timed: bool| {
            let attachments = [Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    // The edge pass discards where there is no edge: it starts from none.
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })];
            let writes = if timed {
                timing.as_deref_mut().and_then(|t| t.writes("SMAA"))
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
        pass("SMAA edges", &t.edges, &self.edges, &t.edges_group, false);
        pass(
            "SMAA weights",
            &t.weights,
            &self.weights,
            &t.weights_group,
            false,
        );
        pass("SMAA blend", out, &self.blend, &t.blend_group, true);
    }
}

/// What SMAA and SGSR 1 do to a picture on the GPU, in both precisions where the device has
/// half floats (lavapipe does).
#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A device with `SHADER_F16` where the adapter has it, and whether it does.
    pub(in crate::gfx) fn device() -> Option<(wgpu::Device, wgpu::Queue, bool)> {
        let adapter = crate::gfx::test_gpu::adapter()?;
        let half = adapter.features().contains(wgpu::Features::SHADER_F16);
        let descriptor = wgpu::DeviceDescriptor {
            required_features: if half {
                wgpu::Features::SHADER_F16
            } else {
                wgpu::Features::empty()
            },
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).ok()?;
        Some((device, queue, half))
    }

    /// An `Rgba16Float` texture of `size` holding `pixel(x, y)`.
    pub(in crate::gfx) fn float_texture(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: [u32; 2],
        pixel: impl Fn(u32, u32) -> [f32; 4],
    ) -> wgpu::Texture {
        use wgpu::util::DeviceExt;
        let mut data = Vec::new();
        for y in 0..size[1] {
            for x in 0..size[0] {
                for c in pixel(x, y) {
                    data.extend(half::f16::from_f32(c).to_le_bytes());
                }
            }
        }
        device.create_texture_with_data(
            queue,
            &wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba16Float,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            },
            Default::default(),
            &data,
        )
    }

    /// The pixels of an `Rgba16Float` `texture`, row by row.
    pub(in crate::gfx) fn read(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
        texture: &wgpu::Texture,
    ) -> Vec<[f32; 4]> {
        let (width, height) = (texture.width(), texture.height());
        let bytes_per_row = (width * 8).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(bytes_per_row * height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            texture.size(),
        );
        queue.submit([encoder.finish()]);
        buffer.map_async(wgpu::MapMode::Read, .., |r| r.unwrap());
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        let data = buffer.get_mapped_range(..).unwrap();
        let value = |p: &[u8]| half::f16::from_le_bytes([p[0], p[1]]).to_f32();
        (0..height as usize)
            .flat_map(|y| {
                let row = &data[y * bytes_per_row as usize..][..width as usize * 8];
                row.chunks(8)
                    .map(|p| [value(p), value(&p[2..]), value(&p[4..]), value(&p[6..])])
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// A shallow slope's stairs between light above paper white and a dark ground: SMAA blends
    /// the pixels along the stairs, leaves the flat parts and the light above 1 as they were,
    /// writes nothing out of half float's range, and keeps each pixel's alpha (HDR's light code).
    #[test]
    fn smaa_softens_stairs_and_keeps_flat_light_above_white() {
        let _gpu = crate::gfx::test_gpu::lock();
        let Some((device, queue, has_half)) = device() else {
            eprintln!("SMAA GPU test skipped: no device");
            return;
        };
        const N: u32 = 48;
        // As the half-float texture holds them.
        let (bright, dark) = (2.0, half::f16::from_f32(0.1).to_f32());
        // Above the line y = x / 4 + 12, bright; below it, dark: steps four pixels long.
        let lit = |x: u32, y: u32| (y as f32) < x as f32 / 4.0 + 12.0;
        let input = float_texture(&device, &queue, [N, N], |x, y| {
            let c = if lit(x, y) { bright } else { dark };
            [c, c, c, 0.5]
        });
        let color = input.create_view(&Default::default());
        if !has_half {
            eprintln!("SMAA GPU test: no SHADER_F16 on this device, f32 only");
        }
        for half in [false, true].into_iter().filter(|h| !h || has_half) {
            let format = wgpu::TextureFormat::Rgba16Float;
            let mut smaa = Smaa::new(&device, &queue, format, SmaaQuality::High, half)
                .unwrap_or_else(|e| panic!("SMAA (half: {half}): {e}"));
            smaa.fit(&device, &color, [N, N]).unwrap();
            let out = float_texture(&device, &queue, [N, N], |_, _| [0.0; 4]);
            let mut encoder = device.create_command_encoder(&Default::default());
            smaa.run(&mut encoder, &out.create_view(&Default::default()), None);
            let pixels = read(&device, &queue, encoder, &out);
            let at = |x: u32, y: u32| pixels[(y * N + x) as usize];
            let blended = pixels
                .iter()
                .filter(|p| p[0] > dark + 0.05 && p[0] < bright - 0.05)
                .count();
            assert!(
                blended >= 8,
                "half: {half}: {blended} pixels blended along the stairs"
            );
            for p in &pixels {
                assert!(
                    p.iter().all(|c| c.is_finite() && *c >= 0.0),
                    "half: {half}: {p:?}"
                );
                assert!(p[0] <= bright + 1e-3, "half: {half}: {p:?} overshoots");
                assert_eq!(p[3], 0.5, "half: {half}: the alpha is the pixel's own");
            }
            // Far from the stairs: as drawn, the light above paper white included.
            assert_eq!(at(2, 2)[0], bright, "half: {half}");
            assert_eq!(at(N - 3, N - 3)[0], dark, "half: {half}");
        }
    }
}
