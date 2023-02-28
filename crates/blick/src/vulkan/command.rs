use ash::vk;
use std::ffi::CString;
use std::sync::Arc;

pub struct RenderPassEncoder<'a> {
    parent: &'a mut CommandBuffer,
    active_pipeline: Option<&'a crate::GraphicsPipeline>,
}

pub struct ComputePassEncoder<'a> {
    parent: &'a mut CommandBuffer,
    active_pipeline: Option<&'a crate::ComputePipeline>,
}

pub struct CommandBuffer {
    pub(super) raw: vk::CommandBuffer,
    command_pool: vk::CommandPool,
    device: Arc<super::DeviceInner>,
}

impl<'a> RenderPassEncoder<'a> {
    pub fn begin(
        parent: &'a mut CommandBuffer,
        pass: &crate::RenderPass,
        framebuffer: &crate::Framebuffer,
        render_area: &crate::Rect<u32>
    ) -> Self {
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(pass.raw())
            .framebuffer(framebuffer.raw())
            .render_area(vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent: vk::Extent2D {
                    width: render_area.width,
                    height: render_area.height,
                },
            })
            .clear_values(
                // TODO:
                &(0..pass.num_attachments())
                    .map(|_| vk::ClearValue {
                        color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] },
                    })
                    .collect::<Vec<_>>()
            )
            .build();

        unsafe {
            parent.device.raw.cmd_begin_render_pass(
                parent.raw,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE
            );
        }

        Self {
            parent,
            active_pipeline: None,
        }
    }

    pub fn bind_pipeline(
        mut self,
        pipeline: &'a crate::GraphicsPipeline
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_bind_pipeline(
                self.parent.raw,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.raw
            );
        }
        self.active_pipeline = Some(pipeline);
        self
    }

    pub fn set_viewport(
        self,
        rect: &crate::Rect<f32>,
        min_depth: f32,
        max_depth: f32
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_set_viewport(
                self.parent.raw,
                0,
                &[vk::Viewport{
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    min_depth,
                    max_depth,
                }],
            );
        }
        self
    }
    pub fn set_scissor(self, rect: &crate::Rect<u32>) -> Self {
        unsafe {
            self.parent.device.raw.cmd_set_scissor(
                self.parent.raw,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: rect.x as _,
                        y: rect.y as _,
                    },
                    extent: vk::Extent2D {
                        width: rect.width,
                        height: rect.height
                    },
                }],
            );
        }
        self
    }

    /// Lazy setting of viewport and scissor
    pub fn set_viewport_and_scissor(self, rect: &crate::Rect<u32>) -> Self {
        unsafe {
            self.parent.device.raw.cmd_set_viewport(
                self.parent.raw,
                0,
                &[vk::Viewport{
                    x: rect.x as _,
                    y: rect.y as _,
                    width: rect.width as _,
                    height: rect.height as _,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );
            self.parent.device.raw.cmd_set_scissor(
                self.parent.raw,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D {
                        x: rect.x as _,
                        y: rect.y as _,
                    },
                    extent: vk::Extent2D {
                        width: rect.width,
                        height: rect.height
                    },
                }],
            );
        }
        self
    }

    pub fn bind_descriptor_set(
        self,
        index: u32,
        set: &crate::DescriptorSet,
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_bind_descriptor_sets(
                self.parent.raw,
                vk::PipelineBindPoint::GRAPHICS,
                self.active_pipeline.unwrap().pipeline_layout,
                index,
                &[set.raw],
                &[],
            );
        }
        self
    }

    pub fn push_constants(self, offset: u32, data: &[u8]) -> Self {
        unsafe {
            self.parent.device.raw.cmd_push_constants(
                self.parent.raw,
                self.active_pipeline.unwrap().pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                offset as _,
                data,
            );
        }
        self
    }

    pub fn bind_index_buffer(
        self,
        buffer: &crate::Buffer,
        offset: u64,
        index_type: crate::IndexType,
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_bind_index_buffer(
                self.parent.raw,
                buffer.raw,
                offset,
                index_type
            );
        }
        self
    }

    pub fn draw(
        self,
        vertex_count: u32,
        instance_count: u32,
        first_vertex: u32,
        first_instance: u32,
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_draw(
                self.parent.raw,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance
            );
        }
        self
    }

    pub fn draw_indexed(
        self,
        index_count: u32,
        instance_count: u32,
        first_index: u32,
        vertex_offset: i32,
        first_instance: u32,
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_draw_indexed(
                self.parent.raw,
                index_count,
                instance_count,
                first_index,
                vertex_offset,
                first_instance
            );
        }
        self
    }
}

impl<'a> Drop for RenderPassEncoder<'a> {
    fn drop(&mut self) {
        unsafe {
            self.parent.device.raw.cmd_end_render_pass(self.parent.raw);
        }
    }
}

impl<'a> ComputePassEncoder<'a> {
    pub fn begin(parent: &'a mut CommandBuffer) -> Self {
        Self {
            parent,
            active_pipeline: None,
        }
    }

    pub fn bind_pipeline(
        mut self,
        pipeline: &'a crate::ComputePipeline
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_bind_pipeline(
                self.parent.raw,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.raw
            );
        }
        self.active_pipeline = Some(pipeline);
        
        self
    }

    pub fn bind_descriptor_set(
        self,
        index: u32,
        set: &crate::DescriptorSet,
    ) -> Self {
        unsafe {
            self.parent.device.raw.cmd_bind_descriptor_sets(
                self.parent.raw,
                vk::PipelineBindPoint::COMPUTE,
                self.active_pipeline.unwrap().pipeline_layout,
                index,
                &[set.raw],
                &[],
            );
        }

        self
    }

    pub fn push_constants(self, offset: u32, data: &[u8]) -> Self {
        unsafe {
            self.parent.device.raw.cmd_push_constants(
                self.parent.raw,
                self.active_pipeline.unwrap().pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                offset as _,
                data,
            );
        }
        self
    }

    pub fn dispatch(self, x: u32, y: u32, z: u32) -> Self {
        unsafe {
            self.parent.device.raw.cmd_dispatch(
                self.parent.raw,
                x,
                y,
                z,
            );
        }
        self
    }
}


impl CommandBuffer {
    pub fn new(device: &Arc<super::DeviceInner>) -> Self {
        // TODO: 1 pool per command buffer for now, change this
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(device.universal_queue.family.index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .build();

        let command_pool = unsafe {
            device.raw.create_command_pool(&pool_create_info, None)
                .expect("Failed to create command pool")
        };

        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(1)
            .build();

        let command_buffer = unsafe {
            device.raw.allocate_command_buffers(&allocate_info)
                .expect("Failed to allocate command buffer")
        }[0];

        Self {
            raw: command_buffer,
            command_pool,
            device: device.clone(),
        }
    }

    /// Begins recording
    pub fn begin(&mut self) {
        unsafe {
            self.device.raw.begin_command_buffer(
                self.raw,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build()
            )
                .expect("Failed to begin command buffer");
        }
    }
    pub fn end(&mut self) {
        unsafe {
            self.device.raw.end_command_buffer(self.raw)
                .expect("Failed to end command buffer")
        }
    }

    pub fn begin_render_pass(
        &mut self,
        pass: &crate::RenderPass,
        framebuffer: &crate::Framebuffer,
        render_area: &crate::Rect<u32>
    ) -> RenderPassEncoder {
        RenderPassEncoder::begin(
            self,
            pass,
            framebuffer,
            render_area,
        )
    }

    pub fn begin_compute_pass(
        &mut self,
    ) -> ComputePassEncoder {
        ComputePassEncoder::begin(
            self,
        )
    }

    pub fn transition<'a>(
        &mut self,
        buffer_barriers: &'a [crate::BufferBarrier],
        image_barriers: &'a [crate::ImageBarrier],
        // TODO: Nicer way to handle these?
        src_stage_mask: crate::PipelineStageFlags,
        dst_stage_mask: crate::PipelineStageFlags,
    ) {
        // TODO: Transition between queues

        let buffer_memory_barriers = buffer_barriers
            .iter()
            .map(|barrier| {
                vk::BufferMemoryBarrier::builder()
                    .buffer(barrier.buffer.raw)
                    .src_access_mask(barrier.src_access_mask)
                    .dst_access_mask(barrier.dst_access_mask)
                    .offset(0)
                    .size(vk::WHOLE_SIZE) // TODO: ?
                    .build()
            })
            .collect::<Vec<_>>();

        let image_memory_barriers = image_barriers
            .iter()
            .map(|barrier| {
                vk::ImageMemoryBarrier::builder()
                    .image(barrier.image.raw)
                    .src_access_mask(barrier.src_access_mask)
                    .dst_access_mask(barrier.dst_access_mask)
                    .old_layout(barrier.old_layout)
                    .new_layout(barrier.new_layout)
                    .subresource_range(vk::ImageSubresourceRange::builder()
                        .aspect_mask(barrier.aspect_mask)
                        // TODO: Add remaining subresource range
                        .base_mip_level(0)
                        .level_count(vk::REMAINING_MIP_LEVELS)
                        .base_array_layer(0)
                        .layer_count(vk::REMAINING_ARRAY_LAYERS)
                        .build()
                    )
                    .build()
                }
            )
            .collect::<Vec<_>>();

        unsafe {
            self.device.raw.cmd_pipeline_barrier(
                self.raw,
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(),
                &[], // TODO: When to use these?
                &buffer_memory_barriers,
                &image_memory_barriers
            );
        }
    }

    pub fn copy_buffer(&mut self) { unimplemented!() }

    pub fn begin_debug_label(&self, label: &str) {
        if let Some(debug_utils) = self.device.instance.debug_utils.as_ref() {
            let label = CString::new(label).unwrap();
            let label = vk::DebugUtilsLabelEXT::builder()
                .label_name(&label)
                .build();
    
            unsafe {
                debug_utils
                    .cmd_begin_debug_utils_label(
                        self.raw,
                        &label
                    );
            }
        }
        // Else debugging not enabled, do nothing
    }
    
    pub fn end_debug_label(&self) {
        if let Some(debug_utils) = self.device.instance.debug_utils.as_ref() {
            unsafe {
                debug_utils
                    .cmd_end_debug_utils_label(
                        self.raw,
                    );
            }
        }
        // Else debugging not enabled, do nothing
    }

}

impl Drop for CommandBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_command_pool(self.command_pool, None);
        }
    }
}
