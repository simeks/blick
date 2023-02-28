use ash::vk;

use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default, Eq, Hash, PartialEq)]
struct FramebufferKey {
    attachments: Vec<vk::ImageView>,
    render_pass: vk::RenderPass,
    width: u32,
    height: u32,
    layers: u32,
}

/// TODO: What if render pass gets destroyed first? (shouldn't happen)
struct FramebufferInner {
    raw: vk::Framebuffer,
    device: Arc<super::DeviceInner>,
}

pub struct Framebuffer {
    inner: Arc<FramebufferInner>,
}

pub struct FramebufferCache {
    cache: Mutex<LruCache<FramebufferKey, Arc<FramebufferInner>>>,
    device: Arc<super::DeviceInner>,
}

impl Framebuffer {
    pub fn raw(&self) -> vk::Framebuffer {
        self.inner.raw
    }
}

impl Drop for FramebufferInner {
    fn drop(&mut self) {
        unsafe {
            self.device.raw.destroy_framebuffer(self.raw, None);
        }
    }
}

impl FramebufferCache {
    /// device: Device for destroying framebuffers
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
        desc: crate::FramebufferDesc<'_>
    ) -> Framebuffer {
        let key = FramebufferKey::from(&desc);
        
        Framebuffer {
            inner:
                self.cache.lock().unwrap().get_or_insert(
                    key,
                    || Arc::new(FramebufferInner::new(&self.device, &desc))
                ).clone(),
        }
    }
}

impl FramebufferInner {
    fn new<'a>(
        device: &Arc<super::DeviceInner>,
        desc: &crate::FramebufferDesc<'a>
    ) -> Self {
        let attachments = desc.attachments
            .iter()
            .map(|a| a.image_view.raw)
            .collect::<Vec<_>>();

        let raw = unsafe {
            device.raw.create_framebuffer(
                &vk::FramebufferCreateInfo::builder()
                    .render_pass(desc.render_pass.raw())
                    .attachments(&attachments)
                    .width(desc.extent.width)
                    .height(desc.extent.height)
                    .layers(1),
                None
            ).expect("Failed to create framebuffer")
        };
        
        Self {
            raw,
            device: device.clone(),
        }
    }
}

impl From<&crate::FramebufferDesc<'_>> for FramebufferKey {
    fn from(desc: &crate::FramebufferDesc<'_>) -> Self {
        Self {
            attachments: desc.attachments
                .iter()
                .map(|a| a.image_view.raw)
                .collect::<Vec<_>>(),
            render_pass: desc.render_pass.raw(),
            width: desc.extent.width,
            height: desc.extent.height,
            layers: 1,
        }
    }
}
