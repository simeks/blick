use anyhow::Result;
use ash::extensions::khr;
use ash::vk;

use std::sync::Arc;

pub enum SwapchainError {
    Outdated,
}

#[derive(Debug)]
pub struct SwapchainDesc {
    pub format: vk::Format,
    pub color_space: vk::ColorSpaceKHR,
    pub extent: crate::Extent2d,
    pub image_count: u32,
    pub present_mode: vk::PresentModeKHR,
}

#[derive(Clone)]
pub struct SwapchainImage {
    pub index: u32,
    pub image: crate::Image,
}

pub struct Swapchain {
    raw: vk::SwapchainKHR,
    loader: khr::Swapchain,

    images: Vec<SwapchainImage>,
}


impl Swapchain {
    /// Any options provided in desc are expected to be supported by our device
    /// TODO: Add compatibility checks?
    pub fn new(
        device: &Arc<super::DeviceInner>,
        surface: &super::Surface,
        desc: &super::SwapchainDesc,
        old_swapchain: Option<&Self>,
    ) -> Self {
        let old_swapchain = match old_swapchain {
            Some(old_swapchain) => old_swapchain.raw,
            None => vk::SwapchainKHR::null(),
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.raw)
            .min_image_count(desc.image_count)
            .image_format(desc.format)
            // No fancy wide gamut color spaces for now
            .image_array_layers(1)
            .image_color_space(desc.color_space)
            .image_extent(desc.extent)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            // TODO: pre_transform, my guess this is mostly for mobile
            .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(desc.present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain)
            .flags(vk::SwapchainCreateFlagsKHR::empty())
            .build();

        let loader = khr::Swapchain::new(&device.instance.raw, &device.raw);
        let raw = unsafe {
            loader.create_swapchain(&swapchain_create_info, None)
                .expect("Failed to create swapchain")
        };

        let images = unsafe {
            loader
                .get_swapchain_images(raw)
                .expect("Failed to get swapchain images")
        };

        let images = images
            .iter()
            .enumerate()
            .map(|(index, img)| {
                let image = super::Image::from_raw(
                    device,
                    *img,
                    crate::ImageDesc {
                        format: desc.format,
                        extent: crate::Extent3d {
                            width: desc.extent.width,
                            height: desc.extent.height,
                            depth: 1,
                        },
                        image_type: vk::ImageType::TYPE_2D,
                        usage: crate::ImageUsage::COLOR_ATTACHMENT,
                    },
                );

                SwapchainImage {
                    index: index as u32,
                    image: Arc::new(image),
                }
            })
            .collect::<Vec<_>>();

        Self {
            raw,
            loader,
            images,
        }
    }

    pub(super) fn acquire_next_image(
        &self,
        semaphore: &super::Semaphore,
    ) -> Result<SwapchainImage> {
        let (index, _) = unsafe {
            match self.loader.acquire_next_image(
                self.raw,
                u64::MAX,
                semaphore.raw,
                vk::Fence::null(),
            ) {
                Ok((index, is_suboptimal)) => (index, is_suboptimal),
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    return Err(anyhow::anyhow!("Swapchain out of date"))
                }
                Err(err) => panic!("Failed to acquire next image: {:?}", err),
            }
        };

        // TODO: Handle suboptimal

        Ok(self.images[index as usize].clone())
    }

    pub(super) fn present_image(
        &self,
        queue: &super::Queue,
        image: &SwapchainImage,
        render_finished: &super::Semaphore,
    ) -> Result<(), SwapchainError> {
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&[render_finished.raw])
            .swapchains(&[self.raw])
            .image_indices(&[image.index])
            .build();
        unsafe {
            match self.loader.queue_present(queue.raw, &present_info) {
                Ok(_) => {
                    // TODO: Handle suboptimal?
                    Ok(())
                },
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    Err(SwapchainError::Outdated)
                },
                Err(err) => panic!("Failed to present image: {:?}", err),
            }
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_swapchain(self.raw, None);
        }
    }
}
