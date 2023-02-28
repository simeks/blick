use super::{Instance, PhysicalDevice, Queue};

use anyhow::Result;

use ash::extensions::khr;
use ash::vk;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::KhrPortabilitySubsetFn;

use gpu_allocator::AllocatorDebugSettings;
use gpu_allocator::vulkan::{Allocator, AllocatorCreateDesc};

use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, Mutex};

// TODO:
const RENDER_PASS_CACHE_SIZE: usize = 16;
const FRAMEBUFFER_CACHE_SIZE: usize = 16;

pub struct DeviceInner {
    pub(super) raw: ash::Device,
    pub(super) instance: Arc<super::Instance>,
    pub(super) physical_device: PhysicalDevice,
    pub(super) allocator: Option<Arc<Mutex<Allocator>>>,
    /// TODO: Single queue for everything for now, change this?
    pub(super) universal_queue: Queue,
}

pub struct Device {
    pub(crate) inner: Arc<DeviceInner>,

    render_pass_cache: super::render_pass::RenderPassCache,
    framebuffer_cache: super::framebuffer::FramebufferCache,
}

impl Drop for DeviceInner {
    fn drop(&mut self) {
        // TODO: Couldn't this result in a lot of headaches if the device
        // only gets dropped after all resources have been dropped? Maybe
        // explicit drop is better and then report any leaked resources

        // Let device finish any pending work
        unsafe { self.raw.device_wait_idle().unwrap() };

        // Destroy allocator
        self.allocator.take().unwrap();

        unsafe {
            self.raw.destroy_device(None);
        }
    }
}

impl Device {
    pub(crate) fn new(
        instance: &Arc<Instance>,
        physical_device: PhysicalDevice,
        config: &crate::BackendConfig,
    ) -> Result<Self> {
        let enabled_extension_names = vec![
            khr::Swapchain::name().as_ptr(),
            //vk::KhrDynamicRenderingFn::name().as_ptr(),
            //vk::KhrShaderNonSemanticInfoFn::name().as_ptr(),
            vk::ExtDescriptorIndexingFn::name().as_ptr(),
            vk::KhrBufferDeviceAddressFn::name().as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            KhrPortabilitySubsetFn::name().as_ptr(),
        ];

        // TODO: For now we just create a single queue using first available graphics 
        //      compatible family
        let universal_queue_family = if let Some(universal_queue) = 
            physical_device.queue_families
                .iter()
                .find(|q| q.properties.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        {
            *universal_queue
        } else {
            anyhow::bail!("No graphics queue family found")
        };

        let queue_priorities = [1.0_f32];
        let queue_create_info = [
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(universal_queue_family.index)
                .queue_priorities(&queue_priorities)
                .build()
        ];

        let enabled_layer_names = if config.debugging {
            vec![
                CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
            ]
        } else {
            vec![]
        };
        
        let enabled_layer_names : Vec<*const c_char> = enabled_layer_names
            .iter()
            .map(|name| name.as_ptr())
            .collect();

        let supported_extensions: HashSet<String> = unsafe {
            let properties = instance
                .raw
                .enumerate_device_extension_properties(physical_device.raw)?;

            properties
                .iter()
                .map(|ext| {
                    super::vk_to_string(&ext.extension_name)
                })
                .collect()
        };

        unsafe {
            for &ext in &enabled_extension_names {
                let ext = CStr::from_ptr(ext)
                    .to_str()
                    .unwrap();
                if !supported_extensions.contains(ext) {
                    return Err(anyhow::anyhow!("Extension {} not supported", ext));
                }
            }
        }

        let mut descriptor_indexing
            = vk::PhysicalDeviceDescriptorIndexingFeatures::default();
        let mut buffer_device_address
            = vk::PhysicalDeviceBufferDeviceAddressFeatures::default();
        let mut dynamic_rendering
            = vk::PhysicalDeviceDynamicRenderingFeatures::default();

        let mut features2 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut descriptor_indexing)
            .push_next(&mut buffer_device_address)
            .push_next(&mut dynamic_rendering)
            .build();

        unsafe {
            // Fills in available features of our device
            instance.raw
                .get_physical_device_features2(
                    physical_device.raw,
                    &mut features2
                )
        };

        // TODO: Check that necessary features are available.

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_info)
            .enabled_layer_names(&enabled_layer_names)
            .enabled_extension_names(&enabled_extension_names)
            .push_next(&mut features2)
            .build();

        let device = unsafe {
            instance.raw.create_device(
                physical_device.raw,
                &device_create_info,
                None
            )
                .expect("Failed to create logical device")
        };

        let allocator = Allocator::new(
            &AllocatorCreateDesc {
                instance: instance.raw.clone(),
                device: device.clone(),
                physical_device: physical_device.raw,
                debug_settings: AllocatorDebugSettings {
                    log_leaks_on_shutdown: config.debugging,
                    log_memory_information: config.debugging,
                    log_allocations: config.debugging,
                    ..Default::default()
                },
                buffer_device_address: true
            }
        )
            .expect("Failed to create allocator");

        let universal_queue = unsafe {
            device.get_device_queue(universal_queue_family.index, 0)
        };

        let inner = Arc::new(
            DeviceInner {
                raw: device,
                instance: instance.clone(),
                physical_device,
                allocator: Some(Arc::new(Mutex::new(allocator))),
                universal_queue: Queue {
                    raw: universal_queue,
                    family: universal_queue_family,
                },
            }
        );

        Ok(
            Self {
                inner: inner.clone(),
                render_pass_cache: super::render_pass::RenderPassCache::new(
                    RENDER_PASS_CACHE_SIZE,
                    &inner,
                ),
                framebuffer_cache: super::framebuffer::FramebufferCache::new(
                    FRAMEBUFFER_CACHE_SIZE,
                    &inner,
                )
            }
        )
    }

    pub fn create_fence(&self) -> Result<crate::Fence> {
        // TODO: Translate error?
        Ok(super::Fence::new(&self.inner))
    }

    pub fn create_semaphore(&self) -> Result<crate::Semaphore> {
        Ok(super::Semaphore::new(&self.inner))
    }

    pub fn create_buffer(&self, desc: crate::BufferDesc) -> Result<crate::Buffer> {
        Ok(Arc::new(super::Buffer::new(&self.inner, desc)))
    }

    pub fn create_image(&self, desc: crate::ImageDesc) -> Result<crate::Image> {
        Ok(Arc::new(super::Image::new(&self.inner, desc)))
    }

    pub fn create_image_view(
        &self,
        image: &crate::Image,
        desc: crate::ImageViewDesc
    ) -> Result<crate::ImageView> {
        Ok(image.view(desc))
    }

    pub fn create_descriptor_set_layout(
        &self,
        desc: crate::DescriptorSetLayoutDesc<'_>
    ) -> Result<crate::DescriptorSetLayout> {
        Ok(super::DescriptorSetLayout::new(&self.inner, desc))
    }

    pub fn create_descriptor_set(
        &self,
        layout: &crate::DescriptorSetLayout
    ) -> Result<crate::DescriptorSet> {
        Ok(Arc::new(super::DescriptorSet::new(&self.inner, layout)))
    }

    pub fn update_descriptor_set(
        &self,
        set: &crate::DescriptorSet,
        entries: &[crate::Descriptor<'_>],
    ) -> Result<()> {
        set.update(entries)
    }

    /// Render passes are cached in the backend based on their desc
    pub fn create_render_pass(
        &self,
        desc: crate::RenderPassDesc<'_>,
    ) -> Result<crate::RenderPass> {
        Ok(self.render_pass_cache.get_or_create(
            desc
        ))
    }

    pub fn create_framebuffer(
        &self,
        desc: crate::FramebufferDesc<'_>,
    ) -> Result<crate::Framebuffer> {
        Ok(self.framebuffer_cache.get_or_create(
            desc
        ))
    }


    pub fn create_graphics_pipeline(
        &self,
        desc: crate::GraphicsPipelineDesc,
    ) -> Result<crate::GraphicsPipeline> {
        Ok(Arc::new(super::GraphicsPipeline::new(&self.inner, desc)))
    }

    pub fn create_compute_pipeline(
        &self,
        desc: crate::ComputePipelineDesc,
    ) -> Result<crate::ComputePipeline> {
        Ok(Arc::new(super::ComputePipeline::new(&self.inner, desc)))
    }


    pub fn create_command_buffer(&self) -> Result<crate::CommandBuffer> {
        Ok(super::CommandBuffer::new(&self.inner))
    }

    pub fn submit(
        &self,
        command_buffers: &[&crate::CommandBuffer],
        wait_semaphores: &[&crate::Semaphore],
        signal_semaphores: &[&crate::Semaphore],
        fence: Option<&crate::Fence>,
    ) -> Result<()> {
        let command_buffers = command_buffers
            .iter()
            .map(|cb| cb.raw)
            .collect::<Vec<_>>();

        let wait_semaphores = wait_semaphores
            .iter()
            .map(|sem| sem.raw)
            .collect::<Vec<_>>();

        let signal_semaphores = signal_semaphores
            .iter()
            .map(|sem| sem.raw)
            .collect::<Vec<_>>();

        let fence = fence
            .map(|fence| fence.raw)
            .unwrap_or(vk::Fence::null());

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .wait_semaphores(&wait_semaphores)
            .signal_semaphores(&signal_semaphores)
            .wait_dst_stage_mask(&[])
            .build();

        unsafe {
            self.inner.raw
                .queue_submit(
                    self.inner.universal_queue.raw,
                    &[submit_info],
                    fence
                )
                .expect("Failed to submit command buffer");
        }

        Ok(())
    }

    pub fn wait(&self, fence: &crate::Fence) -> Result<()> {
        unsafe {
            self.inner.raw
                .wait_for_fences(
                    &[fence.raw],
                    true,
                    u64::MAX
                )
                .expect("Failed to wait for fence");
        }
        Ok(())
    }

    pub fn wait_idle(&self) -> Result<()> {
        unsafe {
            self.inner.raw.device_wait_idle()?
        }
        Ok(())
    }

    pub fn reset_fence(&self, fence: &crate::Fence) -> Result<()> {
        unsafe {
            self.inner.raw
                .reset_fences(&[fence.raw])
                .expect("Failed to reset fence");
        }
        Ok(())
    }
}
