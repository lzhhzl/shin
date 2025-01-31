use std::sync::Arc;

use crate::{RenderClone, RenderCloneCtx};

pub trait BufferOwnership {
    fn new(buffer: wgpu::Buffer) -> Self;
    fn get(&self) -> &wgpu::Buffer;
}

#[derive(Debug)]
pub struct Owned(wgpu::Buffer);

#[derive(Debug, Clone)]
pub struct Shared(Arc<wgpu::Buffer>);

#[derive(Debug)]
pub enum AnyOwnership {
    Owned(Box<Owned>),
    Shared(Shared),
}

impl BufferOwnership for Owned {
    fn new(buffer: wgpu::Buffer) -> Self {
        Self(buffer)
    }

    fn get(&self) -> &wgpu::Buffer {
        &self.0
    }
}

impl RenderClone for Owned {
    fn render_clone(&self, ctx: &mut RenderCloneCtx) -> Self {
        Self(self.0.render_clone(ctx))
    }
}

impl BufferOwnership for Shared {
    fn new(buffer: wgpu::Buffer) -> Self {
        Self(Arc::new(buffer))
    }

    fn get(&self) -> &wgpu::Buffer {
        &self.0
    }
}

impl RenderClone for Shared {
    fn render_clone(&self, _: &mut RenderCloneCtx) -> Self {
        self.clone()
    }
}

impl BufferOwnership for AnyOwnership {
    fn new(_buffer: wgpu::Buffer) -> Self {
        panic!("Do not create a buffer with AnyOwnership directly, use a specific type instead")
    }

    fn get(&self) -> &wgpu::Buffer {
        match self {
            AnyOwnership::Owned(owned) => owned.get(),
            AnyOwnership::Shared(shared) => shared.get(),
        }
    }
}

impl RenderClone for AnyOwnership {
    fn render_clone(&self, ctx: &mut RenderCloneCtx) -> Self {
        match self {
            AnyOwnership::Owned(o) => AnyOwnership::Owned(o.render_clone(ctx)),
            AnyOwnership::Shared(s) => AnyOwnership::Shared(s.render_clone(ctx)),
        }
    }
}
