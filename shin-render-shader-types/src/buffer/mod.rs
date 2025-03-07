mod bytes_address;
mod dynamic_buffer;
pub mod ownership;
pub mod types;

use std::marker::PhantomData;

use ownership::{AnyOwnership, BufferOwnership, Owned, Shared};
use types::BufferType;
use wgpu::util::DeviceExt as _;

pub use self::{bytes_address::BytesAddress, dynamic_buffer::DynamicBufferBackend};
use crate::{
    RenderClone, RenderCloneCtx,
    buffer::types::{ArrayBufferType, IndexMarker, RawMarker, VertexMarker},
    vertices::VertexType,
};

const PHYSICAL_SIZE_ALIGNMENT: BytesAddress = BytesAddress::new(4);

pub enum BufferUsage {
    /// MAP_WRITE | COPY_SRC
    StagingWrite,
    /// MAP_READ | COPY_DST
    StagingRead,
    // TODO: maybe create specialized dynamic buffers for index/vertex/uniform?
    // uniform buffers have very different layout requirements, so it will reduce wasted space somewhat
    /// COPY_DST | INDEX | VERTEX | UNIFORM
    Dynamic,
    // `Dynamic` for GPUs with MAPPABLE_PRIMARY_BUFFERS feature (integrated GPUs normally)
    /// MAP_WRITE | INDEX | VERTEX | UNIFORM
    DynamicMappable,
    /// VERTEX
    Vertex,
    /// INDEX
    Index,
}

impl From<BufferUsage> for wgpu::BufferUsages {
    fn from(value: BufferUsage) -> Self {
        match value {
            BufferUsage::StagingWrite => {
                wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC
            }
            BufferUsage::StagingRead => wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            BufferUsage::Dynamic => {
                wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::INDEX
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::UNIFORM
            }
            BufferUsage::DynamicMappable => {
                wgpu::BufferUsages::MAP_WRITE
                    | wgpu::BufferUsages::INDEX
                    | wgpu::BufferUsages::VERTEX
                    | wgpu::BufferUsages::UNIFORM
            }
            BufferUsage::Vertex => wgpu::BufferUsages::VERTEX,
            BufferUsage::Index => wgpu::BufferUsages::INDEX,
        }
    }
}

#[derive(Debug)]
pub struct Buffer<O: BufferOwnership, T: BufferType> {
    ownership: O,
    // TODO: do we still want to allow suballocation of owned buffers like this?
    // it seems that only suballocating buffer slices may be enough
    offset: BytesAddress,
    /// Logical size of the buffer, in bytes
    ///
    /// Does not necessarily correspond to "physical" buffer size reported to the underlying graphics API
    logical_size: BytesAddress,
    phantom: PhantomData<T>,
}

#[derive(Debug)]
pub struct BufferRef<'a, T: BufferType> {
    buffer: &'a wgpu::Buffer,
    offset: BytesAddress,
    size: BytesAddress,
    phantom: PhantomData<T>,
}

impl<O: BufferOwnership, T: BufferType> Buffer<O, T> {
    // TODO: perhaps make it private?
    pub fn allocate_raw(
        device: &wgpu::Device,
        size_bytes: BytesAddress,
        usage: BufferUsage,
        mapped_at_creation: bool,
        label: Option<&str>,
    ) -> Self {
        let offset = BytesAddress::new(0);
        let logical_size = size_bytes;
        let physical_size = logical_size.align_to(PHYSICAL_SIZE_ALIGNMENT);

        assert!(T::is_valid_offset(offset));
        assert!(T::is_valid_logical_size(logical_size));
        assert!(physical_size.is_aligned_to(PHYSICAL_SIZE_ALIGNMENT));

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label,
            size: physical_size.get(),
            usage: usage.into(),
            mapped_at_creation,
        });

        Buffer {
            ownership: O::new(buffer),
            offset,
            logical_size,
            phantom: PhantomData,
        }
    }

    // TODO: it would be nice to support mapping the typed buffer and allowing the API user to write to it directly
    // instead of building the whole buffer in memory and then copying it to the GPU
    pub fn allocate_raw_with_contents(
        device: &wgpu::Device,
        contents: &[u8],
        usage: BufferUsage,
        label: Option<&str>,
    ) -> Self {
        let offset = BytesAddress::new(0);
        let logical_size = BytesAddress::new(contents.len() as _);

        assert!(T::is_valid_offset(offset));
        assert!(T::is_valid_logical_size(logical_size));

        // wgpu will handle the physical size by itself
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label,
            contents,
            usage: usage.into(),
        });

        Buffer {
            ownership: O::new(buffer),
            offset,
            logical_size,
            phantom: PhantomData,
        }
    }

    #[deprecated(
        note = "Might not work properly if physical size is different from logical; needs to be fixed"
    )]
    pub fn from_wgpu_buffer(buffer: wgpu::Buffer) -> Self {
        let offset = BytesAddress::new(0);
        let size = BytesAddress::new(buffer.size());

        assert!(T::is_valid_offset(offset));
        // TODO: we need a method to derive a logical size from physical
        assert!(T::is_valid_logical_size(size));

        Buffer {
            ownership: O::new(buffer),
            offset,
            logical_size: size,
            phantom: PhantomData,
        }
    }

    pub fn write(&self, queue: &wgpu::Queue, offset: BytesAddress, data: &[u8]) {
        let Some(size) = wgpu::BufferSize::new(data.len() as u64) else {
            // empty writes are no-op
            return;
        };

        // if the data length is not aligned to 4, pad the write
        // the buffer should be large enough (its size alignment is validated at creation)
        let write_size = wgpu::BufferSize::new(wgpu::util::align_to(
            size.get(),
            RawMarker::OFFSET_ALIGNMENT.get(),
        ))
        .unwrap();

        let mut staging = queue
            .write_buffer_with(self.ownership.get(), offset.get(), write_size)
            .expect("failed to write buffer");

        staging[..data.len()].copy_from_slice(data);
    }

    pub fn as_buffer_ref(&self) -> BufferRef<T> {
        BufferRef {
            buffer: self.ownership.get(),
            offset: self.offset,
            size: self.logical_size,
            phantom: PhantomData,
        }
    }

    pub fn raw_bytes_size(&self) -> BytesAddress {
        self.logical_size
    }
}

impl<O: BufferOwnership, T: ArrayBufferType> Buffer<O, T> {
    pub fn as_sliced_buffer_ref(&self, offset: usize, size: usize) -> BufferRef<T> {
        let element_size = size_of::<T::Element>();

        // convert array offset and size into bytes
        let offset = BytesAddress::from_usize(offset * element_size);
        let size = BytesAddress::from_usize(size * element_size);

        // check if we are within the bounds of the buffer
        assert!((BytesAddress::ZERO..self.logical_size).contains(&offset));
        assert!((BytesAddress::ZERO..=self.logical_size).contains(&(offset + size)));

        let new_offset = self.offset + offset;

        BufferRef {
            buffer: self.ownership.get(),
            offset: new_offset,
            size,
            phantom: PhantomData,
        }
    }

    pub fn count(&self) -> usize {
        self.logical_size.get() as usize / size_of::<T::Element>()
    }
}

// TODO: these ops only make sense for buffers that are mapped (or at least are allowed to be mapped
// maybe we should enforce this on the level of types?
impl<'a, T: BufferType> BufferRef<'a, T> {
    fn get_range(&self) -> impl std::ops::RangeBounds<wgpu::BufferAddress> {
        self.offset.get()..(self.offset.get() + self.size.get())
    }

    pub fn as_wgpu_slice(&self) -> wgpu::BufferSlice<'a> {
        self.buffer.slice(self.get_range())
    }

    pub fn as_buffer_binding(&self) -> wgpu::BufferBinding {
        let offset = self.offset.get();
        let size = self.size.get();

        wgpu::BufferBinding {
            buffer: self.buffer,
            offset,
            size: Some(wgpu::BufferSize::new(size).unwrap()),
        }
    }

    pub fn get_mapped_range(&self) -> wgpu::BufferView<'a> {
        // TODO: the call to `as_wgpu_slice` could go on the next version of wgpu since the merge of https://github.com/gfx-rs/wgpu/pull/7123
        self.as_wgpu_slice().get_mapped_range()
    }

    pub fn get_mapped_range_mut(&self) -> wgpu::BufferViewMut<'a> {
        self.as_wgpu_slice().get_mapped_range_mut()
    }

    pub fn into_parts(self) -> (&'a wgpu::Buffer, BytesAddress, BytesAddress) {
        (self.buffer, self.offset, self.size)
    }
}

impl<T: ArrayBufferType> BufferRef<'_, T> {
    pub fn count(&self) -> usize {
        self.size.get() as usize / size_of::<T::Element>()
    }
}

impl<O: BufferOwnership, T: BufferType> RenderClone for Buffer<O, T>
where
    O: RenderClone,
{
    fn render_clone(&self, ctx: &mut RenderCloneCtx) -> Self {
        match self {
            &Buffer {
                ref ownership,
                offset,
                logical_size: size,
                phantom,
            } => Buffer {
                ownership: RenderClone::render_clone(ownership, ctx),
                offset,
                logical_size: size,
                phantom,
            },
        }
    }
}

pub type OwnedBuffer<T> = Buffer<Owned, T>;
pub type SharedBuffer<T> = Buffer<Shared, T>;
pub type AnyBuffer<T> = Buffer<AnyOwnership, T>;

pub type OwnedVertexBuffer<T> = OwnedBuffer<VertexMarker<T>>;
pub type OwnedIndexBuffer = OwnedBuffer<IndexMarker>;

pub type AnyVertexBuffer<T> = AnyBuffer<VertexMarker<T>>;
pub type AnyIndexBuffer = AnyBuffer<IndexMarker>;

pub type VertexBufferRef<'a, T> = BufferRef<'a, VertexMarker<T>>;
pub type IndexBufferRef<'a> = BufferRef<'a, IndexMarker>;

impl<O: BufferOwnership> Buffer<O, RawMarker> {
    pub fn slice_bytes(&self, start: BytesAddress, size: BytesAddress) -> BufferRef<RawMarker> {
        let ownership = &self.ownership;

        let offset = self.offset + start;

        assert!((self.offset..self.offset + self.logical_size).contains(&offset));
        assert!((self.offset..=self.offset + self.logical_size).contains(&(offset + size)));

        assert!(RawMarker::is_valid_offset(offset));
        assert!(RawMarker::is_valid_logical_size(size));

        BufferRef {
            buffer: ownership.get(),
            offset,
            size,
            phantom: PhantomData,
        }
    }
}

impl<T: BufferType> OwnedBuffer<T> {
    pub fn map_async(
        self,
        mode: wgpu::MapMode,
        callback: impl FnOnce(Self, Result<(), wgpu::BufferAsyncError>) + Send + 'static,
    ) where
        Self: Send + 'static,
    {
        self.ownership
            .get()
            .clone()
            .slice(..)
            .map_async(mode, |result| callback(self, result))
    }

    pub fn unmap(&self) {
        self.ownership.get().unmap();
    }
}

impl<T: ArrayBufferType> OwnedBuffer<T> {
    pub fn allocate_with_array_contents(
        device: &wgpu::Device,
        data: &[T::Element],
        usage: BufferUsage,
        label: Option<&str>,
    ) -> Self {
        let data: &[u8] = bytemuck::cast_slice(data);

        Self::allocate_raw_with_contents(device, data, usage, label)
    }
}

impl<T: VertexType> OwnedBuffer<VertexMarker<T>> {
    pub fn allocate_vertex(device: &wgpu::Device, data: &[T], label: Option<&str>) -> Self {
        Self::allocate_with_array_contents(device, data, BufferUsage::Vertex, label)
    }
}

impl OwnedBuffer<IndexMarker> {
    pub fn allocate_index(device: &wgpu::Device, data: &[u16], label: Option<&str>) -> Self {
        Self::allocate_with_array_contents(device, data, BufferUsage::Index, label)
    }
}

impl<T: BufferType> From<OwnedBuffer<T>> for AnyBuffer<T> {
    fn from(value: OwnedBuffer<T>) -> Self {
        AnyBuffer {
            ownership: AnyOwnership::Owned(Box::new(value.ownership)),
            offset: value.offset,
            logical_size: value.logical_size,
            phantom: Default::default(),
        }
    }
}

impl<T: BufferType> From<SharedBuffer<T>> for AnyBuffer<T> {
    fn from(value: SharedBuffer<T>) -> Self {
        AnyBuffer {
            ownership: AnyOwnership::Shared(Box::new(value.ownership)),
            offset: value.offset,
            logical_size: value.logical_size,
            phantom: Default::default(),
        }
    }
}

impl<O: BufferOwnership> Buffer<O, RawMarker> {
    pub fn downcast<T: BufferType>(self) -> Buffer<O, T> {
        assert!(T::is_valid_offset(self.offset));
        assert!(T::is_valid_logical_size(self.logical_size));
        Buffer {
            ownership: self.ownership,
            offset: self.offset,
            logical_size: self.logical_size,
            phantom: PhantomData,
        }
    }
}

impl<'a> BufferRef<'a, RawMarker> {
    pub fn downcast<T: BufferType>(self) -> BufferRef<'a, T> {
        assert!(T::is_valid_offset(self.offset));
        assert!(T::is_valid_logical_size(self.size));
        BufferRef {
            buffer: self.buffer,
            offset: self.offset,
            size: self.size,
            phantom: PhantomData,
        }
    }
}

#[derive(Debug)]
pub enum VertexSource<'a, T: VertexType> {
    VertexBuffer {
        vertices: VertexBufferRef<'a, T>,
    },
    VertexAndIndexBuffer {
        vertices: VertexBufferRef<'a, T>,
        indices: IndexBufferRef<'a>,
    },
    VertexData {
        vertices: &'a [T],
    },
    VertexAndIndexData {
        vertices: &'a [T],
        indices: &'a [u16],
    },
}

/// Information necessary to make a right call to `draw` or `draw_indexed` after binding the vertex source.
#[derive(Debug)]
pub enum VertexSourceInfo {
    VertexBuffer { vertex_count: u32 },
    VertexAndIndexBuffer { index_count: u32 },
}

impl<T: VertexType> VertexSource<'_, T> {
    pub fn info(&self) -> VertexSourceInfo {
        match self {
            VertexSource::VertexBuffer {
                vertices: vertex_buffer,
            } => VertexSourceInfo::VertexBuffer {
                vertex_count: vertex_buffer.count() as u32,
            },
            VertexSource::VertexAndIndexBuffer {
                vertices: _,
                indices: index_buffer,
            } => VertexSourceInfo::VertexAndIndexBuffer {
                index_count: index_buffer.count() as u32,
            },
            VertexSource::VertexData {
                vertices: vertex_data,
            } => VertexSourceInfo::VertexBuffer {
                vertex_count: vertex_data.len() as u32,
            },
            VertexSource::VertexAndIndexData {
                vertices: _,
                indices: index_data,
            } => VertexSourceInfo::VertexAndIndexBuffer {
                index_count: index_data.len() as u32,
            },
        }
    }

    pub fn bind(
        &self,
        dynamic_buffer: &mut impl DynamicBufferBackend,
        pass: &mut wgpu::RenderPass,
    ) {
        match self {
            VertexSource::VertexBuffer {
                vertices: vertex_buffer,
            } => {
                pass.set_vertex_buffer(0, vertex_buffer.as_wgpu_slice());
            }
            VertexSource::VertexAndIndexBuffer {
                vertices: vertex_buffer,
                indices: index_buffer,
            } => {
                pass.set_vertex_buffer(0, vertex_buffer.as_wgpu_slice());
                pass.set_index_buffer(index_buffer.as_wgpu_slice(), wgpu::IndexFormat::Uint16);
            }
            VertexSource::VertexData {
                vertices: vertex_data,
            } => {
                let vertex_buffer = dynamic_buffer.get_vertex_with_data(vertex_data);
                pass.set_vertex_buffer(0, vertex_buffer.as_wgpu_slice());
            }
            VertexSource::VertexAndIndexData {
                vertices: vertex_data,
                indices: index_data,
            } => {
                let vertex_buffer = dynamic_buffer.get_vertex_with_data(vertex_data);
                pass.set_vertex_buffer(0, vertex_buffer.as_wgpu_slice());
                let index_buffer = dynamic_buffer.get_index_with_data(index_data);
                pass.set_index_buffer(index_buffer.as_wgpu_slice(), wgpu::IndexFormat::Uint16);
            }
        }
    }
}
