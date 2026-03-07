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
        ("yuv_to_rgb.comp", shaderc::ShaderKind::Compute),
    ];

    compile_shaders(&compiler, &options, shader_dir, &out_dir, &shaders);

    #[cfg(feature = "live2d")]
    {
        let live2d_shaders = [
            ("live2d/live2d.vert", shaderc::ShaderKind::Vertex),
            ("live2d/live2d.frag", shaderc::ShaderKind::Fragment),
            ("live2d/live2d_mask.vert", shaderc::ShaderKind::Vertex),
            ("live2d/live2d_mask.frag", shaderc::ShaderKind::Fragment),
        ];
        compile_shaders(&compiler, &options, shader_dir, &out_dir, &live2d_shaders);
    }
}

fn compile_shaders(
    compiler: &shaderc::Compiler,
    options: &shaderc::CompileOptions<'_>,
    shader_dir: &Path,
    out_dir: &str,
    shaders: &[(&str, shaderc::ShaderKind)],
) {
    for (filename, kind) in shaders {
        let path = shader_dir.join(filename);
        println!("cargo:rerun-if-changed={}", path.display());

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

        let artifact = compiler
            .compile_into_spirv(&source, *kind, filename, "main", Some(options))
            .unwrap_or_else(|e| panic!("Failed to compile {}: {}", filename, e));

        let spv_name = filename.replace('/', "_");
        let out_path = Path::new(out_dir).join(format!("{}.spv", spv_name));
        fs::write(&out_path, artifact.as_binary_u8())
            .unwrap_or_else(|e| panic!("Failed to write {}: {}", out_path.display(), e));
    }
}
