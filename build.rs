use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        println!("cargo:warning=Windows version resources are skipped for non-Windows targets");
        return;
    }

    if let Err(error) = embed_version_resources() {
        panic!("could not embed Windows version information: {error}");
    }
}

fn embed_version_resources() -> io::Result<()> {
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("Cargo always sets OUT_DIR"));
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("Cargo always sets CARGO_MANIFEST_DIR"),
    );

    let icon = manifest_dir.join("assets").join("mchose.ico");

    let compiler = find_resource_compiler()?;
    let version = numeric_version(&env::var("CARGO_PKG_VERSION").unwrap_or_default());

    println!("cargo:rerun-if-changed={}", icon.display());

    for (binary, description) in [
        ("mchose-tray", "MCHOSE Battery Tray"),
        ("mchose-check", "MCHOSE Battery Diagnostic"),
    ] {
        let rc = output.join(format!("{binary}.rc"));
        let res = output.join(format!("{binary}.res"));

        fs::write(&rc, version_resource(binary, description, version, &icon))?;

        let status = Command::new(&compiler)
            .args(["/nologo", "/fo"])
            .arg(&res)
            .arg(&rc)
            .status()?;

        if !status.success() {
            return Err(io::Error::other(format!(
                "{} failed with {status}",
                compiler.display()
            )));
        }

        println!("cargo:rustc-link-arg-bin={binary}={}", res.display());
    }

    Ok(())
}

fn numeric_version(version: &str) -> [u16; 4] {
    let mut parts = [0; 4];
    for (slot, part) in parts.iter_mut().zip(version.split(['.', '-'])) {
        *slot = part.parse().unwrap_or(0);
    }
    parts
}

fn version_resource(binary: &str, description: &str, version: [u16; 4], icon: &Path) -> String {
    let dotted = format!(
        "{}.{}.{}.{}",
        version[0], version[1], version[2], version[3]
    );

    let comma = format!(
        "{},{},{},{}",
        version[0], version[1], version[2], version[3]
    );

    // rc.exe accepts forward slashes, which avoids having to escape
    // Windows backslashes inside the .rc file.
    let icon = icon.to_string_lossy().replace('\\', "/");

    format!(
        r#"1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
FILEFLAGSMASK 0x3fL
FILEFLAGS 0
FILEOS 0x00040004L
FILETYPE 0x00000001L
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "FileDescription", "{description}\0"
            VALUE "FileVersion", "{dotted}\0"
            VALUE "InternalName", "{binary}\0"
            VALUE "OriginalFilename", "{binary}.exe\0"
            VALUE "ProductName", "MCHOSE Battery\0"
            VALUE "ProductVersion", "{dotted}\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    )
}

fn find_resource_compiler() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("RC") {
        return Ok(PathBuf::from(path));
    }

    let target_dir = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("x86_64") => "x64",
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };
    let kits = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"))
        .join(r"Windows Kits\10\bin");
    let mut versions = fs::read_dir(&kits)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| right.file_name().cmp(&left.file_name()));
    versions
        .into_iter()
        .map(|version| version.join(target_dir).join("rc.exe"))
        .find(|path| Path::new(path).is_file())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Windows SDK rc.exe was not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_semver_to_windows_version() {
        assert_eq!(numeric_version("1.2.3"), [1, 2, 3, 0]);
        assert_eq!(numeric_version("1.2.3-beta.1"), [1, 2, 3, 0]);
    }
}
