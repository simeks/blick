use crate::BackendConfig;

use anyhow::Result;

use ash::vk;
use ash_window::enumerate_required_extensions;
use raw_window_handle::HasRawDisplayHandle;
use std::sync::Arc;
use winit::window::Window;

use super::swapchain;

pub struct Frame {
    pub image_available: crate::Semaphore,
    pub render_finished: crate::Semaphore,
    pub swapchain_image: super::swapchain::SwapchainImage,
}

pub struct Backend {
    swapchain_desc: super::SwapchainDesc,
    swapchain: super::Swapchain,

    #[allow(dead_code)]
    instance: Arc<super::Instance>,
    device: Arc<super::Device>,
    surface: super::Surface,
}

impl Backend {
    pub fn new(
        window: &Window,
        config: BackendConfig,
    ) -> Self {
        let instance = Arc::new(
            super::Instance::new(
                enumerate_required_extensions(window.raw_display_handle()).unwrap(),
                config.debugging,
            )
                .expect("Failed to create vulkan instance")
        );

        let surface = super::Surface::new(
            &instance,
            window,
        )
            .expect("Failed to create vulkan surface");

        let physical_devices = instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate physical devices");

        log::info!("Available devices:");
        physical_devices.iter().for_each(|device| {
            log::info!("    {:?}", super::vk_to_string(&device.properties.device_name));
        });

        // Filter devices supporting presentation
        let physical_devices = physical_devices
            .into_iter()
            .filter(|device| {
                device.queue_families
                    .iter()
                    .any(|queue_family| {
                        surface.supports_queue_family(device, queue_family.index)
                    })
            });

        // Pick first GPU, if no GPU pick first integrated
        let physical_device = physical_devices
            .rev() // Rev due to max_by_key picking from the bottom
            .max_by_key(|device| match device.properties.device_type {
                vk::PhysicalDeviceType::VIRTUAL_GPU => 10,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 100,
                vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                _ => 0,
            })
            .unwrap();

        log::info!(
            "Using device: {}",
            super::vk_to_string(&physical_device.properties.device_name)
        );

        let device = Arc::new(
            super::Device::new(
                &instance,
                physical_device,
                &config,
            )
                .expect("Failed to create vulkan device")
        );

        let surface_capabilities = surface.query_surface_capabilities(
            &device.inner.physical_device
        );

        let surface_formats = surface.query_surface_formats(
            &device.inner.physical_device
        );

        let surface_present_modes = surface.query_surface_present_modes(
            &device.inner.physical_device
        );
    
        let swapchain_desc = make_swapchain_desc(
            window.inner_size().width,
            window.inner_size().height,
            &surface_capabilities,
            &surface_formats,
            &surface_present_modes,
            true
        );

        let swapchain = super::Swapchain::new(
            &device.inner,
            &surface,
            &swapchain_desc,
            None,
        );

        Self {
            surface,
            swapchain,
            swapchain_desc,
            instance,
            device,
        }
    }

    /// Acquires swapchain image
    pub fn begin_frame(&mut self) -> Result<Frame, crate::BeginFrameError> {
        // TODO: Investigate best way of setting up a frame

        // TODO: Don't recreate every frame
        let image_available = self.device.create_semaphore().unwrap();
        let render_finished = self.device.create_semaphore().unwrap();

        let swapchain_image = match self.swapchain.acquire_next_image(&image_available) {
            Ok(image) => image,
            Err(_) => return Err(crate::BeginFrameError::OutdatedSwapchain),
        };

        Ok(Frame {
            image_available,
            render_finished,
            swapchain_image,
        })
    }
    pub fn end_frame(&mut self, frame: Frame) -> Result<(), crate::EndFrameError> {
        match self.swapchain.present_image(
            &self.device.inner.universal_queue,
            &frame.swapchain_image,
            &frame.render_finished,
        ) {
            Ok(_) => (),
            Err(swapchain::SwapchainError::Outdated) => {
                return Err(crate::EndFrameError::OutdatedSwapchain)
            },
        }

        self.device.wait_idle().unwrap();

        Ok(())
    }

    pub fn resize_swapchain(&mut self, width: u32, height: u32) {
        let surface_capabilities = self.surface.query_surface_capabilities(
            &self.device.inner.physical_device
        );

        self.swapchain_desc = super::SwapchainDesc {
            extent: make_swapchain_extent(
                &surface_capabilities,
                width,
                height
            ),
            ..self.swapchain_desc
        };

        self.swapchain = super::Swapchain::new(
            &self.device.inner,
            &self.surface,
            &self.swapchain_desc,
            Some(&self.swapchain),
        );
    }
    pub fn device(&self) -> &super::Device {
        &self.device
    }
    pub fn swapchain_desc(&self) -> &super::SwapchainDesc {
        &self.swapchain_desc
    }
}

fn make_swapchain_extent(
    surface_capabilities: &vk::SurfaceCapabilitiesKHR,
    width: u32,
    height: u32,
) -> crate::Extent2d {
    if surface_capabilities.current_extent.width != std::u32::MAX {
        surface_capabilities.current_extent
    } else {
        crate::Extent2d {
            width: width.max(surface_capabilities.min_image_extent.width)
                .min(surface_capabilities.max_image_extent.width),
            height: height.max(surface_capabilities.min_image_extent.height)
                .min(surface_capabilities.max_image_extent.height),
        }
    }
}

/// Creates a compatible swapchain config
fn make_swapchain_desc(
    width: u32,
    height: u32,
    surface_capabilities: &vk::SurfaceCapabilitiesKHR,
    surface_formats: &[vk::SurfaceFormatKHR],
    surface_present_modes: &[vk::PresentModeKHR],
    vsync: bool,
) -> super::SwapchainDesc {
    // check if list contains most widely used R8G8B8A8 format with nonlinear color space

    let surface_format = surface_formats
        .iter()
        .find(|format| {
            format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .expect("Failed to find suitable supported swapchain format");


    let extent = make_swapchain_extent(surface_capabilities, width, height);

    let preferred_present_modes = if vsync {
        vec![
            vk::PresentModeKHR::FIFO_RELAXED,
            vk::PresentModeKHR::FIFO
        ]
    } else {
        vec![
            vk::PresentModeKHR::MAILBOX,
            vk::PresentModeKHR::IMMEDIATE,
        ]
    };

    let present_mode = preferred_present_modes
        .into_iter()
        .find(|mode| surface_present_modes.contains(mode))
        // default to FIFO since it's always supported
        .unwrap_or(vk::PresentModeKHR::FIFO);

    let image_count = 3;
    let image_count = if surface_capabilities.max_image_count > 0 {
        image_count.min(surface_capabilities.max_image_count)
    } else {
        image_count
    };

    super::SwapchainDesc {
        format: surface_format.format,
        color_space: surface_format.color_space,
        extent,
        // TODO:
        present_mode,
        image_count,
    }
}

