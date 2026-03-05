use std::fs;
use std::path::Path;

fn main() {
    let compiler = shaderc::Compiler::new().expect("Failed to create shaderc compiler");
    let mut options = shaderc::CompileOptions::new().expect("Failed to create compile options");
    options.set_target_env(
        shaderc::TargetEnv::Vulkan,
        shaderc::EnvVersion::Vulkan1_3 as u32,
    );
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);

    let shader_dir = Path::new("src/shaders");
    let out_dir = std::env::var("OUT_DIR").unwrap();

    let shaders = [
        ("sdf_2d.vert", shaderc::ShaderKind::Vertex),
        ("sdf_2d.frag", shaderc::ShaderKind::Fragment),
    ];

    for (filename, kind) in &shaders {
        let path = shader_dir.join(filename);
        println!("cargo:rerun-if-changed={}", path.display());

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

        let artifact = compiler
            .compile_into_spirv(&source, *kind, filename, "main", Some(&options))
            .unwrap_or_else(|e| panic!("Failed to compile {}: {}", filename, e));

        let out_path = Path::new(&out_dir).join(format!("{}.spv", filename));
        fs::write(&out_path, artifact.as_binary_u8())
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
    }
}
