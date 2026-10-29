//! Per-effect extras for single-shader effects (textures, derived uniforms).

use super::ShaderHook;
use crate::context::GpuContext;
use phthora_core::EffectDesc;

pub fn create_hook(_ctx: &GpuContext, _desc: &EffectDesc) -> Option<Box<dyn ShaderHook>> {
    None
}
