use anyhow::Result;
use ash::vk;

use std::ffi::CString;
use std::sync::Arc;

pub struct GraphicsPipeline {
    pub(super) raw: vk::Pipeline,
    pub(super) pipeline_layout: vk::PipelineLayout,
    device: Arc<super::DeviceInner>,
}

pub struct ComputePipeline {
    pub(super) raw: vk::Pipeline,
    #[allow(dead_code)]
    pub(super) pipeline_layout: vk::PipelineLayout,
    device: Arc<super::DeviceInner>,
}


impl GraphicsPipeline {
    pub(super) fn new(
        device: &Arc<super::DeviceInner>,
        desc: crate::GraphicsPipelineDesc,
    ) -> Self {
        let entry_name = CString::new("main").unwrap();

        let shader_stage_create_infos = desc.shader_modules
            .iter()
            .map(|module_desc| {
                let shader_module = create_shader_module(
                    device,
                    module_desc,
                )
                    .expect("Failed to create shader module");

                vk::PipelineShaderStageCreateInfo::builder()
                    .stage(module_desc.stage)
                    .module(shader_module)
                    .name(&entry_name)
                    .build()
            })
            .collect::<Vec<_>>();

        let pipeline_layout = create_pipeline_layout(
            device,
            desc.descriptor_set_layouts,
            desc.push_constant_ranges
        )
            .expect("Failed to create pipeline layout");

        // We only do bindless vertex buffers
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .build();
    
        let vertex_input_assembly_state_create_info =
            vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        // TODO: Allow changing of state parameters

        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1)
            .build();
    
        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .polygon_mode(vk::PolygonMode::FILL)
            .rasterizer_discard_enable(false)
            .line_width(1.0)
            .depth_bias_clamp(0.0)
            .depth_bias_constant_factor(0.0)
            .depth_bias_enable(false)
            .depth_bias_slope_factor(0.0)
            .build();
    
        let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .sample_shading_enable(false)
            .min_sample_shading(0.0)
            .build();
    
        let stencil_state = vk::StencilOpState::builder()
            .fail_op(vk::StencilOp::KEEP)
            .pass_op(vk::StencilOp::KEEP)
            .depth_fail_op(vk::StencilOp::KEEP)
            .compare_op(vk::CompareOp::ALWAYS)
            .build();
    
        let depth_stencil_state_create_info = vk::PipelineDepthStencilStateCreateInfo::builder()
            .depth_test_enable(false)
            .depth_write_enable(false)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .front(stencil_state)
            .back(stencil_state)
            .max_depth_bounds(1.0)
            .build();

        let color_blend_state_create_infos = (0..desc.render_pass.num_attachments())
            .map(|_| {
                vk::PipelineColorBlendAttachmentState::builder()
                    .blend_enable(false)
                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                    .src_color_blend_factor(vk::BlendFactor::ONE)
                    .dst_color_blend_factor(vk::BlendFactor::ZERO)
                    .color_blend_op(vk::BlendOp::ADD)
                    .src_alpha_blend_factor(vk::BlendFactor::ONE)
                    .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                    .alpha_blend_op(vk::BlendOp::ADD)
                    .build()
            })
            .collect::<Vec<_>>();


        // let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
        //     .color_attachment_formats(&[vk::Format::B8G8R8A8_SRGB]) // TODO
        //     .build();

        let graphics_pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&vertex_input_assembly_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .rasterization_state(&rasterization_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .color_blend_state(&vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&color_blend_state_create_infos)
                .build())
            .dynamic_state(&vk::PipelineDynamicStateCreateInfo::builder()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
                .build())
            .layout(pipeline_layout)
            //.push_next(&mut pipeline_rendering_create_info)
            .render_pass(desc.render_pass.raw())
            .build();

        let raw = unsafe {
            device.raw
                .create_graphics_pipelines(
                    // TODO: ?
                    vk::PipelineCache::null(),
                    &[graphics_pipeline_create_info],
                    None
                )
                .expect("Failed to create graphics pipeline")
        }[0];

        // Pipeline is complete, now we can cleanup shader modules
        shader_stage_create_infos
            .iter()
            .for_each(|info| unsafe {
                device.raw.destroy_shader_module(info.module, None)
            });

        Self {
            raw,
            pipeline_layout,
            device: device.clone(),
        }
    }
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.raw.destroy_pipeline(self.raw, None);
        }
    }
}

impl ComputePipeline {
    pub fn new(
        device: &Arc<super::DeviceInner>,
        desc: crate::ComputePipelineDesc,
    ) -> Self {
        let entry_name = CString::new("main").unwrap();

        let shader_module = create_shader_module(
            device,
            &desc.shader_module,
        )
            .expect("Failed to create shader module");

        let shader_stage_create_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(&entry_name)
            .build();

        let pipeline_layout = create_pipeline_layout(
            device,
            desc.descriptor_set_layouts,
            desc.push_constant_ranges
        )
            .expect("Failed to create pipeline layout");

        let compute_pipeline_create_info = vk::ComputePipelineCreateInfo::builder()
            .layout(pipeline_layout)
            .stage(shader_stage_create_info)
            .build();

        let raw = unsafe {
            device.raw
                .create_compute_pipelines(
                    vk::PipelineCache::null(),
                    &[compute_pipeline_create_info],
                    None
                )
                .expect("Failed to create compute pipeline")
        }[0];

        unsafe {
            device.raw.destroy_shader_module(shader_module, None)
        };

        Self {
            raw,
            pipeline_layout,
            device: device.clone(),
        }
    }
}


impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.raw.destroy_pipeline(self.raw, None);
        }
    }
}

fn create_shader_module(
    device: &Arc<super::DeviceInner>,
    desc: &crate::ShaderModuleDesc
) -> Result<vk::ShaderModule> {

    let spirv = match desc.source {
        crate::ShaderSource::Hlsl(src) => {
            let target_profile = match desc.stage {
                crate::ShaderStageFlags::VERTEX => "vs_6_4",
                crate::ShaderStageFlags::FRAGMENT => "ps_6_4",
                crate::ShaderStageFlags::COMPUTE => "cs_6_4",
                _ => unimplemented!(),
            };

            compile_hlsl(
                "shader", // TODO: ?
                src,
                "main",
                target_profile,
            )?
        }
    };

    // Builder requires conversion Vec<u8> -> &[u32] (and then back to ptr)
    let create_info = vk::ShaderModuleCreateInfo {
        s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
        p_next: std::ptr::null(),
        flags: vk::ShaderModuleCreateFlags::empty(),
        code_size: spirv.len() as usize,
        p_code: spirv.as_ptr() as *const u32,
    };

    let module = unsafe {
        device.raw.create_shader_module(&create_info, None)?
    };

    Ok(module)
}

/// Compiles hlsl shader to spirv
/// This will probably be moved to some asset system later
fn compile_hlsl(
    name: &str,
    source: &str,
    entry: &str,
    target_profile: &str,
) -> Result<Vec<u8>> {
    let spirv = hassle_rs::compile_hlsl(
        name,
        source,
        entry,
        target_profile,
        &[
            "-spirv",
            "-enable-templates",
            "-fspv-target-env=vulkan1.2",
            "-WX",  // warnings as errors
            "-Ges", // strict mode
        ],
        &[],
    )?;

    Ok(spirv)
}

fn create_pipeline_layout(
    device: &Arc<super::DeviceInner>,
    descriptor_set_layouts:  &[&crate::DescriptorSetLayout],
    push_constant_ranges:  &[crate::PushConstantRange],
) -> Result<vk::PipelineLayout> {
    let descriptor_set_layouts = descriptor_set_layouts
        .iter()
        .map(|layout| layout.raw)
        .collect::<Vec<_>>();

    let push_constant_ranges = push_constant_ranges
        .iter()
        .map(|range| range.into())
        .collect::<Vec<vk::PushConstantRange>>();

    let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(&descriptor_set_layouts)
        .push_constant_ranges(&push_constant_ranges)
        .build();

    match unsafe {
        device.raw
            .create_pipeline_layout(&pipeline_layout_create_info, None)
    } {
        Ok(layout) => Ok(layout),
        Err(e) => Err(e.into()),
    }
}

impl From<&crate::PushConstantRange> for vk::PushConstantRange {
    fn from(range: &crate::PushConstantRange) -> Self {
        vk::PushConstantRange {
            stage_flags: range.stage_flags,
            offset: range.offset,
            size: range.size,
        }
    }
}
