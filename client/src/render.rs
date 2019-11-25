use log::info;
use std::path::Path;

/// Load a GLSL shader from a file and compile it to SPIR-V
pub fn load_glsl_shader<P: AsRef<Path>>(compiler: &mut shaderc::Compiler, shader_kind: shaderc::ShaderKind, path: P) -> shaderc::CompilationArtifact {
    let path_display = path.as_ref().display().to_string();
    info!("Loading GLSL shader from {}", path_display);
    let glsl_source = std::fs::read_to_string(path).expect("Couldn't read shader from file");

    // TODO: handle warnings
    compiler.compile_into_spirv(
        &glsl_source,
        shader_kind,
        &path_display,
        &"main",
        None,
    ).expect("Failed to compile GLSL shader into SPIR-V shader")
}