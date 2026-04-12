use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::{Context, Result};
use std::sync::LazyLock;

type JsonProducer = Box<dyn Fn() -> Result<serde_json::Value> + Send + Sync>;

pub static OBSERVABLES: LazyLock<Mutex<Vec<(&'static str, JsonProducer)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub trait Observable: Sized + Send + Sync + 'static {
    type State: Clone + Default + serde::Serialize + Send + Sync + 'static;

    const ID: &'static str;
    const EVENT: &'static str;

    fn new(app: &mut tauri::App) -> Result<Self>;

    fn get_state(&self) -> Result<Self::State>;

    fn set_state<T>(&self, state: T) -> Result<()>
    where
        T: FnOnce(Self::State) -> Self::State;
    fn try_set_state<T>(&self, state: T) -> Result<()>
    where
        T: FnOnce(Self::State) -> Result<Self::State>;
    fn subscribe(
        &self,
        observer: observable_property::Observer<Self::State>,
    ) -> Result<observable_property::ObserverId>;
}

pub fn get_all_observable_states() -> Result<HashMap<String, serde_json::Value>> {
    let observables = OBSERVABLES.lock().expect("poisoned");

    observables.iter()
        .map(|(id, produce_json)| {
            produce_json().map(|json| (id.to_string(), json))
        })
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
        use nject::{injectable, inject};

        /// We intentionally don't implement Clone so that observables always have one owner
        /// to make code easier to reason about.
        #[injectable]
        $(#[$meta])*
        $vis struct $name {
            #[inject(::observable_property::ObservableProperty::new(
                $state::default()
            ))]
            state: ::observable_property::ObservableProperty<
                $state
            >,

            /// Used to implement a thread-safe callback-style `set_state`. Doesn't own the data so
            /// $state stays Clone
            #[inject(::std::sync::RwLock::new(()))]
            mutex: ::std::sync::RwLock<()>
        }

        impl $name {
        }

        impl $crate::state::Observable for $name {
            type State = $state;

            const ID: &'static str = $id;
            const EVENT: &'static str = ::std::concat!("state:", $id);

            /// We intentionally take &mut tauri::App so that Observables can only be initialized
            /// in app setup.
            fn new(app: &mut ::tauri::App) -> ::anyhow::Result<Self> {
                use tauri::Manager;

                let handle = app.handle().clone();
                let state = <Self::State as ::std::default::Default>::default();
                let observable_property =
                    ::observable_property::ObservableProperty::new(state);

                handle.manage::<::observable_property::ObservableProperty<$state>>(observable_property.clone());
                {
                    use ::anyhow::Context;
                    let observable_property = observable_property.clone();
                    let mut observables = $crate::state::OBSERVABLES.lock().expect("poisoned");
                    observables.push(($id, Box::new(move || {
                        let state = observable_property.get()?;
                        Ok(serde_json::to_value(state)?)
                    })));
                }

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
                    state: observable_property,
                    mutex: ::std::sync::RwLock::new(())
                })
            }

            fn get_state(&self) -> ::anyhow::Result<Self::State> {
                Ok(self.state.get()?)
            }

            fn set_state<T>(&self, callback: T) -> ::anyhow::Result<()>
            where
                T: ::core::ops::FnOnce(Self::State) -> Self::State
            {
                let mutex = self.mutex.read().unwrap();
                let prev_state = self.state.get()?;
                let next_state = callback(prev_state);
                self.state.set(next_state)?;
                drop(mutex);

                Ok(())
            }

            fn try_set_state<T>(&self, callback: T) -> ::anyhow::Result<()>
            where
                T: ::core::ops::FnOnce(Self::State) -> ::anyhow::Result<Self::State>
            {
                let mutex = self.mutex.read().unwrap();
                let prev_state = self.state.get()?;
                let next_state = callback(prev_state)?;
                self.state.set(next_state)?;
                drop(mutex);

                Ok(())
            }

            fn subscribe(
                &self,
                observer: ::observable_property::Observer<Self::State>,
            ) -> ::anyhow::Result<observable_property::ObserverId> {
                Ok(self.state.subscribe(observer)?)
            }
        }

        impl $name {
            #[allow(dead_code)]
            pub const ID: &'static str =
                <$name as $crate::state::Observable>::ID;

            pub const EVENT: &'static str =
                <$name as $crate::state::Observable>::EVENT;
        }
    };
}
