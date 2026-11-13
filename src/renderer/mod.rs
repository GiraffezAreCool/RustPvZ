/*
in what order am i supposed to destroy stuff??????
*/

use anyhow::Ok;
use anyhow::Result;
use anyhow::bail;
use ash::Entry;
use ash::khr;
use ash::khr::swapchain;
use ash::vk;
use ash::vk::PipelineLayout;
use std::collections::HashMap;
use std::ffi;
use std::io;
use std::sync::Arc;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::raw_window_handle::HasDisplayHandle;
use winit::raw_window_handle::HasWindowHandle;
use winit::window::Window;
use winit::window::WindowId;

#[derive(Clone, Copy)]
struct PhysicalDevice {
    handle: vk::PhysicalDevice,
    properties: vk::PhysicalDeviceProperties,
}

struct LogicalDevice {
    handle: ash::Device,
    graphics_queue: vk::Queue,
    graphics_queue_idx: u32,
}

struct VulkanRenderer {
    entry: Entry,
    instance: ash::Instance,
    physical_device: PhysicalDevice,
    logical_device: LogicalDevice,
    graphics_pipeline: RendererPipeline,
}

impl VulkanRenderer {
    fn new(
        window: &RendererWindow,
        instance: ash::Instance,
        entry: ash::Entry,
        physical_device: PhysicalDevice,
    ) -> Self {
        println!("Making VulkanRenderer");
        let logical_device = Self::create_logical_device(&instance, physical_device).unwrap();
        let graphics_pipeline = RendererPipeline::new(&logical_device, &window, &instance).unwrap();
        Self {
            entry,
            instance,
            physical_device,
            logical_device,
            graphics_pipeline,
        }
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: PhysicalDevice,
    ) -> Result<LogicalDevice> {
        let queue_family_prop =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device.handle) };

        let mut idx = None;

        for (i, queue_family) in queue_family_prop.iter().enumerate() {
            if queue_family.queue_count <= 0 {
                continue;
            }

            if queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                idx = Some(i as u32);
                break;
            }
        }

        let graphics_queue_idx = match idx {
            None => bail!("No graphics queue found"),
            Some(i) => i,
        };

        let queue_info = vec![
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(graphics_queue_idx)
                .queue_priorities(&[1.0]),
        ];

        let handle = unsafe {
            instance.create_device(
                physical_device.handle,
                &vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_info)
                    .enabled_extension_names(&[ash::khr::swapchain::NAME.as_ptr()])
                    .push_next(
                        &mut vk::PhysicalDeviceDynamicRenderingFeatures::default()
                            .dynamic_rendering(true),
                    )
                    .push_next(
                        &mut vk::PhysicalDeviceBufferDeviceAddressFeatures::default()
                            .buffer_device_address(true),
                    )
                    .push_next(
                        &mut vk::PhysicalDeviceDynamicRenderingLocalReadFeaturesKHR::default()
                            .dynamic_rendering_local_read(true),
                    ),
                None,
            )?
        };

        let graphics_queue = unsafe { handle.get_device_queue(graphics_queue_idx, 0) };

        Ok(LogicalDevice {
            handle,
            graphics_queue,
            graphics_queue_idx,
        })
    }

    fn clean_up(&mut self) {
        self.graphics_pipeline.clean_up(&self.logical_device);
        unsafe {
            self.logical_device.handle.destroy_device(None);
            self.instance.destroy_instance(None);
        }
    }
}

//impl Drop for VulkanRenderer {
//    fn drop(&mut self) {
//        self.graphics_pipeline.clean_up(&self.logical_device);
//        unsafe {
//            self.logical_device.handle.destroy_device(None);
//            self.instance.destroy_instance(None);
//        }
//    }
//}

struct RendererWindow {
    window: Window,
    surface: vk::SurfaceKHR,
    surface_instance: ash::khr::surface::Instance,
    capabilities: vk::SurfaceCapabilitiesKHR,
}

//impl Drop for RendererWindow {
//    fn drop(&mut self) {
//        unsafe { self.surface_instance.destroy_surface(self.surface, None) };
//    }
//}

pub struct Engine {
    renderer: VulkanRenderer,
    windows: HashMap<WindowId, RendererWindow>,
    primary_window_id: WindowId,
}

impl Engine {
    pub fn new(event_loop: &ActiveEventLoop) -> Result<Self> {
        println!("Making Engine");
        let primary_window = event_loop.create_window(Default::default())?;
        let primary_window_id = primary_window.id();

        primary_window.set_title("PvZ Rust");

        let entry = unsafe { ash::Entry::load().unwrap() };
        let instance = Self::create_instance(&entry, &primary_window).unwrap();
        let physical_device =
            Self::get_physical_device(&instance, &entry, &primary_window).unwrap();

        let raw_display_handle = primary_window.display_handle()?.as_raw();
        let raw_window_handle = primary_window.window_handle()?.as_raw();

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )?
        };

        let surface_instance = khr::surface::Instance::new(&entry, &instance);

        let capabilities = unsafe {
            surface_instance
                .get_physical_device_surface_capabilities(physical_device.handle, surface)?
        };

        let renderer_window = RendererWindow {
            window: primary_window,
            surface,
            surface_instance,
            capabilities,
        };

        let renderer = VulkanRenderer::new(&renderer_window, instance, entry, physical_device);

        let windows = HashMap::from([(primary_window_id, renderer_window)]);
        Ok(Self {
            renderer,
            windows,
            primary_window_id,
        })
    }

    fn create_instance(entry: &ash::Entry, window: &Window) -> Result<ash::Instance> {
        let app_name = ffi::CString::new("PvZ Rust").unwrap();
        let engine_name = ffi::CString::new("Vulkan Engine").unwrap();

        let app_info = vk::ApplicationInfo::default()
            .application_name(&app_name)
            .engine_name(&engine_name)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);

        let raw_display_handle = window.display_handle()?.as_raw();

        let info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(ash_window::enumerate_required_extensions(
                raw_display_handle,
            )?);

        let instance = unsafe { entry.create_instance(&info, None)? };

        Ok(instance)
    }

    fn get_physical_device(
        instance: &ash::Instance,
        entry: &ash::Entry,
        window: &Window,
    ) -> Result<PhysicalDevice> {
        let mut physical_devices = unsafe { instance.enumerate_physical_devices()? };

        let raw_display_handle = window.display_handle()?.as_raw();
        let raw_window_handle = window.window_handle()?.as_raw();

        let surface_extention = ash::khr::surface::Instance::new(&entry, &instance);

        let compatibility_surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )?
        };
        unsafe {
            physical_devices.retain(|device| {
                surface_extention
                    .get_physical_device_surface_support(*device, 0, compatibility_surface)
                    .unwrap_or(false)
            })
        };

        unsafe { surface_extention.destroy_surface(compatibility_surface, None) };

        let mut handle = physical_devices[0];

        for physical_device in physical_devices {
            let prop = unsafe { instance.get_physical_device_properties(physical_device) };
            if prop.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
                handle = physical_device;
                break;
            }
        }

        let properties = unsafe { instance.get_physical_device_properties(handle) };

        Ok(PhysicalDevice { handle, properties })
    }

    pub fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if window_id == self.primary_window_id {
                    event_loop.exit();
                } else {
                    self.windows.remove(&window_id);
                }
            }
            _ => {}
        }
    }

    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        attributes: winit::window::WindowAttributes,
    ) -> Result<WindowId> {
        let window = event_loop.create_window(attributes)?;
        let window_id = window.id();

        let raw_display_handle = window.display_handle()?.as_raw();
        let raw_window_handle = window.window_handle()?.as_raw();

        let surface = unsafe {
            ash_window::create_surface(
                &self.renderer.entry,
                &self.renderer.instance,
                raw_display_handle,
                raw_window_handle,
                None,
            )?
        };
        let surface_instance =
            khr::surface::Instance::new(&self.renderer.entry, &self.renderer.instance);

        let capabilities = unsafe {
            surface_instance.get_physical_device_surface_capabilities(
                self.renderer.physical_device.handle,
                surface,
            )?
        };

        let renderer_window = RendererWindow {
            window,
            surface,
            surface_instance,
            capabilities,
        };
        self.windows.insert(window_id, renderer_window);

        Ok(window_id)
    }
}

//impl Drop for Engine {
//    fn drop(&mut self) {
//        for (window_id, renderer_window) in &mut self.windows {
//            unsafe {
//                renderer_window
//                    .surface_instance
//                    .destroy_surface(renderer_window.surface, None)
//            };
//        }
//        self.renderer.clean_up();
//    }
//}

struct RendererSwapchain {
    swapchain_instance: khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    image_views: Vec<vk::ImageView>,
    format: vk::Format,
    extent: vk::Extent2D,
}

impl RendererSwapchain {
    fn new(
        instance: &ash::Instance,
        logical_device: &LogicalDevice,
        //physical_device: &PhysicalDevice,
        window: &RendererWindow,
    ) -> Result<Self> {
        println!("Making Swapchain");
        let format = vk::Format::B8G8R8A8_SRGB;
        let extent = window.capabilities.current_extent;

        let swapchain_instance = khr::swapchain::Device::new(&instance, &logical_device.handle);

        let swapchain = unsafe {
            swapchain_instance.create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(window.surface)
                    .min_image_count(
                        3.max(window.capabilities.min_image_count)
                            .min(window.capabilities.max_image_count),
                    )
                    .image_format(format)
                    .image_color_space(vk::ColorSpaceKHR::SRGB_NONLINEAR)
                    .image_extent(extent)
                    .image_array_layers(1)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .queue_family_indices(&[logical_device.graphics_queue_idx])
                    .pre_transform(window.capabilities.current_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(vk::PresentModeKHR::FIFO),
                None,
            )?
        };

        let images = unsafe { swapchain_instance.get_swapchain_images(swapchain)? };

        let mut image_views: Vec<vk::ImageView> = Vec::with_capacity(images.len());

        for image in images {
            let subresource_range = vk::ImageSubresourceRange::default()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1);

            let image_view_info = vk::ImageViewCreateInfo::default()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(format)
                .subresource_range(subresource_range);

            let image_view = unsafe {
                logical_device
                    .handle
                    .create_image_view(&image_view_info, None)?
            };

            image_views.push(image_view);
        }

        Ok(Self {
            swapchain_instance,
            swapchain,
            image_views,
            format,
            extent,
        })
    }

    fn clean_up(&mut self, logical_device: &LogicalDevice) {
        for image_view in &self.image_views {
            unsafe { logical_device.handle.destroy_image_view(*image_view, None) };
        }
        unsafe {
            self.swapchain_instance
                .destroy_swapchain(self.swapchain, None)
        };
    }
}

struct Shader {
    shader_module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
}

impl Shader {
    fn new(
        logical_device: &LogicalDevice,
        code: &[u32],
        stage: vk::ShaderStageFlags,
    ) -> Result<Self> {
        let shader_module_info = vk::ShaderModuleCreateInfo::default().code(code);

        let shader_module = unsafe {
            logical_device
                .handle
                .create_shader_module(&shader_module_info, None)?
        };

        Ok(Shader {
            shader_module,
            stage,
        })
    }

    fn clean_up(&mut self, logical_device: &LogicalDevice) {
        unsafe {
            logical_device
                .handle
                .destroy_shader_module(self.shader_module, None)
        };
    }
}

//impl Drop for Shader {
//    fn drop(&mut self) {
//        unsafe {
//            self.logical_device
//                .handle
//                .destroy_shader_module(self.shader_module, None)
//        };
//    }
//}

struct RendererPipeline {
    pipeline_layout: PipelineLayout,
    pipeline: vk::Pipeline,
    swapchain: RendererSwapchain,
}

impl RendererPipeline {
    fn new(
        logical_device: &LogicalDevice,
        window: &RendererWindow,
        instance: &ash::Instance,
    ) -> Result<Self> {
        let mut swapchain = RendererSwapchain::new(instance, &logical_device, window)?;
        println!("Making RendererPipeline");

        let mut vertex_code = io::Cursor::new(std::fs::read("src/shaders/vert.spv")?);
        let mut vert_shader = Shader::new(
            &logical_device,
            &*(ash::util::read_spv(&mut vertex_code)?),
            vk::ShaderStageFlags::default(),
        )?;

        let mut frag_code = io::Cursor::new(std::fs::read("src/shaders/frag.spv")?);
        let mut frag_shader = Shader::new(
            &logical_device,
            &*(ash::util::read_spv(&mut frag_code)?),
            vk::ShaderStageFlags::default(),
        )?;

        let pipeline_layout = unsafe {
            logical_device
                .handle
                .create_pipeline_layout(&vk::PipelineLayoutCreateInfo::default(), None)?
        };

        let pipeline = Self::create_graphics_pipeline(
            &logical_device,
            &swapchain,
            &vert_shader,
            &frag_shader,
            pipeline_layout,
            Default::default(),
        )?;

        let renderer_pipeline = Self {
            pipeline_layout,
            pipeline,
            swapchain,
        };

        vert_shader.clean_up(logical_device);
        frag_shader.clean_up(logical_device);

        Ok(renderer_pipeline)
    }

    fn create_graphics_pipeline(
        logical_device: &LogicalDevice,
        swapchain: &RendererSwapchain,
        vert_shader: &Shader,
        frag_shader: &Shader,
        pipeline_layout: PipelineLayout,
        pipeline_cache: vk::PipelineCache,
    ) -> Result<vk::Pipeline> {
        let entry_point = std::ffi::CString::new("main")?;

        let graphics_pipeline = unsafe {
            logical_device
                .handle
                .create_graphics_pipelines(
                    pipeline_cache,
                    &[vk::GraphicsPipelineCreateInfo::default()
                        .stages(&[
                            vk::PipelineShaderStageCreateInfo::default()
                                .stage(vk::ShaderStageFlags::VERTEX)
                                .module(vert_shader.shader_module)
                                .name(&entry_point),
                            vk::PipelineShaderStageCreateInfo::default()
                                .stage(vk::ShaderStageFlags::FRAGMENT)
                                .module(frag_shader.shader_module)
                                .name(&entry_point),
                        ])
                        .vertex_input_state(&vk::PipelineVertexInputStateCreateInfo::default())
                        .input_assembly_state(
                            &vk::PipelineInputAssemblyStateCreateInfo::default()
                                .topology(vk::PrimitiveTopology::TRIANGLE_LIST),
                        )
                        .viewport_state(
                            &vk::PipelineViewportStateCreateInfo::default()
                                .viewports(&[vk::Viewport::default()
                                    .width(swapchain.extent.width as f32)
                                    .height(swapchain.extent.height as f32)
                                    .max_depth(1.0)])
                                .scissors(&[vk::Rect2D::default().extent(swapchain.extent)]),
                        )
                        .rasterization_state(
                            &vk::PipelineRasterizationStateCreateInfo::default()
                                .polygon_mode(vk::PolygonMode::FILL)
                                .cull_mode(vk::CullModeFlags::NONE)
                                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                                .line_width(1.0),
                        )
                        .multisample_state(
                            &vk::PipelineMultisampleStateCreateInfo::default()
                                .rasterization_samples(vk::SampleCountFlags::TYPE_1),
                        )
                        .color_blend_state(
                            &vk::PipelineColorBlendStateCreateInfo::default().attachments(&[
                                vk::PipelineColorBlendAttachmentState::default()
                                    .color_write_mask(vk::ColorComponentFlags::RGBA)
                                    .blend_enable(false),
                            ]),
                        )
                        .dynamic_state(
                            &vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&[
                                vk::DynamicState::VIEWPORT,
                                vk::DynamicState::SCISSOR,
                            ]),
                        )
                        .layout(pipeline_layout)
                        .push_next(
                            &mut vk::PipelineRenderingCreateInfo::default()
                                .color_attachment_formats(&[swapchain.format]),
                        )],
                    None,
                )
                .unwrap()
                .into_iter()
                .next()
                .unwrap()
        };

        Ok(graphics_pipeline)
    }

    fn clean_up(&mut self, logical_device: &LogicalDevice) {
        unsafe {
            logical_device.handle.destroy_pipeline(self.pipeline, None);
            logical_device
                .handle
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
        self.swapchain.clean_up(logical_device);
    }
}
