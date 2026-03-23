fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut attributes = tauri_build::Attributes::new();
    #[cfg(windows)]
    {
        attributes = attributes.windows_attributes(
            tauri_build::WindowsAttributes::new_without_app_manifest(),
        );
        add_manifest()?;
    }
    tauri_build::try_build(attributes)?;
    Ok(())
}

#[cfg(windows)]
fn add_manifest() -> Result<(), Box<dyn std::error::Error>> {
    static WINDOWS_MANIFEST_FILE: &str = "windows-app-manifest.xml";
    let manifest = std::env::current_dir()?
        .join(WINDOWS_MANIFEST_FILE);
    println!("cargo:rerun-if-changed={}", manifest.display());
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
        manifest.to_str().ok_or("manifest path is not valid UTF-8")?
    );
    println!("cargo:rustc-link-arg=/WX");
    Ok(())
}
