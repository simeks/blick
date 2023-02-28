mod vulkan;

use ash::vk;
use std::sync::Arc;

pub use vulkan::Backend;
pub use vulkan::CommandBuffer;
pub use vulkan::DescriptorSetLayout;
pub use vulkan::Device;
pub use vulkan::Fence;
pub use vulkan::Framebuffer;
pub use vulkan::ImageView;
pub use vulkan::RenderPass;
pub use vulkan::Semaphore;
pub use vulkan::{ComputePassEncoder, RenderPassEncoder};

pub type Buffer = Arc<vulkan::Buffer>;
pub type Image = Arc<vulkan::Image>;
pub type GraphicsPipeline = Arc<vulkan::GraphicsPipeline>;
pub type ComputePipeline = Arc<vulkan::ComputePipeline>;
pub type DescriptorSet = Arc<vulkan::DescriptorSet>;

pub const MAX_COLOR_ATTACHMENTS: usize = 8;
pub const WHOLE_SIZE: u64 = vk::WHOLE_SIZE;

pub struct BackendConfig {
    pub debugging: bool,
}

// If we ever decide to abstract away vulkan
pub type Extent2d = vk::Extent2D;
pub type Extent3d = vk::Extent3D;

pub type ImageAspectFlags = vk::ImageAspectFlags;
pub type ImageFormat = vk::Format;
pub type ImageLayout = vk::ImageLayout;
pub type ImageType = vk::ImageType;
pub type ImageViewType = vk::ImageViewType;

pub type AccessFlags = vk::AccessFlags;
pub type DescriptorType = vk::DescriptorType;

pub type IndexType = vk::IndexType;

pub type PipelineBindPoint = vk::PipelineBindPoint;
pub type PipelineStageFlags = vk::PipelineStageFlags;
pub type ShaderStageFlags = vk::ShaderStageFlags;

pub type AttachmentLoadOp = vk::AttachmentLoadOp;
pub type AttachmentStoreOp = vk::AttachmentStoreOp;

#[derive(Debug, Clone, Copy)]
pub struct Rect<T> {
    pub x: T,
    pub y: T,
    pub width: T,
    pub height: T,
}

bitflags::bitflags! {
    pub struct BufferUsage: u32 {
        /// Enable buffer to be mapped for reading
        /// Mainly used for staging buffers
        const MAP_READ = 1 << 0;
        /// Enable buffer to be mapped for writing
        /// Mainly used for staging buffers
        const MAP_WRITE = 1 << 1;

        /// Enable copy from buffer
        const TRANSFER_SRC = 1 << 2;
        /// Enable copy to buffer
        const TRANSFER_DST = 1 << 3;
        
        /// Enable use as a uniform buffer
        const UNIFORM = 1 << 4;
        /// Enable use as storage buffer
        const STORAGE = 1 << 5;
        /// Enable use as index buffer
        const INDEX = 1 << 6;
        /// Enable use as vertex buffer
        const VERTEX = 1 << 7;
        /// Enable use as indirect buffer
        const INDIRECT = 1 << 8;
    }
}

bitflags::bitflags! {
    pub struct ImageUsage: u32 {
        const TRANSFER_SRC = 1 << 0;
        const TRANSFER_DST = 1 << 1;
        const SAMPLED = 1 << 2;
        const STORAGE = 1 << 3;

        // TODO: Might be able to combine these
        const COLOR_ATTACHMENT = 1 << 4;
        const DEPTH_STENCIL_ATTACHMENT = 1 << 5;
    }
}

pub struct BufferDesc {
    pub size: u64,
    pub usage: BufferUsage,
}

pub struct ImageDesc {
    pub image_type: ImageType,
    pub format: ImageFormat,
    pub extent: Extent3d,
    pub usage: ImageUsage
}

#[derive(Copy, Clone, Default, Eq, Hash, PartialEq)]
pub struct ImageViewDesc {
    pub view_type: ImageViewType,
    pub aspect_mask: ImageAspectFlags,
    // TODO: Is format needed or can we just use the image's format?
    pub format: ImageFormat,
    pub base_mip_level: u32,
    pub level_count: u32,
}

pub enum DescriptorResource<'a> {
    Buffer {
        buffer: &'a Buffer,
        offset: u64,
        range: u64,
    },
}

pub struct Descriptor<'a> {
    pub binding: u32,
    pub resource: &'a DescriptorResource<'a>,
}

pub struct DescriptorSetLayoutEntry {
    pub binding: u32,
    pub stage_flags: ShaderStageFlags,
    pub ty: DescriptorType,
    pub count: u32,
}

pub struct DescriptorSetLayoutDesc<'a> {
    pub entries: &'a [DescriptorSetLayoutEntry],
}

pub enum ShaderSource<'a> {
    Hlsl(&'a str),
}

pub struct ShaderModuleDesc<'a> {
    pub source: ShaderSource<'a>,
    pub stage: ShaderStageFlags,
}

pub struct PushConstantRange {
    pub stage_flags: ShaderStageFlags,
    pub offset: u32,
    pub size: u32,
}


#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ColorAttachmentDesc {
    pub format: ImageFormat,
    pub layout: ImageLayout,
}

pub struct RenderPassDesc<'a> {
    pub color_attachments: &'a [Option<ColorAttachmentDesc>],
}

pub struct Attachment<'a> {
    pub image_view: &'a ImageView,
}

pub struct FramebufferDesc<'a> {
    pub render_pass: &'a RenderPass,
    pub attachments: &'a [Attachment<'a>],
    pub extent: Extent2d,
}

/// TODO: Are there any point to creating shader modules separately?
/// TODO: Maybe this could be general for both graphics and compute?
pub struct GraphicsPipelineDesc<'a> {
    pub shader_modules: &'a [ShaderModuleDesc<'a>],
    pub descriptor_set_layouts: &'a [&'a DescriptorSetLayout],
    pub push_constant_ranges: &'a [PushConstantRange],
    pub render_pass: &'a RenderPass,
}

pub struct ComputePipelineDesc<'a> {
    pub shader_module: ShaderModuleDesc<'a>,
    pub descriptor_set_layouts: &'a [&'a DescriptorSetLayout],
    pub push_constant_ranges: &'a [PushConstantRange],
}


pub struct BufferBarrier<'a> {
    pub buffer: &'a Buffer,
    pub src_access_mask: AccessFlags,
    pub dst_access_mask: AccessFlags,
}

pub struct ImageBarrier<'a> {
    pub image: &'a Image,
    pub src_access_mask: AccessFlags,
    pub dst_access_mask: AccessFlags,
    pub old_layout: ImageLayout,
    pub new_layout: ImageLayout,
    pub aspect_mask: ImageAspectFlags, // TODO: Make proper subresource range
}

#[derive(Debug)]
pub enum BeginFrameError {
    OutdatedSwapchain,
}

#[derive(Debug)]
pub enum EndFrameError {
    OutdatedSwapchain,
}
