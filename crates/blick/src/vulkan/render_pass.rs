use crate::ColorAttachmentDesc;

use ash::vk;

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};


#[derive(Clone, Default, Eq, Hash, PartialEq)]
pub struct RenderPassKey {
    pub color_attachments: Vec<Option<ColorAttachmentDesc>>,
}

struct RenderPassInner {
    raw: vk::RenderPass,
    num_attachments: u32,
    device: Arc<super::DeviceInner>,
}


pub struct RenderPassCache {
    cache: Mutex<LruCache<RenderPassKey, Arc<RenderPassInner>>>,
    device: Arc<super::DeviceInner>,
}

pub struct RenderPass {
    inner: Arc<RenderPassInner>,
}

impl RenderPass {
    pub fn raw(&self) -> vk::RenderPass {
        self.inner.raw
    }
    pub fn num_attachments(&self) -> u32 {
        self.inner.num_attachments
    }
}

impl RenderPassCache {
    /// device: Device for destroying render passes
    pub fn new(size: usize, device: &Arc<super::DeviceInner>) -> Self {
        Self {
            cache: Mutex::new(LruCache::new(
                NonZeroUsize::new(size).unwrap()
            )),
            device: device.clone(),
        }
    }

    pub fn get_or_create(
        &self,
        desc: crate::RenderPassDesc<'_>
    ) -> RenderPass {
        let key = RenderPassKey::from(&desc);
        
        RenderPass {
            inner:
                self.cache.lock().unwrap().get_or_insert(
                    key,
                    || Arc::new(RenderPassInner::new(&self.device, desc))
                ).clone()
        }
    }
}

impl RenderPassInner {
    fn new(device: &Arc<super::DeviceInner>, desc: crate::RenderPassDesc<'_>) -> Self {
        let mut attachments = Vec::new();
        let mut color_refs = Vec::new();

        for color_attachment in desc.color_attachments.iter() {
            if let Some(color_attachment) = color_attachment {
                attachments.push(
                    vk::AttachmentDescription::builder()
                        .format(color_attachment.format)
                        // TODO:
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        // OK since we clear the image anyway, have to change if we don't
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .final_layout(color_attachment.layout)
                        .build()
                );
                
                color_refs.push(
                    vk::AttachmentReference::builder()
                        .attachment(attachments.len() as u32 - 1)
                        .layout(color_attachment.layout)
                        .build()
                );
            } else {
                color_refs.push(
                    vk::AttachmentReference::builder()
                        .attachment(vk::ATTACHMENT_UNUSED)
                        .layout(vk::ImageLayout::UNDEFINED)
                        .build()
                );
            }
        }

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&color_refs)
            .build();

        let render_pass_create_info = vk::RenderPassCreateInfo::builder()
            .attachments(&attachments)
            .subpasses(&[subpass])
            .build();

        let raw = unsafe {
            device.raw
                .create_render_pass(&render_pass_create_info, None)
                .expect("Failed to create render pass")
        };

        Self {
            raw,
            num_attachments: attachments.len() as u32,
            device: device.clone(),
        }
    }
}

impl Drop for RenderPassInner {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_render_pass(self.raw, None);
        }
    }
}

impl<'a> From<&crate::RenderPassDesc<'a>> for RenderPassKey {
    fn from(desc: &crate::RenderPassDesc) -> Self {
        let mut key = Self {
            ..Default::default()
        };

        desc.color_attachments
            .iter()
            .for_each(|attachment| {
                key.color_attachments.push(*attachment);
            });
        
        key
    }
}

