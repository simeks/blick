use anyhow::Result;
use ash::extensions::ext;
use ash::vk;

#[cfg(any(target_os = "macos", target_os = "ios"))]
use ash::vk::{
    KhrGetPhysicalDeviceProperties2Fn, KhrPortabilityEnumerationFn,
};

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};
use std::ptr;

pub struct Instance {
    pub(super) entry: ash::Entry,
    pub(super) raw: ash::Instance,

    pub(super) debug_utils: Option<ext::DebugUtils>,
    debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
}

pub struct PhysicalDevice {
    pub(super) raw: vk::PhysicalDevice,
    pub(super) properties: vk::PhysicalDeviceProperties,
    #[allow(dead_code)]
    pub(super) memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub(super) queue_families: Vec<super::QueueFamily>,
}


impl Instance {
    pub fn new(
        required_extensions: &'static [*const c_char],
        debugging: bool,
    ) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        let mut extension_names = required_extensions.to_vec();
        let mut layer_names = Vec::new();

        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            extension_names.push(KhrPortabilityEnumerationFn::name().as_ptr());
            // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
            extension_names.push(KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
        }

        if debugging {
            extension_names.push(ext::DebugUtils::name().as_ptr());
            layer_names.push(CString::new("VK_LAYER_KHRONOS_validation").unwrap());
        }

        let layer_names = layer_names
            .iter()
            .map(|layer| layer.as_ptr())
            .collect::<Vec<_>>();

        let application_info = vk::ApplicationInfo::builder()
            .api_version(vk::make_api_version(0, 1, 2, 0))
            .build();

        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            vk::InstanceCreateFlags::default()
        };

        let instance_create_info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names)
            .flags(create_flags)
            .build();

        let instance = unsafe {
            entry.create_instance(&instance_create_info, None)?
        };

        let (debug_utils, debug_messenger) = if debugging {
            let (l, m) = setup_debug_utils(&entry, &instance);
            (Some(l), Some(m))
        } else {
            (None, None)
        };

        Ok(
            Self {
                entry,
                raw: instance,
                debug_utils,
                debug_messenger,
            },
        )
    }

    pub fn enumerate_physical_devices(&self) -> Result<Vec<PhysicalDevice>> {
        Ok(unsafe {
            self.raw.enumerate_physical_devices()?
                .into_iter()
                .map(|device| {
                    let properties = self.raw.get_physical_device_properties(device);
                    let memory_properties = self.raw
                        .get_physical_device_memory_properties(device);
                    let queue_families = self.raw
                        .get_physical_device_queue_family_properties(device)
                        .into_iter()
                        .enumerate()
                        .map(|(index, properties)| {
                            super::QueueFamily {
                                index: index as u32,
                                properties,
                            }
                        })
                        .collect();
                    
                    PhysicalDevice {
                        raw: device,
                        properties,
                        memory_properties,
                        queue_families,
                    }
                })
                .collect::<Vec<_>>()
        })
    }

}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            if let Some(du) = self.debug_utils.take() {
                du.destroy_debug_utils_messenger(
                    self.debug_messenger.take().unwrap(),
                    None
                );
            }
            self.raw.destroy_instance(None);
        }
    }
}

/// Callback function used in Debug Utils.
unsafe extern "system" fn vulkan_debug_utils_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let severity = match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => "[Verbose]",
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => "[Warning]",
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => "[Error]",
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => "[Info]",
        _ => "[Unknown]",
    };
    let types = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let message = CStr::from_ptr((*p_callback_data).p_message);
    log::debug!("[Debug]{}{}{:?}", severity, types, message);

    vk::FALSE
}

pub fn setup_debug_utils(
    entry: &ash::Entry,
    instance: &ash::Instance,
) -> (ash::extensions::ext::DebugUtils, vk::DebugUtilsMessengerEXT) {
    let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);

    let messenger_ci = populate_debug_messenger_create_info();

    let utils_messenger = unsafe {
        debug_utils_loader
            .create_debug_utils_messenger(&messenger_ci, None)
            .expect("Debug Utils Callback")
    };

    (debug_utils_loader, utils_messenger)
}

pub fn populate_debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
    vk::DebugUtilsMessengerCreateInfoEXT {
        s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
        p_next: ptr::null(),
        flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING |
            // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
        pfn_user_callback: Some(vulkan_debug_utils_callback),
        p_user_data: ptr::null_mut(),
    }
}

