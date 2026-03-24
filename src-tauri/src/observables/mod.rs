use std::collections::HashMap;

mod chord_files;
mod chorder;
mod frontmost;
mod git_repos;
mod permissions;
mod settings;

use crate::feature::SafeAppHandle;
pub use chord_files::*;
pub use chorder::*;
pub use frontmost::*;
pub use git_repos::*;
pub use permissions::*;
pub use settings::*;

pub struct ObservableRegistration {
    pub id: &'static str,
    pub get_json: fn(&SafeAppHandle) -> anyhow::Result<serde_json::Value>,
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
    ) -> Result<observable_property::ObserverId, observable_property::PropertyError>;

    fn new(handle: crate::feature::SafeAppHandle) -> anyhow::Result<Self>;
}

pub fn get_all_observable_states(
    handle: SafeAppHandle,
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
        $(#[$meta])*
        $vis struct $name {
            state: ::observable_property::ObservableProperty<::std::sync::Arc<$state>>,
        }

        impl $crate::observables::Observable for $name {
            type State = $state;

            const ID: &'static str = $id;
            const EVENT: &'static str = ::std::concat!("state:", $id);

            fn get_state(&self) -> ::anyhow::Result<::std::sync::Arc<Self::State>> {
                Ok(self.state.get()?)
            }

            fn set_state(&self, state: Self::State) -> ::anyhow::Result<()> {
                Ok(self.state.set(::std::sync::Arc::new(state))?)
            }

            fn new(handle: $crate::feature::SafeAppHandle) -> ::anyhow::Result<Self> {
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

                Ok(Self { state })
            }

            fn subscribe(
                &self,
                observer: ::observable_property::Observer<::std::sync::Arc<Self::State>>,
            ) -> ::std::result::Result<
                ::observable_property::ObserverId,
                ::observable_property::PropertyError,
            > {
                self.state.subscribe(observer)
            }
        }

        impl $name {
            pub const ID: &'static str =
                <$name as $crate::observables::Observable>::ID;

            pub const EVENT: &'static str =
                <$name as $crate::observables::Observable>::EVENT;

            pub fn get_json(
                handle: &$crate::feature::SafeAppHandle,
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
