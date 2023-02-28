use anyhow::Result;
use ash::vk;

use std::collections::HashMap;
use std::sync::Arc;

/// TODO: Hashmap really necessary?
type SharedBindingDesc = Arc<HashMap<u32, vk::DescriptorSetLayoutBinding>>;

pub struct DescriptorSetLayout {
    pub(super) raw: vk::DescriptorSetLayout,
    type_count: HashMap<vk::DescriptorType, u32>,
    bindings: SharedBindingDesc,
    device: Arc<super::DeviceInner>,
}

pub struct DescriptorSet {
    pub(super) raw: vk::DescriptorSet,
    pool: vk::DescriptorPool, // TODO: No more 1 pool per set
    bindings: SharedBindingDesc,
    device: Arc<super::DeviceInner>,
}

impl DescriptorSetLayout {
    pub(super) fn new(
        device: &Arc<super::DeviceInner>,
        desc: crate::DescriptorSetLayoutDesc,
    ) -> Self {
        let mut type_count = HashMap::new();
        
        for entry in desc.entries.iter() {
            type_count
                .entry(entry.ty)
                .and_modify(|c| *c += entry.count)
                .or_insert(entry.count);
        }

        let bindings = desc.entries
            .iter()
            .map(|binding| vk::DescriptorSetLayoutBinding {
                binding: binding.binding,
                descriptor_type: binding.ty,
                descriptor_count: binding.count,
                stage_flags: binding.stage_flags,
                p_immutable_samplers: std::ptr::null(),
            })
            .collect::<Vec<_>>();

        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .flags(vk::DescriptorSetLayoutCreateFlags::empty()) // TODO:
            .bindings(&bindings)
            .build();

        let raw = unsafe {
            device.raw.create_descriptor_set_layout(&create_info, None)
                .expect("Failed to create descriptor set layout")
        };

        let bindings = bindings
            .into_iter()
            .map(|b| (b.binding, b))
            .collect::<HashMap<u32, vk::DescriptorSetLayoutBinding>>();

        Self {
            raw,
            type_count,
            bindings: Arc::new(bindings),
            device: device.clone(),
        }
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_descriptor_set_layout(self.raw, None);
        }
    }
}

impl DescriptorSet {
    pub(super) fn new(
        device: &Arc<super::DeviceInner>,
        layout: &DescriptorSetLayout,
    ) -> Self {
        let pool_sizes = layout.type_count
            .iter()
            .map(|(ty, count)| {
                vk::DescriptorPoolSize {
                    ty: *ty,
                    descriptor_count: *count,
                }
            })
            .collect::<Vec<_>>();

        // TODO: Not using 1 pool per set, this is just to get started
        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&pool_sizes)
            .max_sets(1)
            .flags(vk::DescriptorPoolCreateFlags::empty())
            .build();

        let pool = unsafe {
            device.raw.create_descriptor_pool(
                &descriptor_pool_create_info,
                None
            )
                .expect("Failed to create descriptor pool")
        };

        let raw = unsafe {
            device.raw.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(pool)
                    .set_layouts(&[layout.raw])
                    .build()
            )
                .expect("Failed to allocate descriptor set")
        }[0];
        
        Self {
            raw,
            pool,
            bindings: layout.bindings.clone(),
            device: device.clone(),
        }
    }

    pub(super) fn update<'a>(
        &self,
        entries: &[crate::Descriptor<'a>],
    ) -> Result<()> {
        let mut writes = Vec::with_capacity(entries.len());

        let mut buffer_writes = Vec::new();

        for entry in entries {
            let binding_info = match self.bindings.get(&entry.binding) {
                Some(b) => b,
                None => panic!("Binding {} not found in descriptor set", entry.binding),
            };

            // TODO: Check that binding info matches provided resource type?

            let mut write = vk::WriteDescriptorSet::builder()
                .dst_set(self.raw)
                .dst_binding(entry.binding)
                .descriptor_type(binding_info.descriptor_type);

            write = match entry.resource {
                crate::DescriptorResource::Buffer {
                    buffer,
                    offset,
                    range,
                } => {
                    let index = buffer_writes.len();

                    buffer_writes.push(
                        vk::DescriptorBufferInfo::builder()
                            .buffer(buffer.raw)
                            .offset(*offset)
                            .range(*range)
                            .build()
                    );

                    write.buffer_info(&buffer_writes[index..])
                }
            };

            writes.push(write.build());
        }

        unsafe {
            self.device.raw.update_descriptor_sets(
                &writes,
                &[],
            )
        };
        Ok(())
    }
}

impl Drop for DescriptorSet {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_descriptor_pool(self.pool, None);
        }
    }
}
