use std::any::Any;
use std::collections::HashMap;

mod chorder;
mod git_repos;
mod permissions;
mod settings;

pub use chorder::*;
pub use git_repos::*;
pub use permissions::*;
pub use settings::*;

pub struct ObservableRegistration {
    pub id: &'static str,
    pub default_json: fn() -> anyhow::Result<serde_json::Value>,
}

inventory::collect!(ObservableRegistration);

pub fn get_all_observable_defaults(
) -> anyhow::Result<HashMap<&'static str, serde_json::Value>> {
    inventory::iter::<ObservableRegistration>
        .into_iter()
        .map(|reg| Ok((reg.id, (reg.default_json)()?)))
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

        impl $name {
            pub const ID: &'static str = $id;
            pub const EVENT: &'static str = ::std::concat!("state:", $id);

            $vis fn get_state(&self) -> ::anyhow::Result<::std::sync::Arc<$state>> {
                Ok(self.state.get()?)
            }

            $vis fn set_state(&self, state: $state) -> ::anyhow::Result<()> {
                Ok(self.state.set(::std::sync::Arc::new(state))?)
            }

            $vis fn default_json() -> ::anyhow::Result<::serde_json::Value>
            where
                $state: ::std::default::Default + ::serde::Serialize,
            {
                Ok(::serde_json::to_value(<$state as ::std::default::Default>::default())?)
            }

            $vis fn new(handle: $crate::feature::SafeAppHandle) -> ::anyhow::Result<Self>
            where
                $state: ::serde::Serialize + Send + Sync + 'static,
            {
                let state = <$state as ::std::default::Default>::default();
                let state =
                    ::observable_property::ObservableProperty::new(::std::sync::Arc::new(state));

                state.subscribe(::std::sync::Arc::new(move |_, new_state| {
                    if let Err(e) = handle.emit(Self::EVENT, new_state) {
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

            $vis fn subscribe(
                &self,
                observer: ::observable_property::Observer<::std::sync::Arc<$state>>,
            ) -> ::std::result::Result<
                ::observable_property::ObserverId,
                ::observable_property::PropertyError,
            > {
                self.state.subscribe(observer)
            }
        }

        ::inventory::submit! {
            $crate::observables::ObservableRegistration {
                id: $name::ID,
                default_json: $name::default_json,
            }
        }
    };
}