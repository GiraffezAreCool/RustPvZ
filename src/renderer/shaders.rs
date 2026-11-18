use crate::renderer::LogicalDevice;
use crate::renderer::PhysicalDevice;
use anyhow;
use anyhow::Ok;
use anyhow::Result;
use ash::Instance;
use ash::vk;
use memoffset::offset_of;

pub struct Shader {
    pub shader_module: vk::ShaderModule,
    pub stage: vk::ShaderStageFlags,
}

impl Shader {
    pub fn new(
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

    pub fn clean_up(&mut self, logical_device: &LogicalDevice) {
        unsafe {
            logical_device
                .handle
                .destroy_shader_module(self.shader_module, None)
        };
        println!("Destroyed shader module");
    }
}

const TRIANGLE: [Vertex; 3] = [
    Vertex {
        position: [0.0, -0.5],
        colour: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5],
        colour: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [-0.5, 0.5],
        colour: [0.0, 0.0, 1.0],
    },
];

pub struct Vertex {
    position: [f32; 2],
    colour: [f32; 3],
}

impl Vertex {
    pub fn get_binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 2] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, colour) as u32,
            },
        ]
    }
}

pub struct VertexBuffer {
    pub handle: vk::Buffer,
    pub memory_requirements: vk::MemoryRequirements,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub vertex_length: u32,
}

impl VertexBuffer {
    pub fn new(
        logical_device: &LogicalDevice,
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<Self> {
        println!("Making VertexBuffer");
        let handle = unsafe {
            logical_device.handle.create_buffer(
                &vk::BufferCreateInfo::default()
                    .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
                    .size(std::mem::size_of_val(&TRIANGLE) as u64), // dont like that its just grabign the triangle
                None,
            )?
        };

        let memory_requirements =
            unsafe { logical_device.handle.get_buffer_memory_requirements(handle) };

        let required_memory_flags: vk::MemoryPropertyFlags =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;

        let memory_type = Self::find_memory_type(
            memory_requirements.memory_type_bits,
            required_memory_flags,
            unsafe { instance.get_physical_device_memory_properties(physical_device.handle) },
        );

        let vertex_buffer_memory = unsafe {
            logical_device.handle.allocate_memory(
                &vk::MemoryAllocateInfo::default()
                    .allocation_size(memory_requirements.size)
                    .memory_type_index(memory_type),
                None,
            )?
        };

        unsafe {
            logical_device
                .handle
                .bind_buffer_memory(handle, vertex_buffer_memory, 0)?;

            let data_ptr = logical_device.handle.map_memory(
                vertex_buffer_memory,
                0,
                std::mem::size_of_val(&TRIANGLE) as u64,
                vk::MemoryMapFlags::empty(),
            )? as *mut Vertex;

            data_ptr.copy_from_nonoverlapping(TRIANGLE.as_ptr(), TRIANGLE.len());

            logical_device.handle.unmap_memory(vertex_buffer_memory);
        };

        Ok(Self {
            handle,
            memory_requirements,
            vertex_buffer_memory,
            vertex_length: TRIANGLE.len() as u32,
        })
    }

    fn find_memory_type(
        type_filter: u32,
        required_properties: vk::MemoryPropertyFlags,
        mem_properties: vk::PhysicalDeviceMemoryProperties,
    ) -> u32 {
        for (i, memory_type) in mem_properties.memory_types.iter().enumerate() {
            if (type_filter & (1 << i)) > 0
                && memory_type.property_flags.contains(required_properties)
            {
                return i as u32;
            }
        }

        panic!("Failed to find suitable memory type!")
    }

    pub fn clean_up(&mut self, logical_device: &LogicalDevice) {
        unsafe {
            logical_device.handle.destroy_buffer(self.handle, None);
            logical_device
                .handle
                .free_memory(self.vertex_buffer_memory, None);
            println!("Destroyed vertex buffers");
        };
    }
}
