use crate::AppContext;
use crate::chord_runner::ChordRunner;
use crate::chords::ChordRegistry;
use crate::app::global_hotkey::GlobalHotkeyStore;
use crate::app::placeholder_chords::PlaceholderChordStore;
use crate::app::repos::GitReposStore;
use crate::app::{AppChorder, AppFrontmost, AppPermissions, AppSettings};
use crate::desktop_app::DesktopAppManager;
use crate::registry::ChordPackageRegistry;

mod chorder;
mod chorder_ui;
mod frontmost;
pub mod global_hotkey;
mod permissions;
pub mod placeholder_chords;
pub mod repos;
mod safe_app_handle;
mod settings;

pub use chorder::*;
pub use chorder_ui::*;
pub use frontmost::*;
pub use permissions::*;
pub use safe_app_handle::*;
pub use settings::*;

crate::define_app_managed! {
    settings: AppSettings => app_settings,
    chorder: AppChorder => app_chorder,
    context: AppContext => app_context,
    chord_package_registry: ChordPackageRegistry => app_chord_package_registry,
    frontmost: AppFrontmost => app_frontmost,
    permissions: AppPermissions => app_permissions,
    global_hotkey_store: GlobalHotkeyStore => app_global_hotkey_store,
    placeholder_chord_store: PlaceholderChordStore => app_placeholder_chord_store,
    git_repos_store: GitReposStore => app_git_repos_store,
    chord_registry: ChordRegistry => app_chord_registry,
    desktop_app_manager: DesktopAppManager => desktop_app_manager,
    chord_runner: ChordRunner => chord_runner
}

#[macro_export]
macro_rules! define_app_managed {
    (
        $(
            $field:ident : $ty:ty => $getter:ident
        ),+ $(,)?
    ) => {
        pub struct AppManaged {
            $(
                pub $field: $ty,
            )+
        }

        impl AppManaged {
            pub fn register<R: ::tauri::Runtime>(self, handle: &::tauri::AppHandle<R>) {
                $(
                    let _ = ::tauri::Manager::manage(handle, self.$field);
                )+
            }
        }

        pub trait AppHandleExt {
            $(
                fn $getter(&self) -> &$ty;
            )+

            fn observable_state<T: $crate::observables::Observable>(
                &self,
            ) -> ::anyhow::Result<::std::sync::Arc<T::State>>;
        }

        impl<R: ::tauri::Runtime> AppHandleExt for ::tauri::AppHandle<R> {
            $(
                fn $getter(&self) -> &$ty {
                    ::tauri::Manager::state::<$ty>(self).inner()
                }
            )+

            fn observable_state<T: $crate::observables::Observable>(
                &self,
            ) -> ::anyhow::Result<::std::sync::Arc<T::State>> {
                Ok(::tauri::Manager::state::<::std::sync::Arc<T>>(self).inner().get_state()?)
            }
        }
    };
}
