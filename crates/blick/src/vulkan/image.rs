use crate::ImageDesc;

use ash::vk;

use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub struct Image {
    pub(super) raw: vk::Image,
    pub desc: ImageDesc,
    allocation: Option<Allocation>,
    device: Arc<super::DeviceInner>,

    views: Mutex<HashMap<crate::ImageViewDesc, ImageView>>,
}

#[derive(Clone, Copy)]
pub struct ImageView {
    pub(super) raw: vk::ImageView,
}

impl Image {
    pub(super) fn new(device: &Arc<super::DeviceInner>, desc: ImageDesc) -> Self {
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(desc.image_type)
            .format(desc.format)
            .extent(desc.extent)
            .usage((&desc.usage).into())
            .tiling(vk::ImageTiling::OPTIMAL) // TODO: Will this ever change?
            .flags(vk::ImageCreateFlags::empty())
            .mip_levels(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .array_layers(1)
            .build();

        let image = unsafe {
            device.raw.create_image(&image_create_info, None)
                .expect("Failed to create image")
        };

        let memory_requirements = unsafe {
            device.raw.get_image_memory_requirements(image)
        };

        let allocation = device
            .allocator
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .allocate(
                &AllocationCreateDesc {
                    name: "image",
                    requirements: memory_requirements,
                    location: MemoryLocation::GpuOnly,
                    linear: false,
                    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                }
            )
            .expect("Failed to allocate image memory");

        unsafe {
            device.raw.bind_image_memory(image, allocation.memory(), allocation.offset())
                .expect("Failed to bind image memory")
        };

        Self {
            raw: image,
            desc,
            allocation: Some(allocation),
            device: device.clone(),
            views: Mutex::new(HashMap::new()),
        }
    }
    /// Creates a wrapper around a raw image object
    /// No cleanup will be invoked for this type of Image
    pub(super) fn from_raw(
        device: &Arc<super::DeviceInner>,
        raw: vk::Image,
        desc: ImageDesc
    ) -> Self {
        Self {
            raw,
            desc,
            allocation: None,
            device: device.clone(),
            views: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn view(&self, desc: crate::ImageViewDesc) -> crate::ImageView {
        let mut views = self.views.lock().unwrap();

        if let Some(entry) = views.get(&desc) {
            *entry
        } else {
            let view = ImageView::new(&self.device, self, desc);
            views.insert(desc, view);
            view
        }
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        for view in self.views.lock().unwrap().values() {
            unsafe {
                self.device.raw.destroy_image_view(view.raw, None)
            }
        }

        if self.allocation.is_some() {
            // Only do cleanu if we actually own the image
            self.device.allocator
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .free(self.allocation.take().unwrap())
                .expect("Failed to free buffer memory");

            unsafe {
                self.device.raw.destroy_image(self.raw, None);
            }
        }
    }
}

impl ImageView {
    pub fn new(
        device: &Arc<super::DeviceInner>,
        image: &Image,
        desc: crate::ImageViewDesc,
    ) -> Self {
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image.raw)
            .view_type(desc.view_type)
            .format(desc.format)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            })
            // TODO:
            .subresource_range(vk::ImageSubresourceRange::builder()
                .aspect_mask(desc.aspect_mask)
                .base_mip_level(desc.base_mip_level)
                .level_count(desc.level_count)
                .layer_count(1)
                .build()
            )
            .build();

        let raw = unsafe {
            device.raw.create_image_view(&image_view_create_info, None)
                .expect("Failed to create image view")
        };

        Self {
            raw,
        }
    }
}


impl From<&crate::ImageUsage> for vk::ImageUsageFlags {
    fn from(usage: &crate::ImageUsage) -> Self {
        let mut flags = vk::ImageUsageFlags::empty();

        if usage.contains(crate::ImageUsage::TRANSFER_SRC) {
            flags |= vk::ImageUsageFlags::TRANSFER_SRC;
        } else if usage.contains(crate::ImageUsage::TRANSFER_DST) {
            flags |= vk::ImageUsageFlags::TRANSFER_DST;
        } else if usage.contains(crate::ImageUsage::SAMPLED) {
            flags |= vk::ImageUsageFlags::SAMPLED;
        } else if usage.contains(crate::ImageUsage::STORAGE) {
            flags |= vk::ImageUsageFlags::STORAGE;
        } else if usage.contains(crate::ImageUsage::COLOR_ATTACHMENT) {
            flags |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
        } else if usage.contains(crate::ImageUsage::DEPTH_STENCIL_ATTACHMENT) {
            flags |= vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        }

        flags
    }
}


