use crate::AppContext;
use self::global_hotkey::GlobalHotkeyStore;
use self::placeholder_chords::PlaceholderChordStore;
use self::repos::GitReposStore;
pub(crate) use crate::app::{AppChorder, AppFrontmost, AppPermissions, AppSettings};
use crate::chords::ChordRegistry;
use crate::desktop_app::DesktopAppManager;
use crate::registry::ChordPackageRegistry;
use crate::chord_runner::ChordRunner;

pub mod chorder;
pub use chorder::*;
pub mod chorder_ui;
pub use chorder_ui::*;
mod frontmost;
pub use frontmost::*;
pub mod global_hotkey;
pub use global_hotkey::*;
pub mod permissions;
pub use permissions::*;
pub mod placeholder_chords;
pub use placeholder_chords::*;
pub mod repos;
pub use repos::*;
pub mod safe_app_handle;
pub use safe_app_handle::*;
pub mod settings;
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
