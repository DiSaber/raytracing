use std::sync::Arc;

use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::{
    scene::{GpuScene, Scene},
    shader_types::Uniforms,
};

pub struct State {
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,

    uniforms: Uniforms,
    uniform_buf: wgpu::Buffer,
    rt_target: wgpu::Texture,
    #[expect(dead_code)]
    rt_view: wgpu::TextureView,
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    tlas_package: wgpu::TlasPackage,
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group: wgpu::BindGroup,
    gpu_scene: GpuScene,
}

impl State {
    pub async fn new(window: Arc<Window>, scene: Scene) -> State {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::TEXTURE_BINDING_ARRAY
                    | wgpu::Features::VERTEX_WRITABLE_STORAGE
                    | wgpu::Features::EXPERIMENTAL_RAY_QUERY
                    | wgpu::Features::EXPERIMENTAL_RAY_TRACING_ACCELERATION_STRUCTURE,
                ..Default::default()
            })
            .await
            .unwrap();

        let gpu_scene = scene.upload_to_gpu(&device, &queue);

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(
            90.0_f32.to_radians(),
            size.width as f32 / size.height as f32,
            0.1,
            1000.0,
        );
        let uniforms = Uniforms {
            view_inverse: view.inverse(),
            proj_inverse: proj.inverse(),
        };
        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let side_count = 8;
        let tlas = device.create_tlas(&wgpu::CreateTlasDescriptor {
            label: None,
            flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
            update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            max_instances: side_count * side_count,
        });

        let rt_target = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rt_target"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        });

        let rt_view = rt_target.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Rgba8Unorm),
            dimension: Some(wgpu::TextureViewDimension::D2),
            usage: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("rt_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let rt_compute_shader =
            device.create_shader_module(wgpu::include_wgsl!("shaders/rt_compute.wgsl"));

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rt"),
            layout: None,
            module: &rt_compute_shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let compute_bind_group_layout = compute_pipeline.get_bind_group_layout(0);
        let tlas_package = wgpu::TlasPackage::new(tlas);

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &compute_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&rt_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: gpu_scene.vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: gpu_scene.index_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: gpu_scene.material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: gpu_scene.instance_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: tlas_package.as_binding(),
                },
            ],
        });

        let blit_shader = device.create_shader_module(wgpu::include_wgsl!("shaders/blit.wgsl"));

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit_pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(surface_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let blit_bind_group_layout = blit_pipeline.get_bind_group_layout(0);

        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&rt_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let state = State {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,
            uniforms,
            uniform_buf,
            rt_target,
            rt_view,
            compute_pipeline,
            compute_bind_group,
            tlas_package,
            blit_pipeline,
            blit_bind_group,
            gpu_scene,
        };

        state.configure_surface();

        state
    }

    fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        self.configure_surface();

        let proj = Mat4::perspective_rh(
            90.0_f32.to_radians(),
            self.size.width as f32 / self.size.height as f32,
            0.1,
            1000.0,
        );
        self.uniforms.proj_inverse = proj.inverse();
        self.queue
            .write_buffer(&self.uniform_buf, 0, bytemuck::cast_slice(&[self.uniforms]));
    }

    pub fn render(&mut self) {
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(self.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        // Keep in `render()` for transform change support in the future
        let mut tlas_i = 0usize;
        for (instance_i, (blas, transforms)) in self
            .gpu_scene
            .bottom_level_acceleration_structures
            .iter()
            .zip(self.gpu_scene.instance_transforms.iter())
            .enumerate()
        {
            for transform in transforms {
                self.tlas_package[tlas_i] = Some(wgpu::TlasInstance::new(
                    blas,
                    Mat4::from(transform).transpose().to_cols_array()[..12]
                        .try_into()
                        .unwrap(),
                    instance_i as u32,
                    0xff,
                ));
            }

            tlas_i += 1;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder
            .build_acceleration_structures(std::iter::empty(), std::iter::once(&self.tlas_package));

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        compute_pass.set_pipeline(&self.compute_pipeline);
        compute_pass.set_bind_group(0, Some(&self.compute_bind_group), &[]);
        compute_pass.dispatch_workgroups(
            self.rt_target.width() / 8,
            self.rt_target.height() / 8,
            1,
        );

        drop(compute_pass);

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.blit_pipeline);
        render_pass.set_bind_group(0, Some(&self.blit_bind_group), &[]);
        render_pass.draw(0..3, 0..1);

        drop(render_pass);

        self.queue.submit([encoder.finish()]);
        self.window.pre_present_notify();
        surface_texture.present();
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }
}
