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

const VERTICES_DATA: [Vertex; 4] = [
    Vertex {
        position: [-0.5, -0.5],
        colour: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5],
        colour: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, 0.5],
        colour: [0.0, 0.0, 1.0],
    },
    Vertex {
        position: [-0.5, 0.5],
        colour: [1.0, 1.0, 1.0],
    },
];
const INDICES_DATA: [u32; 6] = [0, 1, 2, 2, 3, 0];

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

//Maybe make just one buffer struct

pub struct VertexBuffer {
    pub handle: vk::Buffer,
    pub buffer_memory: vk::DeviceMemory,
    pub vertex_length: u32,
    pub staging_buffer: StageBuffer,
    pub buffer_size: u64,
}

impl VertexBuffer {
    pub fn new(
        logical_device: &LogicalDevice,
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<Self> {
        let buffer_size = std::mem::size_of_val(&VERTICES_DATA) as u64;
        let staging_buffer = StageBuffer::new(&logical_device, &instance, &physical_device)?;
        unsafe {
            let data_ptr = logical_device.handle.map_memory(
                staging_buffer.buffer_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut Vertex;

            data_ptr.copy_from_nonoverlapping(VERTICES_DATA.as_ptr(), VERTICES_DATA.len());

            logical_device
                .handle
                .unmap_memory(staging_buffer.buffer_memory);
        };

        println!("Making VertexBuffer");
        let handle = unsafe {
            logical_device.handle.create_buffer(
                &vk::BufferCreateInfo::default()
                    .usage(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                    .size(buffer_size), // dont like that its just grabign the triangle
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

        let buffer_memory = unsafe {
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
                .bind_buffer_memory(handle, buffer_memory, 0)?
        };
        Ok(Self {
            handle,
            buffer_memory,
            vertex_length: VERTICES_DATA.len() as u32,
            staging_buffer,
            buffer_size,
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
            self.staging_buffer.clean_up(&logical_device);
            logical_device.handle.destroy_buffer(self.handle, None);
            logical_device.handle.free_memory(self.buffer_memory, None);
            println!("Destroyed vertex buffers");
        };
    }
}

pub struct StageBuffer {
    pub handle: vk::Buffer,
    pub buffer_memory: vk::DeviceMemory,
}

impl StageBuffer {
    pub fn new(
        logical_device: &LogicalDevice,
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<Self> {
        println!("Making StageBuffer");
        let handle = unsafe {
            logical_device.handle.create_buffer(
                &vk::BufferCreateInfo::default()
                    .usage(vk::BufferUsageFlags::TRANSFER_SRC)
                    .size(std::mem::size_of_val(&VERTICES_DATA) as u64), // dont like that its just grabign the triangle
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

        let buffer_memory = unsafe {
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
                .bind_buffer_memory(handle, buffer_memory, 0)?
        };

        Ok(Self {
            handle,
            buffer_memory,
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
            logical_device.handle.free_memory(self.buffer_memory, None);
            println!("Destroyed staging buffers");
        };
    }
}

pub struct IndexBuffer {
    pub handle: vk::Buffer,
    pub buffer_memory: vk::DeviceMemory,
    pub staging_buffer: StageBuffer,
    pub index_length: u32,
    pub buffer_size: u64,
}

impl IndexBuffer {
    pub fn new(
        logical_device: &LogicalDevice,
        instance: &Instance,
        physical_device: &PhysicalDevice,
    ) -> Result<Self> {
        let buffer_size = std::mem::size_of_val(&INDICES_DATA) as u64;
        let staging_buffer = StageBuffer::new(&logical_device, &instance, &physical_device)?;
        unsafe {
            let data_ptr = logical_device.handle.map_memory(
                staging_buffer.buffer_memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )? as *mut u32;

            data_ptr.copy_from_nonoverlapping(INDICES_DATA.as_ptr(), INDICES_DATA.len());

            logical_device
                .handle
                .unmap_memory(staging_buffer.buffer_memory);
        };

        println!("Making IndexBuffer");
        let handle = unsafe {
            logical_device.handle.create_buffer(
                &vk::BufferCreateInfo::default()
                    .usage(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST)
                    .size(buffer_size), // dont like that its just grabign the triangle
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

        let buffer_memory = unsafe {
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
                .bind_buffer_memory(handle, buffer_memory, 0)?
        };

        Ok(Self {
            handle,
            buffer_memory,
            staging_buffer,
            index_length: INDICES_DATA.len() as u32,
            buffer_size,
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
            self.staging_buffer.clean_up(logical_device);
            logical_device.handle.destroy_buffer(self.handle, None);
            logical_device.handle.free_memory(self.buffer_memory, None);
            println!("Destroyed index buffers");
        };
    }
}
