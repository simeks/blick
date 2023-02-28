use ash::vk;
use std::sync::Arc;

pub struct Fence {
    pub(super) raw: vk::Fence,
    device: Arc<super::DeviceInner>,
}

impl Fence {
    pub(super) fn new(device: &Arc<super::DeviceInner>) -> Self {
        let fence_create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::empty())
            .build();

        let raw = unsafe {
            device.raw.create_fence(&fence_create_info, None)
                .expect("Failed to create fence")
        };

        Self {
            raw,
            device: device.clone(),
        }
    }
}
impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_fence(self.raw, None)
        }
    }
}

pub struct Semaphore {
    pub(super) raw: vk::Semaphore,
    device: Arc<super::DeviceInner>,
}

impl Semaphore {
    pub(super) fn new(device: &Arc<super::DeviceInner>) -> Self {
        let semaphore_create_info = vk::SemaphoreCreateInfo::builder()
            .flags(vk::SemaphoreCreateFlags::empty())
            .build();

        let raw = unsafe {
            device.raw.create_semaphore(&semaphore_create_info, None)
                .expect("Failed to create semaphore")
        };

        Self {
            raw,
            device: device.clone(),
        }
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_semaphore(self.raw, None)
        }
    }
}
