use crate::common::*;

const INCLUDE_DIRS: &[&str] = &[
    "libffi",
    "libffi/include",
    "include/msvc",
    // libffi expects us to include the same folder in case of x86 and x86_64 architectures
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    "libffi/src/x86",
    #[cfg(target_arch = "aarch64")]
    "libffi/src/aarch64",
];

const BUILD_FILES: &[&str] = &[
    "tramp.c",
    "closures.c",
    "prep_cif.c",
    "raw_api.c",
    "types.c",
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    "x86/ffi.c",
    #[cfg(any(target_arch = "x86_64"))]
    "x86/ffiw64.c",
    #[cfg(target_arch = "aarch64")]
    "aarch64/ffi.c",
];

fn add_file(build: &mut cc::Build, file: &str) {
    build.file(format!("libffi/src/{}", file));
}

pub fn build_and_link() {
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let asm_path = pre_process_asm(INCLUDE_DIRS, target_arch.as_str());
    let mut build = cc::Build::new();

    for inc in INCLUDE_DIRS {
        build.include(inc);
    }

    for file in BUILD_FILES {
        add_file(&mut build, file);
    }

    build
        .file(asm_path)
        .define("WIN32", None)
        .define("_LIB", None)
        .define("FFI_BUILDING", None)
        .warnings(false)
        .compile("libffi");
}

pub fn probe_and_link() {
    // At the time of writing it wasn't clear if MSVC builds will support
    // dynamic linking of libffi; assuming it's even installed. To ensure
    // existing MSVC setups continue to work, we just compile libffi from source
    // and statically link it.
    build_and_link();
}

pub fn pre_process_asm(include_dirs: &[&str], target_arch: &str) -> String {
    let folder_name = match target_arch {
        "x86_64" => "x86",
        "x86" => "x86",
        "aarch64" => "aarch64",
        _ => panic!("Unsupported arch: {}", target_arch),
    };

    let file_name = match target_arch {
        "x86_64" => "win64_intel",
        "x86" => "sysv_intel",
        "aarch64" => "win64_armasm",
        _ => panic!("Unsupported arch: {}", target_arch),
    };

    let mut cmd = cc::windows_registry::find(&env::var("TARGET").unwrap(), "cl.exe")
        .expect("Could not locate cl.exe");

    let build = cc::Build::new();
    for (key, value) in build.get_compiler().env() {
        if key.to_string_lossy().to_string() == "INCLUDE".to_string() {
            cmd.env(
                "INCLUDE",
                format!("{};{}", value.to_string_lossy(), include_dirs.join(";")),
            );
        }
    }

    cmd.arg("/EP");
    cmd.arg(format!("libffi/src/{}/{}.S", folder_name, file_name));

    let out_path = format!("libffi/src/{}/{}.asm", folder_name, file_name);
    let asm_file = fs::File::create(&out_path).expect("Could not create output file");

    cmd.stdout(asm_file);

    run_command("Pre-process ASM", &mut cmd);

    out_path
}
