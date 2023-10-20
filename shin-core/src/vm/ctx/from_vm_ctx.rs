//! Defines `FromVmCtx` and `FromVmCtxDefault` traits, that are used to convert from compile-time (e.g. `NumberSpec`) to runtime (e.g. `i32`) representations of command parameters
//!
//! Also contains implementation for std types & stuff defined in `shin_core::format`, like `U8String` -> `String` stuff

use crate::format::scenario::instruction_elements::MessageId;
use crate::format::text::{StringArray, U16FixupString, U16String, U8FixupString, U8String};
use crate::vm::VmCtx;
use smallvec::SmallVec;

/// Defines how to convert a compile-time representation `I` to a runtime representation `Self`
///
/// For example this is used to convey that a NumberSpec can be converted to i32 (by inspecting the VmCtx)
pub trait FromVmCtx<I>
where
    Self: Sized,
{
    fn from_vm_ctx(ctx: &VmCtx, input: I) -> Self;
}

/// Defines the default conversion from VmCtx
///
/// For example this is used to convey that a NumberSpec is usually converted to i32
pub trait FromVmCtxDefault
where
    Self: Sized,
{
    type Output: FromVmCtx<Self>;
    fn from_vm_ctx(ctx: &VmCtx, input: Self) -> Self::Output {
        FromVmCtx::<Self>::from_vm_ctx(ctx, input)
    }
}

macro_rules! identity_from_vm_ctx {
    ($($t:ty),*) => {
        $(
            impl FromVmCtx<$t> for $t {
                fn from_vm_ctx(_: &VmCtx, input: $t) -> Self {
                    input
                }
            }
        )*
    };
}

macro_rules! identity_from_vm_ctx_default {
    ($($t:ty),*) => {
        $(
            identity_from_vm_ctx!($t);
            impl FromVmCtxDefault for $t {
                type Output = $t;
            }
        )*
    };
}

identity_from_vm_ctx_default!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64, MessageId);

impl FromVmCtx<U8String> for String {
    fn from_vm_ctx(_: &VmCtx, input: U8String) -> Self {
        input.0
    }
}
impl FromVmCtxDefault for U8String {
    type Output = String;
}

impl FromVmCtx<U16String> for String {
    fn from_vm_ctx(_: &VmCtx, input: U16String) -> Self {
        input.0
    }
}
impl FromVmCtxDefault for U16String {
    type Output = String;
}

impl FromVmCtx<U8FixupString> for String {
    fn from_vm_ctx(_: &VmCtx, input: U8FixupString) -> Self {
        input.0
    }
}
impl FromVmCtxDefault for U8FixupString {
    type Output = String;
}

impl FromVmCtx<U16FixupString> for String {
    fn from_vm_ctx(_: &VmCtx, input: U16FixupString) -> Self {
        input.0
    }
}
impl FromVmCtxDefault for U16FixupString {
    type Output = String;
}

impl FromVmCtx<StringArray> for SmallVec<String, 4> {
    fn from_vm_ctx(_: &VmCtx, input: StringArray) -> Self {
        input.0
    }
}
impl FromVmCtxDefault for StringArray {
    type Output = SmallVec<String, 4>;
}
