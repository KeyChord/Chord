use std::collections::HashMap;
use tauri::AppHandle;
mod chord_package_manager;
pub use chord_package_manager::*;
mod chorder;
pub use chorder::*;
mod desktop_app_manager;
pub use desktop_app_manager::*;
mod frontmost;
pub use frontmost::*;
mod git_repos;
pub use git_repos::*;
mod permissions;
pub use permissions::*;
mod settings;
pub use settings::*;

pub struct ObservableRegistration {
    pub id: &'static str,
    pub get_json: fn(&AppHandle) -> anyhow::Result<serde_json::Value>,
}

inventory::collect!(ObservableRegistration);

pub trait Observable: Sized + Send + Sync + 'static {
    type State: Default + serde::Serialize + Send + Sync + 'static;

    const ID: &'static str;
    const EVENT: &'static str;

    fn get_state(&self) -> anyhow::Result<std::sync::Arc<Self::State>>;
    fn set_state(&self, state: Self::State) -> anyhow::Result<()>;
    fn subscribe(
        &self,
        observer: observable_property::Observer<std::sync::Arc<Self::State>>,
    ) -> anyhow::Result<observable_property::ObserverId>;

    fn placeholder() -> Self;
    fn new(handle: AppHandle) -> anyhow::Result<Self>;
    fn init(&self, observable: Self);
}

pub fn get_all_observable_states(
    handle: AppHandle,
) -> anyhow::Result<HashMap<&'static str, serde_json::Value>> {
    inventory::iter::<ObservableRegistration>
        .into_iter()
        .map(|reg| Ok((reg.id, (reg.get_json)(&handle)?)))
        .collect()
}

#[macro_export]
macro_rules! define_observable {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident($state:ty);
        id: $id:literal $(;)?
    ) => {
        use tauri::Emitter;
        use crate::app::AppHandleExt;

        #[derive(Clone)]
        $(#[$meta])*
        $vis struct $name {
            state: ::std::sync::Arc<::arc_swap::ArcSwap<::std::option::Option<::observable_property::ObservableProperty<::std::sync::Arc<$state>>>>>,
        }

        impl $name {
        }

        impl $crate::observables::Observable for $name {
            type State = $state;

            const ID: &'static str = $id;
            const EVENT: &'static str = ::std::concat!("state:", $id);

            fn placeholder() -> Self {
                Self { state: ::std::sync::Arc::new(::arc_swap::ArcSwap::new(::std::sync::Arc::new(::std::option::Option::None))) }
            }

            fn init(&self, observable: Self) {
                self.state.store(observable.state.load_full())
            }

            fn get_state(&self) -> ::anyhow::Result<::std::sync::Arc<Self::State>> {
                if let Some(self_state) = self.state.load().as_ref() {
                    Ok(self_state.get()?)
                } else {
                    anyhow::bail!("no state")
                }
            }

            fn set_state(&self, state: Self::State) -> ::anyhow::Result<()> {
                if let Some(self_state) = self.state.load().as_ref() {
                    Ok(self_state.set(::std::sync::Arc::new(state))?)
                } else {
                    anyhow::bail!("no state")
                }
            }

            fn new(handle: ::tauri::AppHandle) -> ::anyhow::Result<Self> {
                let state = <Self::State as ::std::default::Default>::default();
                let state =
                    ::observable_property::ObservableProperty::new(::std::sync::Arc::new(state));

                state.subscribe(::std::sync::Arc::new(move |_, new_state| {
                    if let Err(e) = handle.emit(Self::EVENT, new_state.as_ref()) {
                        ::log::error!(
                            "Failed to emit {} for {}: {}",
                            Self::EVENT,
                            stringify!($name),
                            e
                        );
                    }
                }))?;

                Ok(Self { state: ::std::sync::Arc::new(::arc_swap::ArcSwap::new(::std::sync::Arc::new(::std::option::Option::Some(state)))) })
            }

            fn subscribe(
                &self,
                observer: ::observable_property::Observer<::std::sync::Arc<Self::State>>,
            ) -> ::anyhow::Result<observable_property::ObserverId> {
                if let Some(self_state) = self.state.load().as_ref() {
                    Ok(self_state.subscribe(observer)?)
                } else {
                    anyhow::bail!("no state")
                }
            }
        }

        impl $name {
            #[allow(dead_code)]
            pub const ID: &'static str =
                <$name as $crate::observables::Observable>::ID;

            pub const EVENT: &'static str =
                <$name as $crate::observables::Observable>::EVENT;

            pub fn get_json(
                handle: &::tauri::AppHandle,
            ) -> ::anyhow::Result<::serde_json::Value> {
                let state = handle.observable_state::<$name>()?;
                Ok(::serde_json::to_value(state.as_ref())?)
            }
        }

        ::inventory::submit! {
            $crate::observables::ObservableRegistration {
                id: <$name as $crate::observables::Observable>::ID,
                get_json: <$name>::get_json,
            }
        }
    };
}
