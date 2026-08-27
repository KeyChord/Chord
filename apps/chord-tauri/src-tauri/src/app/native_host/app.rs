use crate::app::native_host::NativeHostSupervisor;
use crate::app::state::AppSingleton;
use anyhow::Result;
use nject::provider;
use tauri::AppHandle;

#[provider]
pub struct NativeHostSupervisorProvider {
    #[provide(AppHandle, |v| v.clone())]
    pub handle: AppHandle,
}

impl AppSingleton for NativeHostSupervisor {
    fn init(&self) -> Result<()> {
        Ok(())
    }
}
