use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use observable_property::ObservableProperty;
use tauri::AppHandle;

pub struct ObservableRegistration {
    pub id: &'static str,
    pub get_json: fn(&AppHandle) -> anyhow::Result<serde_json::Value>,
}

inventory::collect!(ObservableRegistration);

pub trait Observable: Sized + Send + Sync + 'static {
    type State: Clone + Default + serde::Serialize + Send + Sync + 'static;

    const ID: &'static str;
    const EVENT: &'static str;

    fn get_state(&self) -> anyhow::Result<Self::State>;
    fn set_state<T>(&self, state: T) -> anyhow::Result<()>
    where
        T: FnOnce(Self::State) -> Self::State;
    fn try_set_state<T>(&self, state: T) -> anyhow::Result<()>
    where
        T: FnOnce(Self::State) -> anyhow::Result<Self::State>;
    fn subscribe(
        &self,
        observer: observable_property::Observer<Self::State>,
    ) -> anyhow::Result<observable_property::ObserverId>;

    fn uninitialized() -> Self;
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
            state:
                // Allows an observable to have many owners, though I'm not entirely sure whether
                // this is best practice (ideally, we should split observables up if they otherwise
                // need multiple owners)
                ::std::sync::Arc<
                    // The ArcSwap is needed to safely switch from an uninitialized observable to an
                    // initialized one
                    ::arc_swap::ArcSwap<
                        // None for uninitialized observables
                        ::std::option::Option<
                            // The RwLock is needed for a thread-safe callback-style `set_state`
                            ::std::sync::RwLock<
                                ::observable_property::ObservableProperty<
                                    $state
                                >
                            >
                        >
                    >
                >
        }

        impl $name {
        }

        impl $crate::state::Observable for $name {
            type State = $state;

            const ID: &'static str = $id;
            const EVENT: &'static str = ::std::concat!("state:", $id);

            fn uninitialized() -> Self {
                Self {
                    state: ::std::sync::Arc::new(
                        ::arc_swap::ArcSwap::new(
                            ::std::sync::Arc::new(
                                ::std::option::Option::None
                            )
                        )
                    )
                }
            }

            fn init(&self, observable: Self) {
                self.state.store(observable.state.load_full())
            }

            fn get_state(&self) -> ::anyhow::Result<Self::State> {
                if let Some(lock) = self.state.load().as_ref() {
                    let observable = lock.read().unwrap();
                    Ok(observable.get()?)
                } else {
                    anyhow::bail!("uninitialized state")
                }
            }

            fn set_state<T>(&self, callback: T) -> ::anyhow::Result<()>
            where
                T: ::core::ops::FnOnce(Self::State) -> Self::State
            {
                if let Some(lock) = self.state.load().as_ref() {
                    let observable = lock.read().unwrap();
                    let prev_state = observable.get()?;
                    let next_state = callback(prev_state);
                    Ok(observable.set(next_state)?)
                } else {
                    anyhow::bail!("uninitialized state")
                }
            }

            fn try_set_state<T>(&self, callback: T) -> ::anyhow::Result<()>
            where
                T: ::core::ops::FnOnce(Self::State) -> ::anyhow::Result<Self::State>
            {
                if let Some(lock) = self.state.load().as_ref() {
                    let observable = lock.read().unwrap();
                    let prev_state = observable.get()?;
                    let next_state = callback(prev_state)?;
                    Ok(observable.set(next_state)?)
                } else {
                    anyhow::bail!("uninitialized state")
                }
            }

            fn new(handle: ::tauri::AppHandle) -> ::anyhow::Result<Self> {
                let state = <Self::State as ::std::default::Default>::default();
                let observable_property =
                    ::observable_property::ObservableProperty::new(state);

                observable_property.subscribe(::std::sync::Arc::new(move |_, new_state| {
                    if let Err(e) = handle.emit(Self::EVENT, new_state) {
                        ::log::error!(
                            "Failed to emit {} for {}: {}",
                            Self::EVENT,
                            stringify!($name),
                            e
                        );
                    }
                }))?;

                Ok(Self {
                    state: ::std::sync::Arc::new(
                        ::arc_swap::ArcSwap::new(
                            ::std::sync::Arc::new(
                                ::std::option::Option::Some(
                                    ::std::sync::RwLock::new(
                                        observable_property
                                    )
                                )
                            )
                        )
                    )
                })
            }

            fn subscribe(
                &self,
                observer: ::observable_property::Observer<Self::State>,
            ) -> ::anyhow::Result<observable_property::ObserverId> {
                if let Some(lock) = self.state.load().as_ref() {
                    let observable = lock.read().unwrap();
                    Ok(observable.subscribe(observer)?)
                } else {
                    anyhow::bail!("no state")
                }
            }
        }

        impl $name {
            #[allow(dead_code)]
            pub const ID: &'static str =
                <$name as $crate::state::Observable>::ID;

            pub const EVENT: &'static str =
                <$name as $crate::state::Observable>::EVENT;

            pub fn get_json(
                handle: &::tauri::AppHandle,
            ) -> ::anyhow::Result<::serde_json::Value> {
                let state = handle.observable_state::<$name>()?;
                Ok(::serde_json::to_value(state)?)
            }
        }

        ::inventory::submit! {
            $crate::state::ObservableRegistration {
                id: <$name as $crate::state::Observable>::ID,
                get_json: <$name>::get_json,
            }
        }
    };
}
