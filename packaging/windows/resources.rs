use std::{env, io, path::PathBuf, process::Command};

/// Embed the Explorer icon into the main executable, independently of Slint's window icon.
pub fn compile() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=packaging/windows/resources.rs");
    if env::var("CARGO_CFG_TARGET_OS")? != "windows" {
        return Ok(());
    }
    println!("cargo:rerun-if-changed=packaging/windows/syncpak.rc");
    println!("cargo:rerun-if-changed=packaging/windows/syncpak.ico");
    println!("cargo:rerun-if-env-changed=RC");
    let output = PathBuf::from(env::var_os("OUT_DIR").ok_or("OUT_DIR is missing")?);
    let msvc = env::var("CARGO_CFG_TARGET_ENV")? == "msvc";
    let resource = output.join(if msvc { "syncpak.res" } else { "syncpak.o" });
    let compiler = env::var_os("RC").map(PathBuf::from).unwrap_or_else(|| {
        if msvc {
            windows_sdk_rc().unwrap_or_else(|| "rc.exe".into())
        } else {
            let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
            format!("{arch}-w64-mingw32-windres").into()
        }
    });
    let mut command = Command::new(&compiler);
    command.current_dir("packaging/windows");
    if msvc {
        command
            .arg("/nologo")
            .arg("/fo")
            .arg(&resource)
            .arg("syncpak.rc");
    } else {
        command.arg("-i").arg("syncpak.rc").arg("-o").arg(&resource);
    }
    if !command.status()?.success() {
        return Err(io::Error::other("Windows icon resource compilation failed").into());
    }
    println!("cargo:rustc-link-arg-bin=sync-pak={}", resource.display());
    Ok(())
}

/// Cargo may run outside a Visual Studio developer prompt; find the SDK tool directly.
fn windows_sdk_rc() -> Option<PathBuf> {
    let kits = PathBuf::from(env::var_os("ProgramFiles(x86)")?).join("Windows Kits/10/bin");
    let mut versions = std::fs::read_dir(kits)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    versions.sort();
    let host = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    versions
        .into_iter()
        .rev()
        .map(|path| path.join(host).join("rc.exe"))
        .find(|path| path.is_file())
}
