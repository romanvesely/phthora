//! Multi-pass effects built from several fullscreen shaders.

use super::GpuEffect;
use crate::context::GpuContext;
use crate::shader::ShaderLibrary;
use phthora_core::EffectDesc;

pub fn create_custom(_ctx: &GpuContext, _lib: &ShaderLibrary, _desc: &EffectDesc) -> Option<Box<dyn GpuEffect>> {
    None
}
