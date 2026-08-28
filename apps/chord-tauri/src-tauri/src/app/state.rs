use anyhow::Result;

pub trait AppSingleton: Send + Sync + 'static {
    fn init(&self) -> Result<()>;
}

macro_rules! define_app_state {
    (
        $(
            $field:ident : $ty:ty
        ),+ $(,)?
    ) => {
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
            fn app_state(&self) -> AppAccessor<'_, Self::Runtime>;
        }

        impl<R: ::tauri::Runtime> AppHandleExt for ::tauri::AppHandle<R> {
            type Runtime = R;

            fn app_state(&self) -> AppAccessor<'_, R> {
                AppAccessor { handle: self }
            }
        }
    };
}

define_app_state! {
    chord_mode_manager: super::chord_mode_manager::ChordModeManager,
    chord_package_manager: super::chord_package_manager::ChordPackageManager,
    chord_package_store: super::chord_package_store::ChordPackageStore,
    chord_action_task_runner: super::chord_runner::ChordActionTaskRunner,
    desktop_app_manager: super::desktop_app::DesktopAppManager,
    dev_lockfile_detector: super::dev_lockfile_detector::DevLockfileDetector,
    frontmost: super::frontmost::AppFrontmost,
    global_hotkey_store: super::global_hotkey_store::GlobalHotkeyStore,
    keyboard: super::keyboard::AppKeyboard,
    app_controller: super::controller::AppController,
    permissions: super::permissions::AppPermissions,
    placeholder_chord_store: super::placeholder_chord_store::PlaceholderChordStore,
    settings: super::settings::AppSettings,
}
