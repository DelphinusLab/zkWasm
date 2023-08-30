use std::fs;
use std::process::Command;

fn main() -> Result<(), std::io::Error> {
    fs::create_dir_all("wasm").unwrap();

    // Check clang exists
    let check_clang_exists = Command::new("clang")
        .args(&["-v"])
        .status()
        .map_err(|err| {
            println!("Commang 'clang' not found, it is required to build C to wasm.");
            err
        })?;
    assert!(check_clang_exists.success());

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
                .status()?;

            assert!(exit.success());
        };
    }

    compile_c!("phantom");
    compile_c!("fibonacci");
    compile_c!("binary_search");
    compile_c!("context");

    Ok(())
}
