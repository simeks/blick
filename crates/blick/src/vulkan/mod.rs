mod backend;
mod buffer;
mod command;
mod descriptor;
mod device;
mod framebuffer;
mod image;
mod instance;
mod render_pass;
mod shader;
mod surface;
mod swapchain;
mod sync;

pub use backend::Backend;
pub use buffer::Buffer;
pub use command::{CommandBuffer, ComputePassEncoder, RenderPassEncoder};
pub use descriptor::{DescriptorSet, DescriptorSetLayout};
pub use device::{Device, DeviceInner};
pub use framebuffer::Framebuffer;
pub use image::{Image, ImageView};
pub use instance::Instance;
pub use instance::PhysicalDevice;
pub use render_pass::RenderPass;
pub use shader::{ComputePipeline, GraphicsPipeline};
pub use surface::Surface;
pub use swapchain::{Swapchain, SwapchainDesc};
pub use sync::{Fence, Semaphore};

use ash::vk;

use std::ffi::CStr;
use std::os::raw::c_char;


#[derive(Copy, Clone)]
pub struct QueueFamily {
    pub index: u32,
    pub properties: vk::QueueFamilyProperties,
}

pub struct Queue {
    pub raw: vk::Queue,
    pub family: QueueFamily,
}

/// Converts a C string (as provided by vulkan) to a Rust string.
pub fn vk_to_string(raw_string : &[c_char]) -> String {
    unsafe {
        CStr::from_ptr(raw_string.as_ptr())
    }.to_str().expect("Failed to convert string").to_owned()
}
