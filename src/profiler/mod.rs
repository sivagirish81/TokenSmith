use anyhow::Result;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod types;
#[cfg(target_os = "windows")]
pub mod windows;

pub fn profile_hardware() -> Result<types::HardwareProfile> {
    if let Ok(raw) = std::env::var("TOKENSMITH_TEST_PROFILE_JSON") {
        let profile = serde_json::from_str::<types::HardwareProfile>(&raw)?;
        return Ok(profile);
    }

    #[cfg(target_os = "macos")]
    {
        return macos::profile();
    }
    #[cfg(target_os = "linux")]
    {
        return linux::profile();
    }
    #[cfg(target_os = "windows")]
    {
        return windows::profile();
    }
    #[allow(unreachable_code)]
    Err(anyhow::anyhow!("unsupported platform"))
}
