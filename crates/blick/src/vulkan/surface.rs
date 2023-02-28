use super::{Instance, PhysicalDevice};

use anyhow::Result;

use ash::extensions::khr;
use ash::vk;

use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

pub struct Surface {
    pub(super) raw : vk::SurfaceKHR,
    loader : khr::Surface,
}

impl Surface {
    pub fn new(
        instance: &Instance,
        window: &Window
    ) -> Result<Self> {
        let surface = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance.raw,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None
            )?
        };
        let loader = khr::Surface::new(&instance.entry, &instance.raw);

        Ok(Self {
            raw: surface,
            loader,
        })
    }

    pub fn query_surface_capabilities(
        &self,
        physical_device: &PhysicalDevice,
    ) -> vk::SurfaceCapabilitiesKHR {
        unsafe {
            self.loader
                .get_physical_device_surface_capabilities(physical_device.raw, self.raw)
                .expect("Failed to query for surface capabilities")
        }
    }

    pub fn query_surface_formats(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Vec<vk::SurfaceFormatKHR> {
        unsafe {
            self.loader
                .get_physical_device_surface_formats(physical_device.raw, self.raw)
                .expect("Failed to query for surface formats")
        }
    }

    pub fn query_surface_present_modes(
        &self,
        physical_device: &PhysicalDevice,
    ) -> Vec<vk::PresentModeKHR> {
        unsafe {
            self.loader
                .get_physical_device_surface_present_modes(physical_device.raw, self.raw)
                .expect("Failed to query for surface present modes")
        }
    }

    pub fn supports_queue_family(
        &self,
        physical_device: &PhysicalDevice,
        queue_family_index: u32,
    ) -> bool {
        unsafe {
            self.loader
                .get_physical_device_surface_support(
                    physical_device.raw,
                    queue_family_index,
                    self.raw,
                )
                .expect("Failed to query for surface support")
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.loader.destroy_surface(self.raw, None);
        }
    }
}

