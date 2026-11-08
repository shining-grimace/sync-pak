#[path = "packaging/windows/resources.rs"]
mod windows_resources;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    slint_build::compile("ui/app.slint")?;
    windows_resources::compile()?;
    Ok(())
}
