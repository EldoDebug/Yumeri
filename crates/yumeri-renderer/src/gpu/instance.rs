use ash::vk;
use raw_window_handle::RawDisplayHandle;
use std::ffi::CStr;

use crate::error::Result;

pub struct VulkanInstance {
    entry: ash::Entry,
    instance: ash::Instance,
    #[cfg(debug_assertions)]
    debug_utils_loader: ash::ext::debug_utils::Instance,
    #[cfg(debug_assertions)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanInstance {
    pub fn new(display_handle: RawDisplayHandle) -> Result<Self> {
        let entry = unsafe { ash::Entry::load().expect("failed to load Vulkan") };

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"Yumeri")
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(c"Yumeri Engine")
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(vk::API_VERSION_1_3);

        let mut extension_names =
            ash_window::enumerate_required_extensions(display_handle)?.to_vec();

        #[cfg(debug_assertions)]
        extension_names.push(ash::ext::debug_utils::NAME.as_ptr());

        let layer_names: Vec<*const i8> = if cfg!(debug_assertions) {
            vec![c"VK_LAYER_KHRONOS_validation".as_ptr()]
        } else {
            vec![]
        };

        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names)
            .enabled_layer_names(&layer_names);

        let instance = unsafe { entry.create_instance(&instance_info, None)? };

        #[cfg(debug_assertions)]
        let (debug_utils_loader, debug_messenger) = {
            let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
                .message_severity(
                    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                )
                .message_type(
                    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                )
                .pfn_user_callback(Some(vulkan_debug_callback));

            let loader = ash::ext::debug_utils::Instance::new(&entry, &instance);
            let messenger =
                unsafe { loader.create_debug_utils_messenger(&debug_info, None)? };
            (loader, messenger)
        };

        log::info!("Vulkan instance created (API 1.3)");

        Ok(Self {
            entry,
            instance,
            #[cfg(debug_assertions)]
            debug_utils_loader,
            #[cfg(debug_assertions)]
            debug_messenger,
        })
    }

    pub fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    pub fn raw(&self) -> &ash::Instance {
        &self.instance
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            #[cfg(debug_assertions)]
            self.debug_utils_loader
                .destroy_debug_utils_messenger(self.debug_messenger, None);
            self.instance.destroy_instance(None);
        }
    }
}

#[cfg(debug_assertions)]
unsafe extern "system" fn vulkan_debug_callback(
    severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _ty: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    let message = unsafe { CStr::from_ptr((*callback_data).p_message) };
    let message = message.to_string_lossy();

    match severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => log::error!("[Vulkan] {message}"),
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => log::warn!("[Vulkan] {message}"),
        _ => log::debug!("[Vulkan] {message}"),
    }

    vk::FALSE
}
