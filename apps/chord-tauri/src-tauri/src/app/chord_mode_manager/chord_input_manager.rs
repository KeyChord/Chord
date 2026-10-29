use super::chord_loops::{ChordLoops, run_plan, select_run_window};
use super::chord_sequence::{ChordSequence, ChordSequenceDispatch, ChordTaskQueue};
use crate::app::AppHandleExt;
use crate::app::chord_runner::release_chord_sequences;
use crate::app::state::AppSingleton;
use crate::models::{
    ChordEventTrigger, ChordInputEvent, ChordKeyboardEvent, ChordMacroName, ChordMacros, Key,
    KeyEvent, MetaChordCommand, expand_typed_reference, select_escape,
};
use crate::state::{
    ChordExecutionObservable, ChordInputObservable, ChordInputState, ChordMacrosObservable,
    ChordMacrosState, MetaChordHint, Observable,
};
use anyhow::{Context, Result, bail};
use keycode::KeyMappingCode;
use nject::injectable;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;
use tokio::sync::oneshot;

#[injectable]
pub struct ChordInputManager {
    #[inject(Mutex::new(None))]
    active_task_run: Mutex<Option<(ChordInputEvent, oneshot::Sender<()>)>>,
    /// Repeated keydowns notify handlers without inserting duplicate buffer characters.
    #[inject(Mutex::new(HashSet::new()))]
    held_keys: Mutex<HashSet<Key>>,
    /// Keys already held when insert mode began, such as Space; their repeats type nothing.
    #[inject(Mutex::new(HashSet::new()))]
    carried_keys: Mutex<HashSet<Key>>,
    #[inject(Mutex::new(ChordSequence::default()))]
    sequence: Mutex<ChordSequence>,
    #[inject(ChordTaskQueue::default())]
    task_queue: ChordTaskQueue,
    #[inject(ChordLoops::default())]
    loops: ChordLoops,
    /// Counts executions, so a run that fails later only marks its own chord.
    #[inject(Arc::new(AtomicU64::new(0)))]
    executions: Arc<AtomicU64>,
    handle: AppHandle,
    observable: ChordInputObservable,
    execution_observable: ChordExecutionObservable,
    macros_observable: ChordMacrosObservable,
}

impl ChordInputManager {
    pub(super) fn load_macros(&self) -> Result<()> {
        let store = self
            .handle
            .store_builder("named-macros.json")
            .disable_auto_save()
            .build()?;
        match store.reload() {
            Ok(()) => {}
            Err(tauri_plugin_store::Error::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).context("failed to load named macros"),
        }
        let definitions = store
            .get("definitions")
            .map(serde_json::from_value)
            .transpose()
            .context("failed to read named macros")?
            .unwrap_or_default();
        let mut sequence = self.sequence.lock();
        sequence.macros = ChordMacros::from_persistent(definitions)?;
        self.macros_observable.set_state(|_| ChordMacrosState {
            entries: sequence.macros.entries(),
        })
    }

    pub fn update_macro(
        &self,
        target: ChordMacroName,
        value: Option<String>,
        previous: Option<ChordMacroName>,
    ) -> Result<()> {
        let mut sequence = self.sequence.lock();
        sequence
            .macros
            .update(target, value, previous, |definitions| {
                let store = self
                    .handle
                    .store_builder("named-macros.json")
                    .disable_auto_save()
                    .build()?;
                let old = store.get("definitions");
                store.set("definitions", serde_json::to_value(definitions)?);
                if let Err(error) = store.save() {
                    if let Some(old) = old {
                        store.set("definitions", old);
                    } else {
                        store.delete("definitions");
                    }
                    return Err(error).context("failed to save named macros");
                }
                Ok(())
            })?;
        self.macros_observable.set_state(|_| ChordMacrosState {
            entries: sequence.macros.entries(),
        })
    }

    pub fn reset(&self) -> Result<()> {
        self.held_keys.lock().clear();
        self.end_active_task();
        self.reset_matching_chords()?;
        self.observable.set_state(|_| ChordInputState::default())
    }

    pub fn stop_loops(&self) -> Result<()> {
        self.loops.stop_all(&self.execution_observable)
    }

    pub fn configure_meta_commands(
        &self,
        hints: Vec<MetaChordHint>,
        halt_loop_template: String,
    ) -> Result<()> {
        self.loops
            .configure_meta_commands(&self.execution_observable, hints, halt_loop_template)
    }

    /// Execute a complete chord sequence without synthesizing keyboard input.
    pub fn execute_sequence(&self, sequence: &str) -> Result<()> {
        let resolved = self
            .sequence
            .lock()
            .macros
            .resolve_named(sequence)?
            .with_context(|| format!("`{sequence}` references an undefined macro"))?;
        let input = Key::parse_sequence(&resolved)?;
        if input.len() < 2 {
            bail!("a chord sequence must contain at least two keys");
        }
        self.end_active_task();
        self.reset_matching_chords()?;
        let dispatch = self.resolve_dispatch(&input, &[], &[ChordEventTrigger::Input], None)?;
        let Some(event) = dispatch.input_event.clone() else {
            self.reset_matching_chords()?;
            bail!("no chord is configured for `{sequence}`");
        };
        self.set_executed_event(event)?;
        self.run_dispatch(dispatch, false)?;
        self.reset_matching_chords()
    }

    // Called only while Chord mode is active.
    pub fn handle_key_event(&self, key_event: &KeyEvent) -> Result<()> {
        let fresh_press = should_handle_held_key_event(&mut self.held_keys.lock(), key_event);
        let (KeyEvent::Press(key) | KeyEvent::Release(key)) = key_event;
        let keyboard = self.handle.app_state().keyboard();
        let Some(mut device_keys) = keyboard.state().keys() else {
            return Ok(());
        };
        // Account for the event even if the OS key-state query has not caught up yet.
        match key_event {
            KeyEvent::Press(_) => {
                if !device_keys.contains(key) {
                    device_keys.push(*key);
                }
            }
            KeyEvent::Release(_) => device_keys.retain(|held| held != key),
        }
        let event = ChordKeyboardEvent::new(key_event, &device_keys, !fresh_press);
        let press_triggers = [ChordEventTrigger::Keydown, ChordEventTrigger::Keypress];
        // Releasing Shift makes the block typed while it was held solid.
        if !event.shift_key {
            self.close_block()?;
        }

        if let KeyEvent::Release(Key(code)) = key_event {
            let should_end = self
                .active_task_run
                .lock()
                .as_ref()
                .is_some_and(|(input, _)| {
                    *code == KeyMappingCode::CapsLock
                        || input.input.last().is_some_and(|key| key.0 == *code)
                });
            if should_end {
                self.end_active_task();
            }
        }

        if !fresh_press {
            return self.dispatch_current_buffer(&press_triggers, &event);
        }
        let is_shift_pressed = event.shift_key;
        if Self::should_toggle_indicator_for_tab_event(key_event, is_shift_pressed) {
            if matches!(key_event, KeyEvent::Press(_)) {
                self.handle
                    .app_state()
                    .chord_mode_manager()
                    .panel
                    .toggle()?;
            }
            return self.dispatch_current_buffer(
                if matches!(key_event, KeyEvent::Press(_)) {
                    &press_triggers
                } else {
                    &[ChordEventTrigger::Keyup]
                },
                &event,
            );
        }
        if Key::modifiers().contains(key)
            || device_keys
                .iter()
                .any(|key| Key::non_shift_modifiers().contains(key))
        {
            return self.dispatch_current_buffer(
                if matches!(key_event, KeyEvent::Press(_)) {
                    &press_triggers
                } else {
                    &[ChordEventTrigger::Keyup]
                },
                &event,
            );
        }

        match key_event {
            KeyEvent::Release(Key(code)) => {
                self.dispatch_current_buffer(&[ChordEventTrigger::Keyup], &event)?;
                let state = self.observable.get_state()?;
                if *code == KeyMappingCode::Space
                    && Self::should_execute_key_buffer_on_release(&state)
                {
                    self.execute_input(
                        &state.input,
                        &state.quotes,
                        &[ChordEventTrigger::Input],
                        &event,
                        false,
                    )?;
                }
            }
            KeyEvent::Press(Key(KeyMappingCode::CapsLock)) => {
                let state = self.observable.get_state()?;
                // Blocks alone never run, so Caps repeats the last chord as on an empty buffer.
                if !state.has_loose_keys() {
                    self.dispatch_current_buffer(&press_triggers, &event)?;
                    if let Some(selected) = state.selected_input_event {
                        self.execute_input(
                            &selected.input,
                            &selected.quotes,
                            &[ChordEventTrigger::Input],
                            &event,
                            true,
                        )?;
                    }
                } else {
                    self.execute_input(
                        &state.input,
                        &state.quotes,
                        &[
                            ChordEventTrigger::Keydown,
                            ChordEventTrigger::Keypress,
                            ChordEventTrigger::Input,
                        ],
                        &event,
                        true,
                    )?;
                }
            }
            KeyEvent::Press(Key(KeyMappingCode::Escape)) => {
                self.clear_key_buffer()?;
                self.dispatch_current_buffer(&press_triggers, &event)?;
            }
            KeyEvent::Press(Key(KeyMappingCode::Space)) => {
                self.dispatch_current_buffer(&press_triggers, &event)?;
            }
            KeyEvent::Press(Key(KeyMappingCode::Tab)) => {
                // Typed escapes other than complete macro references expand only on Tab.
                let state = self.observable.get_state()?;
                let selected = select_escape(&state.input, &self.sequence.lock().macros)?;
                if let Some(input) = selected {
                    let blocks = rebase_blocks(state.blocks, &state.input, &input);
                    self.edit_buffer(input, blocks, state.open_block, state.quotes, false)?;
                }
                self.dispatch_current_buffer(&press_triggers, &event)?;
            }
            // A quote always opens like an unshifted key, entering insert mode at the
            // start of a step. It ends a block typed before it, since insert mode
            // never sees Shift's release.
            KeyEvent::Press(key @ Key(KeyMappingCode::Quote)) => {
                self.handle_key_press(key, &event, false)?;
            }
            KeyEvent::Press(key) => {
                if is_shift_pressed && Self::should_clear_key_buffer_on_shifted_press(key) {
                    self.clear_key_buffer()?;
                    self.dispatch_current_buffer(&press_triggers, &event)?;
                } else {
                    self.handle_key_press(key, &event, is_shift_pressed)?;
                }
            }
        }
        Ok(())
    }

    fn resolve_dispatch(
        &self,
        keys: &[Key],
        quotes: &[String],
        triggers: &[ChordEventTrigger],
        keyboard_event: Option<&ChordKeyboardEvent>,
    ) -> Result<ChordSequenceDispatch> {
        let raw_event = ChordInputEvent {
            input: keys.to_vec(),
            application_id: self.handle.app_state().frontmost().frontmost()?,
            quotes: quotes.to_vec(),
        };
        self.sequence
            .lock()
            .dispatch_command(raw_event, triggers, keyboard_event, |event| {
                self.handle
                    .app_state()
                    .chord_package_manager()
                    .matching_chord_contexts(event)
            })
    }

    fn dispatch_current_buffer(
        &self,
        triggers: &[ChordEventTrigger],
        event: &ChordKeyboardEvent,
    ) -> Result<()> {
        let state = self.observable.get_state()?;
        self.run_dispatch(
            self.resolve_dispatch(&state.input, &state.quotes, triggers, Some(event))?,
            false,
        )
    }

    fn end_active_task(&self) {
        if let Some((_, release)) = self.active_task_run.lock().take() {
            let _ = release.send(());
        }
    }

    fn run_dispatch(&self, mut dispatch: ChordSequenceDispatch, hold_input: bool) -> Result<()> {
        if let Some(description) = dispatch.description.take() {
            self.observable.set_state(|mut state| {
                // Keyup callbacks for an emptied buffer must not replace the
                // description of the chord that just completed.
                if state.selected_input_event.is_none() || dispatch.input_event.is_some() {
                    state.description = Some(description);
                }
                state
            })?;
        }
        if let Some(command) = dispatch.meta_command.take() {
            match command {
                MetaChordCommand::Settings => self.handle.app_state().settings().ui.open()?,
                MetaChordCommand::Reload => {
                    self.stop_loops()?;
                    crate::tauri_app::scripting::reload_configs(self.handle.clone());
                }
                MetaChordCommand::Quit => {
                    self.stop_loops()?;
                    self.handle.exit(0);
                }
                MetaChordCommand::Halt(id) => self.loops.halt(id, &self.execution_observable)?,
                MetaChordCommand::SetLayout(layout) => self
                    .handle
                    .app_state()
                    .chord_mode_manager()
                    .panel
                    .set_layout(layout)?,
                MetaChordCommand::CycleLayout => self
                    .handle
                    .app_state()
                    .chord_mode_manager()
                    .panel
                    .cycle_layout()?,
                MetaChordCommand::SetMacro { name, sequence } => {
                    self.update_macro(name, Some(sequence), None)?;
                }
            }
        }
        let loop_run = dispatch
            .plan
            .take()
            .map(|plan| {
                self.loops
                    .reserve(plan.input.clone(), &self.execution_observable)
                    .map(|(id, cancelled)| (id, plan, cancelled))
            })
            .transpose()?;
        if dispatch.tasks.is_empty() && dispatch.expired.is_empty() && loop_run.is_none() {
            return Ok(());
        }
        let release = if let Some(input_event) = dispatch
            .input_event
            .clone()
            .filter(|_| hold_input && loop_run.is_none())
        {
            self.end_active_task();
            let (sender, receiver) = oneshot::channel();
            *self.active_task_run.lock() = Some((input_event, sender));
            Some(receiver)
        } else {
            None
        };
        let report_failure = {
            let observable = self.observable.provide();
            let executions = self.executions.clone();
            let execution = executions.load(Ordering::SeqCst);
            let executed = dispatch.input_event.is_some();
            move |error: &anyhow::Error| {
                // Only the chord this run executed, and only while it is still shown.
                if !executed || executions.load(Ordering::SeqCst) != execution {
                    return;
                }
                let message = format!("{error:#}");
                if let Err(error) = observable.set_state(|mut state| {
                    if state.selected_input_event.is_some() {
                        state.error = Some(message);
                    }
                    state
                }) {
                    log::error!("failed to show chord error: {error:#}");
                }
            }
        };
        let handle = self.handle.clone();
        // Expired scopes have unique IDs. Releasing them can wait behind a busy
        // package loop, but must not stall commands in unrelated packages.
        let expired = std::mem::take(&mut dispatch.expired);
        let loops = self.loops.clone();
        let execution_observable = self.execution_observable.provide();
        let window_app = dispatch
            .input_event
            .as_ref()
            .map(|event| event.application_id.clone());
        let window_handle = handle.clone();
        self.task_queue.enqueue(async move {
            let mut loop_run = loop_run;
            // Select the target window outside the handlers, once per run, off the
            // keyboard thread. Callbacks while typing have no completed input to target.
            if let Some(app_id) = window_app {
                select_run_window(
                    &mut dispatch.tasks,
                    loop_run.as_mut().map(|(_, plan, _)| plan),
                    || async move {
                        tauri::async_runtime::spawn_blocking(move || {
                            crate::app::frontmost::focused_window_id(
                                &window_handle,
                                app_id.as_deref(),
                            )
                        })
                        .await
                        .ok()
                        .flatten()
                    },
                )
                .await;
            }
            // Previous callbacks must finish creating their scopes before cleanup
            // is scheduled. The cleanup itself need not block new callbacks.
            if !expired.is_empty() {
                let release_handle = handle.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = release_chord_sequences(release_handle, expired).await {
                        log::error!("failed to release chord values: {error:#}");
                    }
                });
            }
            let result = async {
                let runner = handle.app_state().chord_action_task_runner();
                let mut release = release;
                let count = dispatch.tasks.len();
                for (index, task) in dispatch.tasks.into_iter().enumerate() {
                    if loop_run
                        .as_ref()
                        .is_some_and(|(_, _, cancelled)| *cancelled.borrow())
                    {
                        break;
                    }
                    let mut task_run = runner.start_task(&task)?;
                    if index + 1 == count {
                        if let Some(release) = release.take() {
                            let _ = release.await;
                        }
                    }
                    runner.end_task(&mut task_run).await?;
                }
                Ok::<(), anyhow::Error>(())
            }
            .await;
            if let Err(error) = result {
                log::error!("error running chord events: {error:#}");
                report_failure(&error);
                if let Some((id, plan, _)) = loop_run {
                    loops.finish(id, &execution_observable);
                    if !plan.scopes.is_empty() {
                        tauri::async_runtime::spawn(release_chord_sequences(handle, plan.scopes));
                    }
                }
            } else if let Some((id, plan, cancelled)) = loop_run {
                // Only the initializers stay in the ordered event queue. The plan must
                // never keep a later chord or its halt command waiting on that queue.
                tauri::async_runtime::spawn(async move {
                    let result = run_plan(handle, plan, cancelled).await;
                    loops.finish(id, &execution_observable);
                    if let Err(error) = result {
                        log::error!("chord loop {id} stopped: {error:#}");
                        report_failure(&error);
                    }
                });
            }
        });
        Ok(())
    }

    fn reset_matching_chords(&self) -> Result<()> {
        let expired = self.sequence.lock().reset();
        self.run_dispatch(
            ChordSequenceDispatch {
                expired,
                ..Default::default()
            },
            false,
        )
    }

    fn set_executed_event(&self, event: ChordInputEvent) -> Result<()> {
        self.executions.fetch_add(1, Ordering::SeqCst);
        self.observable.set_state(move |state| {
            let (input, quotes) = state.block_input();
            ChordInputState {
                input,
                quotes,
                inserting: false,
                pressed_input_event: Some(event.clone()),
                selected_input_event: Some(event),
                error: None,
                ..state
            }
        })
    }

    /// Input that cannot run stays on screen like an executed chord, with the
    /// reason in place of the chord's name. Its blocks stay in the buffer.
    fn fail_input(&self, input: &[Key], quotes: &[String], error: String) -> Result<()> {
        self.end_active_task();
        self.executions.fetch_add(1, Ordering::SeqCst);
        self.observable.set_state(|state| {
            let (kept_input, kept_quotes) = state.block_input();
            ChordInputState {
                input: kept_input,
                quotes: kept_quotes,
                inserting: false,
                blocks: state.blocks,
                open_block: state.open_block,
                pressed_input_event: None,
                // Nothing ran, so no application received the chord.
                selected_input_event: Some(ChordInputEvent {
                    input: input.to_vec(),
                    application_id: None,
                    quotes: quotes.to_vec(),
                }),
                description: None,
                error: Some(error),
            }
        })?;
        self.reset_matching_chords()
    }

    fn execute_input(
        &self,
        keys: &[Key],
        quotes: &[String],
        triggers: &[ChordEventTrigger],
        event: &ChordKeyboardEvent,
        hold_input: bool,
    ) -> Result<()> {
        match self.resolve_dispatch(keys, quotes, triggers, Some(event)) {
            Ok(dispatch) => self.complete_dispatch(keys, quotes, dispatch, hold_input),
            Err(error) => self.fail_input(keys, quotes, format!("{error:#}")),
        }
    }

    fn complete_dispatch(
        &self,
        keys: &[Key],
        quotes: &[String],
        mut dispatch: ChordSequenceDispatch,
        hold_input: bool,
    ) -> Result<()> {
        if let Some(event) = dispatch.input_event.clone() {
            self.set_executed_event(event)?;
            self.run_dispatch(dispatch, hold_input)?;
            return self.reset_matching_chords();
        }
        let error = dispatch.error.take();
        self.run_dispatch(dispatch, false)?;
        match error {
            Some(error) => self.fail_input(keys, quotes, error),
            // Chords that only handle keyboard events have no input action to run.
            None => self.clear_loose_keys(),
        }
    }

    fn should_execute_key_buffer_on_release(state: &ChordInputState) -> bool {
        state.has_loose_keys() && state.selected_input_event.is_none()
    }

    fn should_toggle_indicator_for_tab_event(key_event: &KeyEvent, is_shift_pressed: bool) -> bool {
        matches!(
            key_event,
            KeyEvent::Press(Key(KeyMappingCode::Tab)) | KeyEvent::Release(Key(KeyMappingCode::Tab))
        ) && is_shift_pressed
    }

    /// Backspace removes the last key after the blocks, else the whole last block.
    fn next_key_buffer_for_key_press(key_buffer: &[Key], blocks: &[u32], key: &Key) -> Vec<Key> {
        let mut next_key_buffer = key_buffer.to_vec();
        if key.0 == KeyMappingCode::Backspace {
            let block_ends = blocks.iter().map(|&end| end as usize);
            if block_ends.clone().last() == Some(next_key_buffer.len()) {
                let previous_end = block_ends.rev().nth(1).unwrap_or(0);
                next_key_buffer.truncate(previous_end);
            } else {
                next_key_buffer.pop();
            }
        } else if Key::serialize_sequence(&[*key]).is_some() {
            next_key_buffer.push(*key);
        }
        next_key_buffer
    }

    /// A macro reference expands as soon as the key that completes it is typed.
    fn expand_typed_reference(&self, input: Vec<Key>) -> Vec<Key> {
        expand_typed_reference(&input, &self.sequence.lock().macros).unwrap_or(input)
    }

    /// A key typed with Shift held joins the open block, which takes in every key
    /// after the solid blocks; see `ChordInputState::blocks`.
    fn handle_key_press(&self, key: &Key, event: &ChordKeyboardEvent, shifted: bool) -> Result<()> {
        let state = self.observable.get_state()?;
        let mut blocks = state.blocks;
        if state.open_block && !shifted {
            blocks = close_block(&state.input, blocks);
        }
        let mut next_key_buffer = Self::next_key_buffer_for_key_press(&state.input, &blocks, key);
        blocks = rebase_blocks(blocks, &state.input, &next_key_buffer);
        let grew = next_key_buffer.len() > state.input.len();
        if grew {
            let expanded = self.expand_typed_reference(next_key_buffer.clone());
            blocks = rebase_blocks(blocks, &next_key_buffer, &expanded);
            next_key_buffer = expanded;
        }
        self.edit_buffer(
            next_key_buffer,
            blocks,
            shifted && (state.open_block || grew),
            state.quotes,
            key.0 == KeyMappingCode::Quote,
        )?;
        self.dispatch_current_buffer(
            &[ChordEventTrigger::Keydown, ChordEventTrigger::Keypress],
            event,
        )
    }

    fn close_block(&self) -> Result<()> {
        if !self.observable.get_state()?.open_block {
            return Ok(());
        }
        self.observable.set_state(|state| ChordInputState {
            blocks: close_block(&state.input, state.blocks),
            open_block: false,
            ..state
        })
    }

    fn should_clear_key_buffer_on_shifted_press(key: &Key) -> bool {
        key.0 == KeyMappingCode::Backspace
    }

    /// Esc and Shift+Backspace clear the whole input, blocks included.
    fn clear_key_buffer(&self) -> Result<()> {
        self.observable.set_state(|state| ChordInputState {
            blocks: vec![],
            open_block: false,
            ..state
        })?;
        self.clear_loose_keys()
    }

    /// Clears the input after its blocks.
    fn clear_loose_keys(&self) -> Result<()> {
        self.end_active_task();
        self.observable.set_state(|state| {
            let (input, quotes) = state.block_input();
            ChordInputState {
                input,
                quotes,
                inserting: false,
                selected_input_event: None,
                pressed_input_event: None,
                description: None,
                error: None,
                ..state
            }
        })?;
        self.reset_matching_chords()
    }

    /// Replaces the buffer being typed; see `ChordSequence::edit_quotes`.
    fn edit_buffer(
        &self,
        input: Vec<Key>,
        blocks: Vec<u32>,
        open_block: bool,
        quotes: Vec<String>,
        opened: bool,
    ) -> Result<()> {
        let (quotes, inserting) = self.sequence.lock().edit_quotes(&input, quotes, opened)?;
        if inserting {
            *self.carried_keys.lock() = self.held_keys.lock().clone();
        }
        self.observable.set_state(|state| ChordInputState {
            input,
            quotes,
            inserting,
            blocks,
            open_block,
            description: None,
            selected_input_event: None,
            pressed_input_event: None,
            error: None,
            ..state
        })
    }

    pub fn is_inserting(&self) -> bool {
        self.observable
            .get_state()
            .is_ok_and(|state| state.inserting)
    }

    /// In insert mode Chord stays active without Space and every key is typed into
    /// the open quote. The controller handles Caps+Space and Esc before this.
    pub fn handle_insert_key_event(&self, key_event: &KeyEvent) -> Result<()> {
        let mut text = self
            .observable
            .get_state()?
            .quotes
            .pop()
            .unwrap_or_default();
        // The OS keeps toggling Caps Lock on a plain Caps press, so read it as typed.
        let caps_lock = matches!(key_event, KeyEvent::Press(key) if key.is_letter())
            && crate::app::keyboard::AppKeyboard::get_caps_lock().unwrap_or_else(|error| {
                log::debug!("Failed to read the Caps Lock toggle: {error:#}");
                false
            });
        let changed = type_into_quote(
            &mut text,
            &mut self.held_keys.lock(),
            &mut self.carried_keys.lock(),
            key_event,
            caps_lock,
        );
        if !changed {
            return Ok(());
        }
        self.observable.set_state(|mut state| {
            if let Some(quote) = state.quotes.last_mut() {
                *quote = text;
            }
            state
        })
    }

    /// Caps+Space ends insert mode by closing the quote; Space is held again.
    pub fn finish_insert(&self) -> Result<()> {
        let state = self.observable.get_state()?;
        let mut input = state.input;
        input.push(Key(KeyMappingCode::Quote));
        self.edit_buffer(
            input.clone(),
            state.blocks,
            state.open_block,
            state.quotes.clone(),
            false,
        )?;
        self.held_keys.lock().insert(Key(KeyMappingCode::Space));
        self.run_dispatch(
            self.resolve_dispatch(&input, &state.quotes, &[ChordEventTrigger::Keydown], None)?,
            false,
        )
    }

}

/// `blocks` with the keys of `input` after them made solid as one more block.
fn close_block(input: &[Key], mut blocks: Vec<u32>) -> Vec<u32> {
    let end = input.len() as u32;
    if blocks.last().is_none_or(|&last| last < end) {
        blocks.push(end);
    }
    blocks
}

/// `blocks` after the input changes from `before` to `after`: no block extends past
/// the first changed key, and blocks left empty are dropped.
fn rebase_blocks(blocks: Vec<u32>, before: &[Key], after: &[Key]) -> Vec<u32> {
    let kept = before
        .iter()
        .zip(after)
        .take_while(|(before, after)| before == after)
        .count() as u32;
    let mut rebased = Vec::with_capacity(blocks.len());
    for end in blocks {
        let end = end.min(kept);
        if end > rebased.last().copied().unwrap_or(0) {
            rebased.push(end);
        }
    }
    rebased
}

/// Applies one insert-mode key event to the open quote's `text`, returning whether
/// it changed. Shift and other modifiers come from `held`, which follows the event
/// order. As on macOS, Caps Lock capitalizes letters, with or without Shift. Key
/// repeat types again, as in a text field, except for `carried` keys held since
/// before insert mode began.
fn type_into_quote(
    text: &mut String,
    held: &mut HashSet<Key>,
    carried: &mut HashSet<Key>,
    key_event: &KeyEvent,
    caps_lock: bool,
) -> bool {
    should_handle_held_key_event(held, key_event);
    let key = match key_event {
        KeyEvent::Press(key) => key,
        KeyEvent::Release(key) => {
            carried.remove(key);
            return false;
        }
    };
    if carried.contains(key)
        || Key::modifiers().contains(key)
        || held
            .iter()
            .any(|key| Key::non_shift_modifiers().contains(key))
    {
        return false;
    }
    if key.0 == KeyMappingCode::Backspace {
        return text.pop().is_some();
    }
    let shift = held.iter().any(|key| {
        matches!(
            key.0,
            KeyMappingCode::ShiftLeft | KeyMappingCode::ShiftRight
        )
    });
    match key.text_char(shift || (caps_lock && key.is_letter())) {
        Some(character) => {
            text.push(character);
            true
        }
        None => false,
    }
}

fn should_handle_held_key_event(held_keys: &mut HashSet<Key>, key_event: &KeyEvent) -> bool {
    match key_event {
        KeyEvent::Press(key) => held_keys.insert(*key),
        KeyEvent::Release(key) => {
            held_keys.remove(key);
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_mode_types_text_in_event_order_and_ignores_keys_held_from_chord_mode() {
        use KeyEvent::{Press, Release};
        use KeyMappingCode::*;
        let mut text = String::new();
        let mut held = HashSet::from([Key(Space), Key(Quote)]);
        let mut carried = held.clone();
        let mut apply = |text: &mut String, events: &[KeyEvent], caps_lock: bool| {
            for event in events {
                type_into_quote(text, &mut held, &mut carried, event, caps_lock);
            }
        };
        // Space and the quote were held to open insert mode: their repeats type nothing.
        apply(
            &mut text,
            &[
                Press(Key(Space)),
                Press(Key(Quote)),
                Release(Key(Quote)),
                Release(Key(Space)),
            ],
            false,
        );
        assert_eq!(text, "");
        // Shift follows the event order, so a fast release still types uppercase.
        apply(
            &mut text,
            &[
                Press(Key(ShiftLeft)),
                Press(Key(KeyH)),
                Release(Key(ShiftLeft)),
                Release(Key(KeyH)),
                Press(Key(KeyI)),
                Press(Key(KeyI)),
                Release(Key(KeyI)),
                Press(Key(Space)),
                Release(Key(Space)),
                Press(Key(Quote)),
                Release(Key(Quote)),
                Press(Key(MetaLeft)),
                Press(Key(KeyV)),
                Release(Key(KeyV)),
                Release(Key(MetaLeft)),
                Press(Key(Enter)),
                Press(Key(ArrowLeft)),
            ],
            false,
        );
        assert_eq!(text, "Hii '\n");
        apply(
            &mut text,
            &[Press(Key(Backspace)), Press(Key(Backspace))],
            false,
        );
        assert_eq!(text, "Hii ");
        // Caps Lock capitalizes letters only, and Shift keeps them capitalized.
        apply(
            &mut text,
            &[
                Press(Key(KeyO)),
                Press(Key(Digit1)),
                Press(Key(ShiftLeft)),
                Press(Key(KeyK)),
                Press(Key(Digit1)),
                Release(Key(ShiftLeft)),
            ],
            true,
        );
        assert_eq!(text, "Hii O1K!");
    }

    #[test]
    fn backspace_edits_the_buffer_and_non_text_keys_preserve_it() {
        let buffer = Key::parse_sequence("j,a").unwrap();
        assert_eq!(
            ChordInputManager::next_key_buffer_for_key_press(
                &buffer,
                &[],
                &Key(KeyMappingCode::Backspace)
            ),
            Key::parse_sequence("j,").unwrap()
        );
        assert_eq!(
            ChordInputManager::next_key_buffer_for_key_press(
                &buffer,
                &[],
                &Key(KeyMappingCode::ShiftLeft)
            ),
            buffer
        );
        // Enter is entered like arrows and every other key with sequence notation.
        for key in [KeyMappingCode::ArrowLeft, KeyMappingCode::Enter] {
            let mut expected = buffer.clone();
            expected.push(Key(key));
            assert_eq!(
                ChordInputManager::next_key_buffer_for_key_press(&buffer, &[], &Key(key)),
                expected
            );
        }
        assert!(
            ChordInputManager::next_key_buffer_for_key_press(
                &[],
                &[],
                &Key(KeyMappingCode::Backspace)
            )
            .is_empty()
        );
    }

    #[test]
    fn backspace_removes_keys_after_the_blocks_then_one_whole_block() {
        let backspace = |buffer: &str, blocks: &[u32]| {
            let before = Key::parse_sequence(buffer).unwrap();
            let after = ChordInputManager::next_key_buffer_for_key_press(
                &before,
                blocks,
                &Key(KeyMappingCode::Backspace),
            );
            let blocks = rebase_blocks(blocks.to_vec(), &before, &after);
            (Key::serialize_sequence(&after).unwrap(), blocks)
        };
        // Blocks `g` and `de`, then the loose `f`.
        assert_eq!(backspace("gdef", &[1, 3]), ("gde".into(), vec![1, 3]));
        assert_eq!(backspace("gde", &[1, 3]), ("g".into(), vec![1]));
        assert_eq!(backspace("g", &[1]), ("".into(), vec![]));
    }

    #[test]
    fn shift_blocks_take_in_loose_keys_and_stay_after_a_run() {
        let state = |input: &str, blocks: &[u32], open_block: bool| ChordInputState {
            input: Key::parse_sequence(input).unwrap(),
            blocks: blocks.to_vec(),
            open_block,
            ..Default::default()
        };
        // `g`, then Shift+d: the open block is everything after the solid blocks.
        let open = state("gd", &[], true);
        assert!(!open.has_loose_keys());
        assert_eq!(open.block_input().0, open.input);
        // Releasing Shift makes it one solid block; Shift+a after that is another.
        assert_eq!(close_block(&open.input, open.blocks.clone()), vec![2]);
        assert_eq!(
            close_block(&Key::parse_sequence("gda").unwrap(), vec![2]),
            vec![2, 3]
        );
        // Closing twice adds nothing.
        assert_eq!(close_block(&open.input, vec![2]), vec![2]);
        // Only keys after the blocks run, and a run keeps the blocks.
        let typed = state("gde", &[2], false);
        assert!(typed.has_loose_keys());
        assert_eq!(
            typed.block_input().0,
            Key::parse_sequence("gd").unwrap()
        );
        assert!(!state("gd", &[2], false).has_loose_keys());
        assert!(!ChordInputState::default().has_loose_keys());
    }

    #[test]
    fn blocks_keep_their_quote_texts() {
        let quote = Key(KeyMappingCode::Quote);
        let mut input = vec![quote, quote, Key(KeyMappingCode::KeyA)];
        let block_len = input.len() as u32;
        input.extend([Key(KeyMappingCode::Enter), quote, quote]);
        let state = ChordInputState {
            input,
            quotes: vec!["hi".into(), "there".into()],
            blocks: vec![block_len],
            ..Default::default()
        };
        let (input, quotes) = state.block_input();
        assert_eq!(input.len(), 3);
        assert_eq!(quotes, vec!["hi".to_owned()]);
    }

    #[test]
    fn edits_cut_blocks_at_the_first_changed_key() {
        let keys = |input: &str| Key::parse_sequence(input).unwrap();
        // Appending keeps every block.
        assert_eq!(rebase_blocks(vec![1, 3], &keys("gde"), &keys("gdef")), vec![1, 3]);
        // An expansion that replaces keys from index 2 ends the second block there.
        assert_eq!(rebase_blocks(vec![1, 3], &keys("gde"), &keys("gdxy")), vec![1, 2]);
        // A block left empty is dropped.
        assert_eq!(rebase_blocks(vec![1, 3], &keys("gde"), &keys("gxy")), vec![1]);
        assert_eq!(rebase_blocks(vec![1], &keys("g"), &keys("x")), Vec::<u32>::new());
    }

    #[test]
    fn key_repeat_is_observed_without_reinserting_the_character() {
        let mut held = HashSet::new();
        let key = Key(KeyMappingCode::KeyJ);
        assert!(should_handle_held_key_event(
            &mut held,
            &KeyEvent::Press(key)
        ));
        assert!(!should_handle_held_key_event(
            &mut held,
            &KeyEvent::Press(key)
        ));
        assert!(should_handle_held_key_event(
            &mut held,
            &KeyEvent::Release(key)
        ));
        assert!(should_handle_held_key_event(
            &mut held,
            &KeyEvent::Press(key)
        ));
    }
}
