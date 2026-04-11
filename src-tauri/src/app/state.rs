use tauri::AppHandle;
use anyhow::Result;

pub trait AppSingleton<T>: Send + Sync + 'static {
    fn new(handle: AppHandle) -> Self;
    fn init(&self, init: T) -> Result<()>;
}

macro_rules! define_app_state {
    (
        $(
            $field:ident : $ty:ty
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

        /// A proxy struct to namespace the app-managed state getters.
        #[allow(dead_code)]
        pub struct AppAccessor<'a, R: ::tauri::Runtime> {
            handle: &'a ::tauri::AppHandle<R>,
        }

        impl<'a, R: ::tauri::Runtime> AppAccessor<'a, R> {
            $(
                #[allow(dead_code)]
                pub fn $field(&self) -> &'a $ty {
                    ::tauri::Manager::state::<$ty>(self.handle).inner()
                }
            )+
        }

        #[allow(dead_code)]
        pub trait AppHandleExt {
            /// Associated runtime type to avoid cluttering the trait with generics.
            type Runtime: ::tauri::Runtime;

            /// Returns an accessor to retrieve app-managed states.
            fn state(&self) -> AppAccessor<'_, Self::Runtime>;

            fn observable_state<T: $crate::state::Observable>(
                &self,
            ) -> ::anyhow::Result<::std::sync::Arc<T::State>>;
        }

        impl<R: ::tauri::Runtime> AppHandleExt for ::tauri::AppHandle<R> {
            type Runtime = R;

            fn state(&self) -> AppAccessor<'_, R> {
                AppAccessor { handle: self }
            }

            fn observable_state<T: $crate::state::Observable>(
                &self,
            ) -> ::anyhow::Result<::std::sync::Arc<T::State>> {
                Ok(::tauri::Manager::state::<T>(self).get_state()?)
            }
        }
    };
}

define_app_state! {
    chord_input_manager: super::chord_input_manager::AppChordInputManager,
    chord_package_manager: super::chord_package_manager::ChordPackageManager,
    chord_package_registry: super::chord_package_registry::ChordPackageRegistry,
    chord_package_store: super::chord_package_store::ChordPackageStore,
    chord_action_task_runner: super::chord_runner::ChordActionTaskRunner,
    desktop_app_manager: super::desktop_app::DesktopAppManager,
    dev_lockfile_detector: super::dev_lockfile_detector::DevLockfileDetector,
    frontmost: super::frontmost::AppFrontmost,
    git_repos_store: super::git_repos_store::GitReposStore,
    global_hotkey_store: super::global_hotkey_store::GlobalHotkeyStore,
    keyboard: super::keyboard::AppKeyboard,
    mode_manager: super::mode::AppModeManager,
    permissions: super::permissions::AppPermissions,
    placeholder_chord_store: super::placeholder_chord_store::PlaceholderChordStore,
    settings: super::settings::AppSettings,
}
