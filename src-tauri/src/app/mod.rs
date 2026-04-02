pub mod chord_package_manager;
pub mod chord_package_registry;
pub mod chord_package_store;
pub mod chord_runner;
pub mod chorder;
pub mod context;
pub mod desktop_app;
pub mod dev_lockfile_detector;
pub mod frontmost;
pub mod git_repos_store;
pub mod global_hotkey_store;
pub mod permissions;
pub mod placeholder_chord_store;
pub mod settings;
pub mod state;

macro_rules! define_app_managed {
    (
        $(
            $field:ident : $ty:ty => $getter:ident
        ),+ $(,)?
    ) => {
        #[allow(dead_code)]
        pub struct AppManaged {
            $(
                pub $field: $ty,
            )+
        }

        impl AppManaged {
            #[allow(dead_code)]
            pub fn register<R: ::tauri::Runtime>(self, handle: &::tauri::AppHandle<R>) {
                $(
                    let _ = ::tauri::Manager::manage(handle, self.$field);
                )+
            }
        }

        #[allow(dead_code)]
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
                Ok(::tauri::Manager::state::<T>(self).get_state()?)
            }
        }
    };
}

define_app_managed! {
    chord_action_task_runner: self::chord_runner::ChordActionTaskRunner => chord_action_task_runner,
    chord_package_manager: self::chord_package_manager::ChordPackageManager => chord_package_manager,
    dev_lockfile_detector: self::dev_lockfile_detector::DevLockfileDetector => app_dev_lockfile_detector,
    desktop_app_manager: self::desktop_app::DesktopAppManager => desktop_app_manager,
    settings: self::settings::AppSettings => app_settings,
    chorder: self::chorder::AppChorder => app_chorder,
    context: self::context::AppContext => app_context,
    frontmost: self::frontmost::AppFrontmost => app_frontmost,
    permissions: self::permissions::AppPermissions => app_permissions,
    global_hotkey_store: self::global_hotkey_store::GlobalHotkeyStore => app_global_hotkey_store,
    placeholder_chord_store: self::placeholder_chord_store::PlaceholderChordStore => app_placeholder_chord_store,
    git_repos_store: self::git_repos_store::GitReposStore => app_git_repos_store,
}
