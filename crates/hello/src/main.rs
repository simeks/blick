use winit::event::{Event, WindowEvent};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::platform::run_return::EventLoopExtRunReturn;


fn main() {
    let mut event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("blick")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");

    let backend = blick::Backend::new(
        &window,
        blick::BackendConfig {
            debugging: true,
        },
    );

    let mut renderer = Renderer::new(backend);

    let mut running = true;
    while running {
        event_loop.run_return(|event, _, control_flow| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            *control_flow = ControlFlow::Exit;
                            running = false;
                        },
                        WindowEvent::Resized(size) => {
                            renderer.resize(size.width, size.height);
                        },
                        _ => {}
                    }
                },
                winit::event::Event::MainEventsCleared => {
                    *control_flow = ControlFlow::Exit;
                },
                _ => {}
            }
        });
        renderer.draw_frame();
    }
}

#[allow(dead_code)]
struct Renderer {
    backend: blick::Backend,
    command_buffer: blick::CommandBuffer,
    descriptor_set_layout: blick::DescriptorSetLayout,
    descriptor_set: blick::DescriptorSet,
    buffer: blick::Buffer,
    render_pass: blick::RenderPass,
    compute_pipeline: blick::ComputePipeline,
    pipeline: blick::GraphicsPipeline,
    frame_idx: usize,
}

impl Renderer {
    pub fn new(
        render_backend: blick::Backend,
    ) -> Self {
        let device = render_backend.device();
        let command_buffer = device.create_command_buffer().unwrap();

        let descriptor_set_layout = device.create_descriptor_set_layout(
            blick::DescriptorSetLayoutDesc {
                entries: &[
                    blick::DescriptorSetLayoutEntry {
                        binding: 0,
                        stage_flags: blick::ShaderStageFlags::ALL,
                        ty: blick::DescriptorType::STORAGE_BUFFER,
                        count: 1,
                    },
                ],
            }
        ).unwrap();

        let buffer = device.create_buffer(
            blick::BufferDesc {
                size: 4*4*3,
                usage: blick::BufferUsage::STORAGE,
            }
        ).unwrap();

        let descriptor_set = device.create_descriptor_set(
            &descriptor_set_layout,
        ).unwrap();

        device.update_descriptor_set(
            &descriptor_set,
            &[
                blick::Descriptor {
                    binding: 0,
                    resource: &blick::DescriptorResource::Buffer {
                        buffer: &buffer,
                        offset: 0,
                        range: blick::WHOLE_SIZE,
                    },
                },
            ],
        ).unwrap();

        let render_pass = device.create_render_pass(
            blick::RenderPassDesc {
                color_attachments: &[
                    Some(blick::ColorAttachmentDesc {
                        // TODO: Format might change with swapchain change
                        format: render_backend.swapchain_desc().format,
                        layout: blick::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    })
                ]
            }
        ).unwrap();

        let compute_pipeline = device.create_compute_pipeline(
            blick::ComputePipelineDesc {
                shader_module: blick::ShaderModuleDesc {
                    source: blick::ShaderSource::Hlsl(
                        include_str!("../../../assets/shaders/triangle_cs.hlsl"),
                    ),
                    // TODO: Do we really need to spec stage for compute pipeline
                    stage: blick::ShaderStageFlags::COMPUTE,
                },
                descriptor_set_layouts: &[
                    &descriptor_set_layout,
                ],
                push_constant_ranges: &[
                    blick::PushConstantRange {
                        stage_flags: blick::ShaderStageFlags::COMPUTE,
                        offset: 0,
                        size: 4*3,
                    },
                ],
            }
        ).unwrap();

        let pipeline = device.create_graphics_pipeline(
            blick::GraphicsPipelineDesc {
                shader_modules: &[
                    blick::ShaderModuleDesc {
                        source: blick::ShaderSource::Hlsl(
                            include_str!("../../../assets/shaders/triangle_vs.hlsl"),
                        ),
                        stage: blick::ShaderStageFlags::VERTEX,
                    },
                    blick::ShaderModuleDesc {
                        source: blick::ShaderSource::Hlsl(
                            include_str!("../../../assets/shaders/triangle_ps.hlsl"),
                        ),
                        stage: blick::ShaderStageFlags::FRAGMENT,
                    },
                ],
                descriptor_set_layouts: &[&descriptor_set_layout],
                push_constant_ranges: &[],
                render_pass: &render_pass,
            }
        ).unwrap();

        Self {
            backend: render_backend,
            command_buffer,
            descriptor_set_layout,
            descriptor_set,
            buffer,
            render_pass,
            compute_pipeline,
            pipeline,
            frame_idx: 0,
        }
    }

    pub fn draw_frame(&mut self) {
        let frame = match self.backend.begin_frame() {
            Ok(frame) => frame,
            Err(blick::BeginFrameError::OutdatedSwapchain) => {
                // TODO:
                panic!("Skip frame: Swapchain out of date");
            },
        };

        let extent = frame.swapchain_image.image.desc.extent;

        let device = self.backend.device();
        let framebuffer = device.create_framebuffer(
            blick::FramebufferDesc {
                render_pass: &self.render_pass,
                attachments: &[
                    blick::Attachment {
                        image_view: &device.create_image_view(
                            &frame.swapchain_image.image,
                            blick::ImageViewDesc {
                                view_type: blick::ImageViewType::TYPE_2D,
                                aspect_mask: blick::ImageAspectFlags::COLOR,
                                format: frame.swapchain_image.image.desc.format,
                                base_mip_level: 0,
                                level_count: 1,
                            }
                        ).unwrap()
                    }
                ],
                extent: blick::Extent2d {
                    width: extent.width,
                    height: extent.height,
                },
            }
        ).unwrap();

        let extent = blick::Rect {
            x: 0,
            y: 0,
            width: extent.width,
            height: extent.height,
        };

        self.command_buffer.begin();

        let all_values: [[u32;3];3] = [
            [0, 0, 0],
            [1, 1, 1],
            [2, 2, 2],
        ];
        let values = all_values[(self.frame_idx / 60) % 3];

        let push_constants = unsafe {
            std::slice::from_raw_parts(values.as_ptr() as *const u8, 3*4)
        };

        self.command_buffer.begin_compute_pass()
            .bind_pipeline(&self.compute_pipeline)
            .bind_descriptor_set(0, &self.descriptor_set)
            .push_constants(
                    0,
                push_constants
            )
            .dispatch(3, 1, 1);

        self.command_buffer.transition(
            &[
                blick::BufferBarrier {
                    buffer: &self.buffer,
                    src_access_mask: blick::AccessFlags::SHADER_WRITE,
                    dst_access_mask: blick::AccessFlags::SHADER_READ,
                }
            ],
            &[],
            blick::PipelineStageFlags::COMPUTE_SHADER,
            blick::PipelineStageFlags::VERTEX_SHADER,
        );

        self.command_buffer.begin_render_pass(
                &self.render_pass,
                &framebuffer,
                &extent,
        )
            .bind_pipeline(&self.pipeline)
            .bind_descriptor_set(0, &self.descriptor_set)
            .set_viewport_and_scissor(&extent)
            .draw(3,1, 0, 0);


        self.command_buffer.transition(
            &[],
            &[
                blick::ImageBarrier {
                    image: &frame.swapchain_image.image,
                    src_access_mask: blick::AccessFlags::COLOR_ATTACHMENT_WRITE,
                    dst_access_mask: blick::AccessFlags::empty(),
                    old_layout: blick::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    new_layout: blick::ImageLayout::PRESENT_SRC_KHR,
                    aspect_mask: blick::ImageAspectFlags::COLOR,
                }
            ],
            blick::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            blick::PipelineStageFlags::BOTTOM_OF_PIPE,
        );

        self.command_buffer.end();

        self.backend.device().submit(
            &[&self.command_buffer],
            &[],
            &[&frame.render_finished],
            None,
        ).unwrap();

        match self.backend.end_frame(frame) {
            Ok(_) => {},
            Err(blick::EndFrameError::OutdatedSwapchain) => {
                panic!("end_frame: Swapchain out of date");
            },
        }

        self.frame_idx += 1;
    }

    fn resize(&mut self, width: u32, height: u32) {
        if self.backend.swapchain_desc().extent.width == width
        && self.backend.swapchain_desc().extent.height == height {
            return;
        }
        self.backend.resize_swapchain(width, height);
    }
}
