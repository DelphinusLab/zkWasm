use std::fs;
use std::process::Command;

fn main() {
    fs::create_dir_all("wasm").unwrap();

    macro_rules! compile_c {
        ($file: expr) => {
            let exit = Command::new("clang")
                .args(&[
                    "-O3",
                    "--target=wasm32",
                    "-nostdlib",
                    "-fno-builtin",
                    "-Wl,--export-all",
                    "-Wl,--no-entry",
                    "-Wl,--allow-undefined",
                    "-Wl,--export-dynamic",
                    &format!("-owasm/{}.wasm", $file),
                    &format!("c/{}.c", $file),
                ])
                .status()
                .unwrap();

            assert!(exit.success());
        };
    }

    compile_c!("phantom");
    compile_c!("fibonacci");
    compile_c!("binary_search");
    compile_c!("context_cont");
}
