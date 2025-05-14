// TODO: Clean-up, visibility.
use std::{collections::VecDeque, sync::Arc};

use bytemuck::{Pod, Zeroable};
use smallvec::{SmallVec, smallvec};
use thiserror::Error;
use ultraviolet::{Vec2, Vec4};
use wgpu::{
    AddressMode,
    Backends,
    BindGroup,
    BindGroupDescriptor,
    BindGroupEntry,
    BindGroupLayoutDescriptor,
    BindGroupLayoutEntry,
    BindingResource,
    BindingType,
    BlendState,
    Buffer,
    BufferAddress,
    BufferBindingType,
    BufferDescriptor,
    BufferSize,
    BufferUsages,
    Color,
    ColorTargetState,
    ColorWrites,
    CommandEncoder,
    CommandEncoderDescriptor,
    CompositeAlphaMode,
    ComputePassDescriptor,
    ComputePipeline,
    ComputePipelineDescriptor,
    CreateSurfaceError,
    Device,
    DeviceDescriptor,
    Extent3d,
    Features,
    FilterMode,
    FragmentState,
    Instance,
    InstanceDescriptor,
    LoadOp,
    Maintain,
    MapMode,
    MultisampleState,
    Operations,
    PipelineCompilationOptions,
    PipelineLayoutDescriptor,
    PowerPreference,
    PresentMode,
    PrimitiveState,
    Queue,
    RenderPassColorAttachment,
    RenderPassDescriptor,
    RenderPipeline,
    RenderPipelineDescriptor,
    RequestAdapterOptions,
    RequestDeviceError,
    SamplerBindingType,
    SamplerDescriptor,
    ShaderStages,
    StorageTextureAccess,
    StoreOp,
    Surface,
    SurfaceConfiguration,
    SurfaceError,
    TexelCopyBufferInfo,
    TexelCopyBufferLayout,
    Texture,
    TextureDescriptor,
    TextureDimension,
    TextureFormat,
    TextureSampleType,
    TextureUsages,
    TextureView,
    TextureViewDescriptor,
    TextureViewDimension,
    VertexAttribute,
    VertexBufferLayout,
    VertexFormat,
    VertexState,
    VertexStepMode,
    util::{BufferInitDescriptor, DeviceExt},
};
use winit::window::Window;

use crate::helpers::{Position, Size};

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Equivalent to [`wgpu::RequestAdapterError`]
    #[error("No `wgpu::Adapter` found.")]
    AdapterNotFound, // AdapterNotFound(#[from] wgpu::RequestAdapterError),
    /// Equivalent to [`wgpu::RequestDeviceError`]
    #[error("No `wgpu::Device` found.")]
    DeviceNotFound(#[from] RequestDeviceError),
    /// Equivalent to [`wgpu::CreateSurfaceError`]
    #[error("Unable to create a surface.")]
    CreateSurface(#[from] CreateSurfaceError),
    /// Equivalent to [`wgpu::SurfaceError`]
    #[error("The GPU failed to acquire a surface frame.")]
    Surface(#[from] SurfaceError),
    /// No texture format found
    #[error("No `wgpu::TextureFormat` found.")]
    TextureFormatNotFound,
    /// No present mode found
    #[error("No `wgpu::PresentMode` found.")]
    PresentModeNotFound,
    /// Unable to create a backing texture; Width is greater than GPU limits
    #[error("Texture width is invalid: {0}")]
    TextureWidth(u32),
    /// Unable to create a backing texture; Height is greater than GPU limits
    #[error("Texture height is invalid: {0}")]
    TextureHeight(u32),
}

// #[derive(Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
pub struct StorageBufferObject {
    pub length: u32,
    padding: [u8; 4],
    pub points: SmallVec<[Vec2; 4096]>,
}

impl StorageBufferObject {
    pub fn as_bytes(&self) -> SmallVec<[u8; size_of::<Self>()]> {
        let mut vec = SmallVec::new();
        vec.extend_from_slice(bytemuck::bytes_of(&self.length)); // TODO: self.points.len()
        vec.extend_from_slice(bytemuck::cast_slice(&self.padding));
        vec.extend_from_slice(bytemuck::cast_slice(&self.points));
        vec
    }
}

impl Default for StorageBufferObject {
    fn default() -> Self {
        Self {
            length: 0,
            padding: Default::default(),
            points: smallvec![Vec2::default(); 4096],
        }
    }
}

// Respect std140 alignment!
#[derive(Clone, Copy, Default, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct ComputeUniformBufferObject {
    pub color: Vec4,
    pub action: u32,
    pub stroke: f32,
    pub anti_aliasing_scale: f32,
    pub dash_length: f32,
    pub gap_length: f32,
    _padding: [u8; 12],
}

// Respect std140 alignment!
#[derive(Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
pub struct VertexUniformBufferObject {
    pub scale: Vec2,
    pub offset: Vec2,
}

// Respect std140 alignment!
#[derive(Clone, Copy, Default, Pod, Zeroable)]
#[repr(C)]
pub struct FragmentUniformBufferObject {
    pub grid_scale: Vec2,
    pub action: u32,
    pub preview: u32, // bool
}

#[derive(Clone, Copy)]
pub enum CopyDirection {
    BackToFront,
    FrontToBack,
}

pub struct Renderer {
    pub window: Arc<Window>,
    pub window_size: Size<u32>,
    pub device: Device,
    pub queue: Queue,
    surface: Surface<'static>,
    pub texture_size: Size<u32>,
    texture_extent: Extent3d,
    pub texture_format: TextureFormat,
    present_mode: PresentMode,
    back_texture: Texture,
    front_texture: Texture,
    pub storage_buffer_object: StorageBufferObject,
    pub storage_buffer_object_changed: bool,
    storage_buffer: Buffer,
    pub compute_uniform_buffer_object: ComputeUniformBufferObject,
    pub compute_uniform_buffer_object_changed: bool,
    compute_uniform_buffer: Buffer,
    compute_bind_group: BindGroup,
    compute_pipeline: ComputePipeline,
    pub vertex_uniform_buffer_object: VertexUniformBufferObject,
    pub vertex_uniform_buffer_object_changed: bool,
    vertex_uniform_buffer: Buffer,
    pub fragment_uniform_buffer_object: FragmentUniformBufferObject,
    pub fragment_uniform_buffer_object_changed: bool,
    fragment_uniform_buffer: Buffer,
    render_bind_group: BindGroup,
    vertex_buffer: Buffer,
    render_pipeline: RenderPipeline,
    fill_buffer_bytes_per_row: u32,
    fill_buffer: Buffer,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, texture_size: Size<u32>) -> Result<Self, Error> {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                // Same as setting `WGPU_POWER_PREF` to `high`.
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await
            .ok_or(Error::AdapterNotFound)?;
        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("device"),
                    required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
                        | Features::MAPPABLE_PRIMARY_BUFFERS,
                    required_limits: adapter.limits(),
                    ..Default::default()
                },
                None,
            )
            .await?;
        let limits = device.limits();
        if texture_size.width == 0 || texture_size.width > limits.max_texture_dimension_2d {
            return Err(Error::TextureWidth(texture_size.width));
        }
        if texture_size.height == 0 || texture_size.height > limits.max_texture_dimension_2d {
            return Err(Error::TextureHeight(texture_size.height));
        }
        let window_size = Size::<u32>::from(window.inner_size());
        let surface = instance.create_surface(window.clone())?;
        let capabilities = surface.get_capabilities(&adapter);
        let texture_extent = Extent3d {
            width: texture_size.width,
            height: texture_size.height,
            depth_or_array_layers: 1,
        };
        let texture_format = capabilities
            .formats
            .into_iter()
            .filter(|format| {
                let features = format.guaranteed_format_features(device.features());
                features.allowed_usages.contains(
                    TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST
                        | TextureUsages::TEXTURE_BINDING
                        | TextureUsages::STORAGE_BINDING
                        | TextureUsages::RENDER_ATTACHMENT,
                )
                // && features
                // .flags
                // .contains(TextureFormatFeatureFlags::STORAGE_READ_WRITE)
            })
            .max_by_key(|format| match format {
                TextureFormat::Rgba8Unorm => 2,
                TextureFormat::Bgra8Unorm => 1,
                _ => 0,
            })
            .ok_or(Error::TextureFormatNotFound)?;
        let present_mode = capabilities
            .present_modes
            .into_iter()
            .max_by_key(|present_mode| match present_mode {
                PresentMode::Mailbox => 4,
                PresentMode::Fifo => 3,
                PresentMode::FifoRelaxed => 2,
                PresentMode::Immediate => 1,
                _ => 0,
            })
            .ok_or(Error::PresentModeNotFound)?;
        let back_texture = device.create_texture(&TextureDescriptor {
            label: Some("back texture"),
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: texture_format,
            usage: TextureUsages::COPY_SRC | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let front_texture = device.create_texture(&TextureDescriptor {
            label: Some("front texture"),
            size: texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: texture_format,
            usage: TextureUsages::COPY_SRC
                | TextureUsages::COPY_DST
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let front_texture_view = front_texture.create_view(&TextureViewDescriptor {
            label: Some("front texture view"),
            ..Default::default()
        });
        let storage_buffer_object = StorageBufferObject::default();
        let storage_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("storage buffer"),
            contents: &storage_buffer_object.as_bytes(),
            usage: BufferUsages::COPY_DST | BufferUsages::STORAGE,
        });
        let compute_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/compute.wgsl"));
        let compute_uniform_buffer_object = ComputeUniformBufferObject::default();
        let compute_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("compute uniform buffer"),
            contents: bytemuck::cast_slice(&[compute_uniform_buffer_object]),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });
        let compute_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("compute bind gropu layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None, // BufferSize::new(size_of::<StorageBufferObject>() as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(size_of::<ComputeUniformBufferObject>() as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: texture_format,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });
        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("compute bind group"),
            layout: &compute_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: storage_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: compute_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&front_texture_view),
                },
            ],
        });
        let compute_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("compute pipeline layout"),
            bind_group_layouts: &[&compute_bind_group_layout],
            push_constant_ranges: &[],
        });
        // let compute_pipeline_cache = unsafe {
        //     device.create_pipeline_cache(&PipelineCacheDescriptor {
        //         label: Some("compute pipeline cache"),
        //         data: None,
        //         fallback: true,
        //     })
        // };
        let compute_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some("compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("compute"),
            compilation_options: PipelineCompilationOptions::default(),
            cache: None, // Some(&compute_pipeline_cache)
        });
        let render_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/render.wgsl"));
        let vertex_uniform_buffer_object = VertexUniformBufferObject::default();
        let vertex_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex uniform buffer"),
            contents: bytemuck::cast_slice(&[vertex_uniform_buffer_object]),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });
        let fragment_uniform_buffer_object = FragmentUniformBufferObject::default();
        let fragment_uniform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("fragment uniform buffer"),
            contents: bytemuck::cast_slice(&[fragment_uniform_buffer_object]),
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        });
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 1.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        });
        let render_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("render bind group layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(size_of::<VertexUniformBufferObject>() as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage {
                            read_only: true,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None, // BufferSize::new(size_of::<StorageBufferObject>() as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(size_of::<FragmentUniformBufferObject>() as u64),
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float {
                            filterable: true,
                        },
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let render_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("render bind group"),
            layout: &render_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: vertex_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: storage_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: fragment_uniform_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(&front_texture_view),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });
        // TODO: Use a triangle and add clip rect.
        // #[rustfmt::skip]
        // let vertex_data: [[f32; 2]; 3] = [
        //     [-1.0, 1.0],
        //     [3.0, 1.0],
        //     [-1.0, -3.0],
        // ];
        #[rustfmt::skip]
        let vertex_data: [[f32; 2]; 6] = [
            [-1.0, -1.0],
            [1.0, -1.0],
            [-1.0, 1.0],
            [-1.0, 1.0],
            [1.0, -1.0],
            [1.0, 1.0],
        ];
        let vertex_data_bytes = bytemuck::cast_slice(&vertex_data);
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: vertex_data_bytes,
            usage: BufferUsages::VERTEX,
        });
        let vertex_buffer_layout = VertexBufferLayout {
            array_stride: (vertex_data_bytes.len() / vertex_data.len()) as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: VertexFormat::Float32x2,
                offset: 0,
                shader_location: 0,
            }],
        };
        let render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render pipeline layout"),
            bind_group_layouts: &[&render_bind_group_layout],
            push_constant_ranges: &[],
        });
        // let render_pipeline_cache = unsafe {
        //     device.create_pipeline_cache(&PipelineCacheDescriptor {
        //         label: Some("render pipeline cache"),
        //         data: None,
        //         fallback: true,
        //     })
        // };
        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: VertexState {
                module: &render_shader,
                entry_point: Some("vertex"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[vertex_buffer_layout],
            },
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &render_shader,
                entry_point: Some("fragment"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        let fill_buffer_bytes_per_row = (texture_size.width * 4).div_ceil(256) * 256;
        let fill_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("fill buffer"),
            size: u64::from(fill_buffer_bytes_per_row * texture_size.height),
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let renderer = Self {
            window,
            window_size,
            device,
            queue,
            surface,
            texture_size,
            texture_extent,
            texture_format,
            present_mode,
            back_texture,
            front_texture,
            storage_buffer_object,
            storage_buffer_object_changed: false,
            storage_buffer,
            compute_uniform_buffer_object,
            compute_uniform_buffer_object_changed: false,
            compute_uniform_buffer,
            compute_bind_group,
            compute_pipeline,
            vertex_uniform_buffer_object,
            vertex_uniform_buffer_object_changed: false,
            vertex_uniform_buffer,
            fragment_uniform_buffer_object,
            fragment_uniform_buffer_object_changed: false,
            fragment_uniform_buffer,
            render_bind_group,
            vertex_buffer,
            render_pipeline,
            fill_buffer_bytes_per_row,
            fill_buffer,
        };
        renderer.configure_surface();
        Ok(renderer)
    }

    fn configure_surface(&self) {
        self.surface.configure(&self.device, &SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: self.texture_format,
            width: self.window_size.width,
            height: self.window_size.height,
            present_mode: self.present_mode,
            desired_maximum_frame_latency: 2,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
        });
    }

    pub fn resize_window(&mut self, size: Size<u32>, zoom: f32, offset: Position<f32>) {
        self.window_size = size;
        self.configure_surface();
        self.scale_texture(zoom);
        self.vertex_uniform_buffer_object.offset = Vec2::new(offset.x, offset.y); // TODO: Trait. This doesn't need to change!
        self.vertex_uniform_buffer_object_changed = true;
        self.window.request_redraw();
    }

    // pub fn resize_texture(&mut self, size: Size<u32>) {
    //     todo!();
    // }

    pub fn scale_texture(&mut self, zoom: f32) {
        // TODO: Implement a trait to convert Size<T> to Size<U>.
        let (window_width, window_height) = (self.window_size.width as f32, self.window_size.height as f32);
        let (texture_width, texture_height) = (self.texture_size.width as f32, self.texture_size.height as f32);

        let window_ratio = window_width / window_height;
        let texture_ratio = texture_width / texture_height;

        self.vertex_uniform_buffer_object.scale =
            Vec2::new((texture_ratio / window_ratio).min(1.0), (window_ratio / texture_ratio).min(1.0)) * zoom * 0.01;
        self.vertex_uniform_buffer_object_changed = true;
    }

    pub fn render_with<F>(&mut self, render_function: F) -> Result<(), Error>
    where
        F: FnOnce(&mut CommandEncoder, &TextureView, &Self),
    {
        if self.storage_buffer_object_changed {
            self.storage_buffer_object_changed = false;
            self.queue.write_buffer(&self.storage_buffer, 0, &self.storage_buffer_object.as_bytes());
        }
        if self.vertex_uniform_buffer_object_changed {
            self.vertex_uniform_buffer_object_changed = false;
            self.queue.write_buffer(
                &self.vertex_uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.vertex_uniform_buffer_object]),
            );
        }
        if self.fragment_uniform_buffer_object_changed {
            self.fragment_uniform_buffer_object_changed = false;
            self.queue.write_buffer(
                &self.fragment_uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.fragment_uniform_buffer_object]),
            );
        }
        let current_texture = self.surface.get_current_texture().or_else(|_| {
            // Reconfigure the surface and retry immediately on any error.
            // See https://github.com/parasyte/pixels/issues/121
            // See https://github.com/parasyte/pixels/issues/346
            self.configure_surface();
            self.surface.get_current_texture()
        })?;
        let current_texture_view = current_texture.texture.create_view(&TextureViewDescriptor {
            label: Some("current texture view"),
            ..Default::default()
        });
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("command encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &current_texture_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.2,
                            g: 0.2,
                            b: 0.2,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.render_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // render_pass.set_scissor_rect(self.clip_rect.0, self.clip_rect.1, self.clip_rect.2, self.clip_rect.3);
            // render_pass.draw(0..3, 0..1);
            render_pass.draw(0..6, 0..1);
        }
        (render_function)(&mut encoder, &current_texture_view, self);
        self.queue.submit([encoder.finish()]);
        self.device.poll(Maintain::Wait);
        self.window.pre_present_notify();
        current_texture.present();
        Ok(())
    }

    #[allow(dead_code)]
    #[inline]
    pub fn render(&mut self) -> Result<(), Error> {
        self.render_with(|_encoder, _current_texture_view, _renderer| {})
    }

    pub fn cursor_absolute_to_relative(&self, absolute: Position<f32>) -> Position<f32> {
        // TODO: Implement a trait to convert Size<T> to Size<U>.
        let (window_width, window_height) = (self.window_size.width as f32, self.window_size.height as f32);
        let (texture_width, texture_height) = (self.texture_size.width as f32, self.texture_size.height as f32);

        let ndc_x = (absolute.x / window_width).mul_add(2.0, -1.0);
        let ndc_y = (absolute.y / window_height).mul_add(-2.0, 1.0);

        let quad_x = (ndc_x / self.vertex_uniform_buffer_object.scale.x) - self.vertex_uniform_buffer_object.offset.x;
        let quad_y = (ndc_y / self.vertex_uniform_buffer_object.scale.y) - self.vertex_uniform_buffer_object.offset.y;

        let uv_x = quad_x.mul_add(0.5, 0.5);
        let uv_y = quad_y.mul_add(-0.5, 0.5);

        let tex_x = uv_x * texture_width;
        let tex_y = uv_y * texture_height;

        Position::new(tex_x, tex_y)
    }

    pub fn draw(&mut self) {
        if self.storage_buffer_object_changed {
            self.storage_buffer_object_changed = false;
            self.queue.write_buffer(&self.storage_buffer, 0, &self.storage_buffer_object.as_bytes());
        }
        if self.compute_uniform_buffer_object_changed {
            self.compute_uniform_buffer_object_changed = false;
            self.queue.write_buffer(
                &self.compute_uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.compute_uniform_buffer_object]),
            );
        }
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("command encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("compute pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_bind_group, &[]);
            // TODO: Just force a texture extent that is a multiple of 8. That would also make the id check in the
            // compute shader redundant.
            compute_pass.dispatch_workgroups(
                self.texture_size.width.div_ceil(8),
                self.texture_size.height.div_ceil(8),
                1,
            );
        }
        self.queue.submit([encoder.finish()]);
        self.device.poll(Maintain::Wait);
        self.window.request_redraw();
    }

    pub fn copy_texture(&self, direction: CopyDirection) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("command encoder"),
        });
        let (source, destination) = match direction {
            CopyDirection::BackToFront => (&self.back_texture, &self.front_texture),
            CopyDirection::FrontToBack => (&self.front_texture, &self.back_texture),
        };
        // TODO: Copy only the smallet part that needs to be copied.
        encoder.copy_texture_to_texture(source.as_image_copy(), destination.as_image_copy(), self.texture_extent);
        self.queue.submit([encoder.finish()]);
        self.device.poll(Maintain::Wait);
    }

    pub fn fill(&self, position: Position<u32>, color: [u8; 4]) {
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("command encoder"),
        });
        encoder.copy_texture_to_buffer(
            self.front_texture.as_image_copy(),
            TexelCopyBufferInfo {
                buffer: &self.fill_buffer,
                // https://docs.rs/wgpu/latest/wgpu/struct.TexelCopyBufferLayout.html
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.fill_buffer_bytes_per_row),
                    rows_per_image: None,
                },
            },
            self.texture_extent,
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(Maintain::Wait);
        self.fill_buffer.slice(..).map_async(MapMode::Read, |_| ());
        self.device.poll(Maintain::Wait);
        {
            let mut buffer = self.fill_buffer.slice(..).get_mapped_range_mut();
            self.flood_fill(&mut buffer, self.texture_size, position, color);
        }
        self.fill_buffer.unmap();
        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("command encoder"),
        });
        encoder.copy_buffer_to_texture(
            TexelCopyBufferInfo {
                buffer: &self.fill_buffer,
                // https://docs.rs/wgpu/latest/wgpu/struct.TexelCopyBufferLayout.html
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.fill_buffer_bytes_per_row),
                    rows_per_image: None,
                },
            },
            self.front_texture.as_image_copy(),
            self.texture_extent,
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(Maintain::Wait);
    }

    // TODO: Clean-up!
    fn flood_fill(&self, buffer: &mut [u8], texture_size: Size<u32>, position: Position<u32>, new_color: [u8; 4]) {
        if position.x >= texture_size.width || position.y >= texture_size.height {
            return;
        }

        let start_idx = (position.y * self.fill_buffer_bytes_per_row + position.x * 4) as usize;
        let target_color: [u8; 4] = buffer[start_idx..start_idx + 4].try_into().unwrap();

        if target_color == new_color {
            return;
        }

        let mut queue = VecDeque::new();
        queue.push_back((position.x as i32, position.y as i32));

        while let Some((x, y)) = queue.pop_front() {
            if x < 0 || y < 0 || x >= texture_size.width as i32 || y >= texture_size.height as i32 {
                continue;
            }

            let idx = (y as u32 * self.fill_buffer_bytes_per_row + x as u32 * 4) as usize;
            if buffer[idx..idx + 4] != target_color {
                continue;
            }

            buffer[idx..idx + 4].copy_from_slice(&new_color);

            queue.push_back((x - 1, y));
            queue.push_back((x + 1, y));
            queue.push_back((x, y - 1));
            queue.push_back((x, y + 1));
        }
    }
}
