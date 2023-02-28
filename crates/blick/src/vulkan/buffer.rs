use crate::BufferDesc;

use anyhow::Result;
use ash::vk;

use gpu_allocator::MemoryLocation;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};

use std::sync::Arc;

pub struct Buffer {
    pub(super) raw: vk::Buffer,
    allocation: Option<Allocation>,
    device: Arc<super::DeviceInner>,
}

impl Buffer {
    pub(super) fn new(device: &Arc<super::DeviceInner>, desc: BufferDesc) -> Self {
        let buffer_create_info = vk::BufferCreateInfo::builder()
            .size(desc.size)
            .usage((&desc.usage).into())
            .sharing_mode(vk::SharingMode::EXCLUSIVE) // TODO: Always exclusive?
            .build();

        let buffer = unsafe {
            device.raw.create_buffer(&buffer_create_info, None)
                .expect("Failed to create buffer")
        };

        let memory_requirements = unsafe {
            device.raw.get_buffer_memory_requirements(buffer)
        };

        let allocation = device.allocator
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .allocate(
                &AllocationCreateDesc {
                    name: "buffer",
                    requirements: memory_requirements,
                    location: MemoryLocation::from(&desc.usage),
                    linear: true,
                    allocation_scheme: AllocationScheme::GpuAllocatorManaged,
                }
            )
            .expect("Failed to allocate buffer memory");

        unsafe {
            device.raw.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .expect("Failed to bind buffer memory")
        };

        Self {
            raw: buffer,
            allocation: Some(allocation),
            device: device.clone(),
        }
    }
    pub fn mapped_ptr<T>(&self) -> Result<*mut T> {
        Ok(
            self.allocation
                .as_ref()
                .unwrap()
                .mapped_ptr()
                .unwrap()
                .as_ptr() as *mut _
        )
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.device.allocator
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .free(self.allocation.take().unwrap())
            .expect("Failed to free buffer memory");

        unsafe {
            self.device.raw.destroy_buffer(self.raw, None);
        }
    }
}

impl From<&crate::BufferUsage> for MemoryLocation {
    fn from(usage: &crate::BufferUsage) -> Self {
        if usage.contains(crate::BufferUsage::MAP_READ) {
            MemoryLocation::GpuToCpu
        } else if usage.contains(crate::BufferUsage::MAP_WRITE) {
            MemoryLocation::CpuToGpu
        } else {
            MemoryLocation::GpuOnly
        }
    }
}

impl From<&crate::BufferUsage> for vk::BufferUsageFlags {
    fn from(usage: &crate::BufferUsage) -> Self {
        let mut flags = vk::BufferUsageFlags::empty();

        if usage.contains(crate::BufferUsage::TRANSFER_SRC) {
            flags |= vk::BufferUsageFlags::TRANSFER_SRC;
        } else if usage.contains(crate::BufferUsage::TRANSFER_DST) {
            flags |= vk::BufferUsageFlags::TRANSFER_DST;
        } else if usage.contains(crate::BufferUsage::UNIFORM) {
            flags |= vk::BufferUsageFlags::UNIFORM_BUFFER;
        } else if usage.contains(crate::BufferUsage::STORAGE) {
            flags |= vk::BufferUsageFlags::STORAGE_BUFFER;
        } else if usage.contains(crate::BufferUsage::INDEX) {
            flags |= vk::BufferUsageFlags::INDEX_BUFFER;
        } else if usage.contains(crate::BufferUsage::VERTEX) {
            flags |= vk::BufferUsageFlags::VERTEX_BUFFER;
        }

        flags
    }
}
