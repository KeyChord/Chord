use super::chord_loops::ChordPlan;
use crate::app::chord_package_manager::{ChordPackage, ChordReference};
use crate::app::chord_runner::ChordActionTask;
use crate::models::{
    BuiltinChordKind, CORE_CHORD_FILE, CORE_CHORD_PACKAGE, ChordAction, ChordCommand,
    ChordEventTrigger, ChordInputEvent, ChordKeyboardEvent, ChordMacroName, ChordMacros,
    ChordRepeat, ChordRunConfig, ChordStep, ChordTaskAction, Key, MetaChordCommand,
    TypeChordAction, has_pending_escape, is_core_chords_file, serialize_with_quotes,
    undefined_reference,
};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tauri::async_runtime::JoinHandle;
use tokio::sync::oneshot;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ChordIdentity {
    package: String,
    file: PathBuf,
    index: u32,
    pattern: String,
    application: Option<String>,
}

impl ChordIdentity {
    fn new(reference: &ChordReference, event: &ChordInputEvent) -> Self {
        Self {
            package: reference.package_name.clone(),
            file: reference.chords_file_pathslug.clone(),
            index: reference.chord.index,
            pattern: reference.chord.raw_trigger.clone(),
            application: event.application_id.clone(),
        }
    }
}

#[derive(Default)]
pub(super) struct ChordSequence {
    active: HashMap<ChordIdentity, String>,
    pub(super) macros: ChordMacros,
}

#[derive(Default)]
pub(super) struct ChordSequenceDispatch {
    pub tasks: Vec<ChordActionTask>,
    pub expired: Vec<String>,
    pub input_event: Option<ChordInputEvent>,
    pub description: Option<String>,
    pub meta_command: Option<MetaChordCommand>,
    pub plan: Option<ChordPlan>,
    /// Why the requested input trigger has nothing to run. Unset when the input
    /// still matches chords that only handle keyboard events.
    pub error: Option<String>,
}

impl ChordSequence {
    pub fn dispatch_command(
        &mut self,
        raw_event: ChordInputEvent,
        triggers: &[ChordEventTrigger],
        keyboard_event: Option<&ChordKeyboardEvent>,
        mut matching: impl FnMut(&ChordInputEvent) -> Vec<(ChordPackage, ChordReference)>,
    ) -> Result<ChordSequenceDispatch> {
        if raw_event
            .input
            .first()
            .is_some_and(|key| key.to_sequence_char() == Some('0'))
        {
            if !triggers.contains(&ChordEventTrigger::Input) {
                return Ok(ChordSequenceDispatch {
                    expired: self.reset(),
                    ..Default::default()
                });
            }
            // Resolve configured core setters before expansion so numbered buffer
            // references in their bodies are saved verbatim. Escapes still await Tab.
            // All other commands can use macros in their arguments.
            if !has_pending_escape(&raw_event.input) {
                let event = raw_event.clone();
                for (_, reference) in matching(&event) {
                    if !is_core_chords_file(
                        &reference.package_name,
                        &reference.chords_file_pathslug,
                    ) {
                        continue;
                    }
                    if let Some(ChordAction::Builtin(action)) = reference.chord.actions.first() {
                        if matches!(
                            action.command,
                            BuiltinChordKind::SetBuffer | BuiltinChordKind::SetMacro
                        ) {
                            let command = action.resolve(&reference.chord.trigger, &event.input)?;
                            return Ok(ChordSequenceDispatch {
                                expired: self.reset(),
                                input_event: Some(raw_event),
                                description: Some(reference.chord.description(&event.input)),
                                meta_command: Some(command),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
        }
        let command = ChordCommand::parse(&raw_event.input, &raw_event.quotes, &self.macros)?;
        let (steps, repeat, config) = match command {
            ChordCommand::Meta { keys } => {
                let event = ChordInputEvent {
                    input: keys,
                    application_id: raw_event.application_id.clone(),
                    ..Default::default()
                };
                let mut command = None;
                let mut description = None;
                for (_, reference) in matching(&event) {
                    if !is_core_chords_file(
                        &reference.package_name,
                        &reference.chords_file_pathslug,
                    ) {
                        continue;
                    }
                    if let Some(ChordAction::Builtin(action)) = reference.chord.actions.first() {
                        description = Some(reference.chord.description(&event.input));
                        if triggers.contains(&ChordEventTrigger::Input) {
                            command = Some(action.resolve(&reference.chord.trigger, &event.input)?);
                        }
                        break;
                    }
                }
                let error = (command.is_none() && triggers.contains(&ChordEventTrigger::Input))
                    .then(|| no_chord_found(1, &event.input));
                return Ok(ChordSequenceDispatch {
                    expired: self.reset(),
                    input_event: command.as_ref().map(|_| raw_event),
                    meta_command: command,
                    description,
                    error,
                    ..Default::default()
                });
            }
            ChordCommand::Insert { .. } => {
                return Ok(ChordSequenceDispatch {
                    expired: self.reset(),
                    description: Some(
                        "Insert mode · Caps+Space closes the quote · Esc cancels".into(),
                    ),
                    ..Default::default()
                });
            }
            ChordCommand::Pending => {
                let error = triggers.contains(&ChordEventTrigger::Input).then(|| {
                    if let Some(reference) = undefined_reference(&raw_event.input, &self.macros) {
                        format!("`{reference}` references an undefined macro")
                    } else if has_pending_escape(&raw_event.input) {
                        "Press Tab to expand the escape".into()
                    } else {
                        "Incomplete or invalid chord".into()
                    }
                });
                return Ok(ChordSequenceDispatch {
                    expired: self.reset(),
                    error,
                    ..Default::default()
                });
            }
            ChordCommand::Run {
                steps,
                repeat,
                config,
            } => (steps, repeat, config),
        };
        // Plain finite repeats keep running inside one task. Loops, configured delays or
        // intervals, and multi-step sequences run as a background plan that can be halted.
        let is_plan =
            steps.len() > 1 || repeat == ChordRepeat::Loop || config != ChordRunConfig::default();
        let (last, earlier) = steps.split_last().expect("parsed runs are never empty");
        // A quote matches no chord, so typing one also ends any lifecycle scopes.
        let mut resolve = |step: &ChordStep| match step {
            ChordStep::Chord(keys) => {
                let event = ChordInputEvent {
                    input: keys.clone(),
                    application_id: raw_event.application_id.clone(),
                    ..Default::default()
                };
                let contexts = matching(&event);
                (event, contexts)
            }
            ChordStep::Quote { .. } => (
                ChordInputEvent {
                    application_id: raw_event.application_id.clone(),
                    ..Default::default()
                },
                vec![],
            ),
        };
        // Keyboard callbacks and the typing scope belong to the step being typed.
        let (event, contexts) = resolve(last);
        let num_times = match repeat {
            ChordRepeat::Times(count) if !is_plan => count,
            _ => 1,
        };
        let mut dispatch = self.dispatch(&event, &contexts, triggers, keyboard_event, num_times)?;
        if let ChordStep::Quote { text, intervals } = last {
            dispatch.description = Some(describe_quote(text, intervals));
            if triggers.contains(&ChordEventTrigger::Input) {
                dispatch
                    .tasks
                    .push(type_task(event.clone(), text, intervals, num_times));
                dispatch.input_event = Some(event.clone());
            }
        }
        let earlier: Vec<_> = earlier
            .iter()
            .map(|step| {
                let (event, contexts) = resolve(step);
                (step, event, contexts)
            })
            .collect();
        if let Some(description) = &mut dispatch.description {
            let mut names: Vec<String> = earlier
                .iter()
                .map(|(step, event, contexts)| match step {
                    ChordStep::Quote { text, intervals } => describe_quote(text, intervals),
                    ChordStep::Chord(_) => Self::describe(event, contexts).unwrap_or_else(|| {
                        Key::serialize_sequence(&event.input).unwrap_or_default()
                    }),
                })
                .collect();
            names.push(std::mem::take(description));
            *description = describe_run(repeat, &config, &names);
        }
        if dispatch.input_event.is_none() {
            // Chords that only handle keyboard events still matched the typed keys.
            if triggers.contains(&ChordEventTrigger::Input) && contexts.is_empty() {
                dispatch.error = Some(no_chord_found(steps.len(), &event.input));
            }
            return Ok(dispatch);
        }
        if !is_plan {
            dispatch.input_event = Some(raw_event);
            return Ok(dispatch);
        }
        // Earlier chords get their own scopes, owned by the plan like the last one.
        let mut plan_steps = Vec::new();
        let mut init_tasks = Vec::new();
        let mut scopes = Vec::new();
        for (step, event, contexts) in &earlier {
            if let ChordStep::Quote { text, intervals } = step {
                plan_steps.push(type_task(event.clone(), text, intervals, 1));
                continue;
            }
            let mut isolated = ChordSequence::default();
            let mut resolved = isolated.dispatch(
                event,
                contexts,
                &[ChordEventTrigger::Input],
                keyboard_event,
                1,
            )?;
            if resolved.input_event.is_none() {
                // Nothing has run in these scopes yet, so they need no release. The
                // typed step's input task must not run on its own either.
                dispatch.tasks.pop();
                dispatch.input_event = None;
                dispatch.error = Some(no_chord_found(steps.len(), &event.input));
                return Ok(dispatch);
            }
            plan_steps.extend(resolved.tasks.pop());
            init_tasks.append(&mut resolved.tasks);
            scopes.extend(isolated.reset());
        }
        let last_task = dispatch.tasks.pop().expect("resolved input has a task");
        if let ChordTaskAction::Handler(action) = &last_task.action {
            if let Some(id) = &action.sequence_id {
                self.detach(id);
                scopes.push(id.clone());
            }
        }
        plan_steps.push(last_task);
        dispatch.tasks.append(&mut init_tasks);
        dispatch.plan = Some(ChordPlan {
            input: serialize_with_quotes(&raw_event.input, &raw_event.quotes)
                .context("invalid chord sequence")?,
            steps: plan_steps,
            repeat,
            delay: config.delay.unwrap_or_default(),
            intervals: config.intervals,
            scopes,
        });
        dispatch.input_event = Some(raw_event);
        Ok(dispatch)
    }

    /// The chord whose input action would run, else any chord with the same keys.
    fn describe(
        event: &ChordInputEvent,
        contexts: &[(ChordPackage, ChordReference)],
    ) -> Option<String> {
        contexts
            .iter()
            .find(|(_, reference)| {
                reference
                    .chord
                    .actions
                    .iter()
                    .any(|action| action.trigger() == ChordEventTrigger::Input)
            })
            .or_else(|| contexts.first())
            .map(|(_, reference)| reference.chord.description(&event.input))
    }

    /// Quote texts after the buffer changes to `input`, and whether it is now in
    /// insert mode (ends with an open quote). A quote `opened` by the key just typed
    /// starts empty; one reopened by deleting its closing `'` keeps its text.
    pub fn edit_quotes(
        &self,
        input: &[Key],
        mut quotes: Vec<String>,
        opened: bool,
    ) -> Result<(Vec<String>, bool)> {
        let ChordCommand::Insert { quote } = ChordCommand::parse(input, &quotes, &self.macros)?
        else {
            return Ok((quotes, false));
        };
        if opened {
            quotes.truncate(quote);
        }
        if quotes.len() <= quote {
            quotes.resize(quote + 1, String::new());
        }
        Ok((quotes, true))
    }

    pub fn reset(&mut self) -> Vec<String> {
        self.active.drain().map(|(_, id)| id).collect()
    }

    pub fn set_macro(&mut self, name: ChordMacroName, sequence: String) {
        self.macros.set(name, sequence);
    }

    /// A background loop owns its persistent values until it halts or fails.
    pub fn detach(&mut self, id: &str) {
        self.active.retain(|_, active_id| active_id != id);
    }

    pub fn dispatch(
        &mut self,
        event: &ChordInputEvent,
        contexts: &[(ChordPackage, ChordReference)],
        triggers: &[ChordEventTrigger],
        keyboard_event: Option<&ChordKeyboardEvent>,
        num_times: u32,
    ) -> Result<ChordSequenceDispatch> {
        let matching: HashSet<_> = contexts
            .iter()
            .filter(|(_, r)| {
                r.chord
                    .actions
                    .iter()
                    .any(|a| matches!(a, ChordAction::Emit(_)))
            })
            .map(|(_, r)| ChordIdentity::new(r, event))
            .collect();
        let mut dispatch = ChordSequenceDispatch::default();
        dispatch.description = Self::describe(event, contexts);
        self.active.retain(|identity, id| {
            if matching.contains(identity) {
                true
            } else {
                dispatch.expired.push(id.clone());
                false
            }
        });
        // All initializers precede the keyboard/input callbacks for this buffer snapshot.
        for (package, reference) in contexts {
            if !reference
                .chord
                .actions
                .iter()
                .any(|a| matches!(a, ChordAction::Emit(_)))
            {
                continue;
            }
            let identity = ChordIdentity::new(reference, event);
            if !self.active.contains_key(&identity) {
                let id = uuid::Uuid::new_v4().to_string();
                self.active.insert(identity, id.clone());
                Self::append_tasks(
                    &mut dispatch.tasks,
                    package,
                    reference,
                    event,
                    ChordEventTrigger::Init,
                    &id,
                    keyboard_event,
                    1,
                )?;
            }
        }
        for &trigger in triggers {
            for (package, reference) in contexts {
                let id = self.active.get(&ChordIdentity::new(reference, event));
                if trigger == ChordEventTrigger::Input {
                    if let Some(action) = reference
                        .chord
                        .actions
                        .iter()
                        .find(|a| a.trigger() == trigger)
                    {
                        if let Some(task) = package.resolve_action_task(
                            event.clone(),
                            reference,
                            action,
                            num_times,
                            id.cloned(),
                            keyboard_event.cloned(),
                        )? {
                            dispatch.input_event = Some(event.clone());
                            dispatch.description = Some(reference.chord.description(&event.input));
                            dispatch.tasks.push(task);
                        }
                        // Ordinary chord completion retains the existing first-match priority.
                        break;
                    }
                } else if let Some(id) = id {
                    Self::append_tasks(
                        &mut dispatch.tasks,
                        package,
                        reference,
                        event,
                        trigger,
                        id,
                        keyboard_event,
                        1,
                    )?;
                }
            }
        }
        Ok(dispatch)
    }

    fn append_tasks(
        tasks: &mut Vec<ChordActionTask>,
        package: &ChordPackage,
        reference: &ChordReference,
        event: &ChordInputEvent,
        trigger: ChordEventTrigger,
        id: &str,
        keyboard_event: Option<&ChordKeyboardEvent>,
        num_times: u32,
    ) -> Result<()> {
        let keyboard_event = keyboard_event.cloned().map(|mut event| {
            if trigger == ChordEventTrigger::Keypress {
                event.r#type = "keypress".into();
            }
            event
        });
        for action in reference
            .chord
            .actions
            .iter()
            .filter(|a| a.trigger() == trigger)
        {
            if let Some(task) = package.resolve_action_task(
                event.clone(),
                reference,
                action,
                num_times,
                Some(id.to_string()),
                keyboard_event.clone(),
            )? {
                tasks.push(task);
            }
        }
        Ok(())
    }
}

/// Names the chord only within a sequence; a lone chord is already on screen.
fn no_chord_found(chords: usize, input: &[Key]) -> String {
    match Key::serialize_sequence(input) {
        Some(sequence) if chords > 1 => format!("No chord found for \"{sequence}\""),
        _ => "No chord found".into(),
    }
}

fn format_pauses(pauses: &[Duration]) -> String {
    let pauses: Vec<_> = pauses
        .iter()
        .map(|pause| format!("{}ms", pause.as_millis()))
        .collect();
    pauses.join(", ")
}

fn describe_run(repeat: ChordRepeat, config: &ChordRunConfig, steps: &[String]) -> String {
    let mut parts = Vec::new();
    match repeat {
        ChordRepeat::Times(1) => {}
        ChordRepeat::Times(count) => parts.push(format!("Repeat {count} times")),
        ChordRepeat::Loop => parts.push("Loop".into()),
    }
    if let Some(delay) = config.delay {
        parts.push(format!("Delay {}", format_pauses(&[delay])));
    }
    if !config.intervals.is_empty() {
        parts.push(format!("Interval {}", format_pauses(&config.intervals)));
    }
    parts.push(steps.join(" → "));
    parts.join(" · ")
}

fn describe_quote(text: &str, intervals: &[Duration]) -> String {
    let text = text.escape_debug();
    if intervals.is_empty() {
        format!("Type '{text}'")
    } else {
        format!("Type '{text}' ({} apart)", format_pauses(intervals))
    }
}

/// Typing quoted keys belongs to Chord itself rather than to a package.
fn type_task(
    event: ChordInputEvent,
    text: &str,
    intervals: &[Duration],
    num_times: u32,
) -> ChordActionTask {
    ChordActionTask {
        package_name: CORE_CHORD_PACKAGE.into(),
        initiator_file_pathslug: CORE_CHORD_FILE.into(),
        action: ChordTaskAction::Type(TypeChordAction {
            text: text.into(),
            intervals_ms: intervals
                .iter()
                .map(|interval| u32::try_from(interval.as_millis()).unwrap_or(u32::MAX))
                .collect(),
        }),
        event,
        num_times,
    }
}

/// Reserve callback order on the keyboard thread, before spawning any async work.
#[derive(Default)]
pub(super) struct ChordTaskQueue {
    tail: Mutex<Option<oneshot::Receiver<()>>>,
}

impl ChordTaskQueue {
    pub fn enqueue(&self, job: impl Future<Output = ()> + Send + 'static) -> JoinHandle<()> {
        let (completed, next) = oneshot::channel();
        let previous = self.tail.lock().replace(next);
        tauri::async_runtime::spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            job.await;
            let _ = completed.send(());
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        ChordTaskAction, CompiledChordsFile, CompiledChordsFileHandler, Key, KeyEvent,
        ParsedChordsFile,
    };
    use keycode::KeyMappingCode;
    use std::sync::Arc;

    fn fixture(source: &str) -> ChordPackage {
        let parsed: ParsedChordsFile = source.parse().unwrap();
        let path = PathBuf::from("chords/dev/fixture/macos.toml");
        let handlers = ["prepare", "down", "press", "up", "input", "instagram"]
            .map(|event| CompiledChordsFileHandler {
                event: event.into(),
                handler_id: event.into(),
            })
            .to_vec();
        let file = CompiledChordsFile {
            name: parsed.name,
            pathslug: path.clone(),
            meta: parsed.meta,
            handlers,
            executables: parsed.executables,
            chords: parsed.chords,
            chord_hints: parsed.chord_hints,
        };
        ChordPackage {
            name: "@fixture/test".into(),
            js_package: None,
            raw_chords_files: HashMap::new(),
            compiled_chords_files: HashMap::from([(path, file)]),
            global_chords: vec![],
        }
    }

    fn event(input: &str) -> ChordInputEvent {
        ChordInputEvent {
            input: Key::parse_sequence(input).unwrap(),
            application_id: Some("dev.fixture".into()),
            ..Default::default()
        }
    }

    /// Each character as the key that types it; escapes are never notation here.
    fn typed(input: &str) -> ChordInputEvent {
        ChordInputEvent {
            input: input
                .chars()
                .map(|c| c.to_string().parse().unwrap())
                .collect(),
            application_id: Some("dev.fixture".into()),
            ..Default::default()
        }
    }

    /// Typing `input`, then pressing Tab.
    fn tab(sequence: &ChordSequence, input: &str) -> ChordInputEvent {
        ChordInputEvent {
            input: crate::models::select_escape(&typed(input).input, &sequence.macros)
                .unwrap()
                .unwrap(),
            ..typed(input)
        }
    }

    fn contexts(
        package: &ChordPackage,
        event: &ChordInputEvent,
    ) -> Vec<(ChordPackage, ChordReference)> {
        package
            .matching_app_chords(event)
            .into_iter()
            .map(|r| (package.clone(), r))
            .collect()
    }

    fn core_fixture(source: &str) -> ChordPackage {
        use crate::models::{CORE_CHORD_FILE, CORE_CHORD_PACKAGE};
        let mut core = fixture("name = 'fixture'");
        core.name = CORE_CHORD_PACKAGE.into();
        core.global_chords = ParsedChordsFile::parse_core(source)
            .unwrap()
            .chords
            .into_iter()
            .map(|chord| ChordReference {
                package_name: core.name.clone(),
                chords_file_pathslug: PathBuf::from(CORE_CHORD_FILE),
                chord,
            })
            .collect();
        core
    }

    fn save_macro(sequence: &mut ChordSequence, core: &ChordPackage, input: &str) {
        let dispatch = sequence
            .dispatch_command(event(input), &[ChordEventTrigger::Input], None, |e| {
                core.matching_global_chords(e)
                    .into_iter()
                    .map(|r| (core.clone(), r))
                    .collect()
            })
            .unwrap();
        assert_eq!(dispatch.input_event, Some(event(input)));
        assert!(dispatch.tasks.is_empty() && dispatch.plan.is_none());
        let Some(MetaChordCommand::SetMacro {
            name,
            sequence: value,
        }) = dispatch.meta_command
        else {
            panic!("expected core macro setter")
        };
        // The input manager applies the resolved native operation on completion.
        sequence.set_macro(name, value);
    }

    fn handlers(dispatch: &ChordSequenceDispatch) -> Vec<&str> {
        dispatch
            .tasks
            .iter()
            .map(|task| match &task.action {
                ChordTaskAction::Handler(action) => action.handler_id.as_str(),
                _ => panic!("expected handler"),
            })
            .collect()
    }

    #[test]
    fn story_input_tasks_preserve_the_sequence_and_resolve_capture_arguments() {
        let package = fixture(
            r#"
name = "Instagram"
[chords]
'hs.(\d+)' = { '@input:instagram' = ['story', '$1'] }
'hsr(\d+)' = { name = "Home: Stories > Reply @ #", '@input:instagram' = ['story', '$1'] }
"#,
        );
        for input in ["hs.12", "hsr12"] {
            let input_event = event(input);
            let reference = package
                .resolve_chord_from_input_event(&input_event)
                .unwrap();
            let task = package
                .resolve_task(input_event, reference, 1)
                .unwrap()
                .unwrap();
            let ChordTaskAction::Handler(action) = task.action else {
                panic!("expected Instagram handler");
            };
            assert_eq!(action.handler_id, "instagram");
            assert_eq!(action.input, input);
            assert_eq!(action.event_args, vec!["story".into(), "12".into()]);
        }
        for input in ["hs,12", "xhsr12", "hsr12x", "hsr"] {
            assert!(
                package
                    .resolve_chord_from_input_event(&event(input))
                    .is_none()
            );
        }
    }

    #[test]
    fn descriptions_follow_resolved_captures_macros_and_repeat_counts() {
        let package = fixture(
            r#"name = 'fixture'
[chords]
'hsr(\d+)' = { name = 'Reply to story $1', '@input:input' = ['$1'] }
"#,
        );
        let mut sequence = ChordSequence::default();
        sequence.set_macro(ChordMacroName::Buffer(1), "hsr34".into());
        for (input, description) in [
            ("hsr34", "Reply to story 34"),
            ("12hsr34", "Repeat 12 times · Reply to story 34"),
            (r"\1", "Reply to story 34"),
            (r"12\1", "Repeat 12 times · Reply to story 34"),
            ("1hsr34", "Loop · Reply to story 34"),
        ] {
            for trigger in [ChordEventTrigger::Keydown, ChordEventTrigger::Input] {
                let dispatch = sequence
                    .dispatch_command(event(input), &[trigger], None, |e| contexts(&package, e))
                    .unwrap();
                assert_eq!(dispatch.description.as_deref(), Some(description));
                assert_eq!(
                    dispatch.input_event.is_some(),
                    trigger == ChordEventTrigger::Input
                );
            }
        }
        let core = core_fixture(include_str!(concat!(
            env!("OUT_DIR"),
            "/chords-chord.macos.toml"
        )));
        let dispatch = sequence
            .dispatch_command(event("0h12"), &[ChordEventTrigger::Input], None, |e| {
                core.matching_global_chords(e)
                    .into_iter()
                    .map(|r| (core.clone(), r))
                    .collect()
            })
            .unwrap();
        assert_eq!(dispatch.description.as_deref(), Some("Halt loop #12"));
    }

    #[test]
    fn init_runs_once_until_mismatch_then_restarts_with_a_new_scope() {
        let package = fixture(
            r#"
name = "fixture"
[chords]
'j,(.*)' = { '@init:prepare=state' = [], '@keydown:down=last' = ['$state', '$1'], '@keypress:press' = ['$last'], '@keyup:up' = ['$state'], '@input:input' = ['$state'] }
"#,
        );
        let mut sequence = ChordSequence::default();
        let keyboard_event =
            ChordKeyboardEvent::new(&KeyEvent::Press(Key(KeyMappingCode::Comma)), &[], false);
        let e = event("j,");
        let first = sequence
            .dispatch(
                &e,
                &contexts(&package, &e),
                &[ChordEventTrigger::Keydown, ChordEventTrigger::Keypress],
                Some(&keyboard_event),
                1,
            )
            .unwrap();
        assert_eq!(handlers(&first), ["prepare", "down", "press"]);
        let ChordTaskAction::Handler(init) = &first.tasks[0].action else {
            panic!()
        };
        assert_eq!(init.assign_to.as_deref(), Some("state"));
        let id = init.sequence_id.clone();
        let ChordTaskAction::Handler(down) = &first.tasks[1].action else {
            panic!()
        };
        assert_eq!(down.assign_to.as_deref(), Some("last"));
        assert_eq!(down.event_args[1].as_str(), Some(""));
        let ChordTaskAction::Handler(press) = &first.tasks[2].action else {
            panic!()
        };
        assert_eq!(press.keyboard_event.as_ref().unwrap().r#type, "keypress");

        for (input, trigger, expected) in [
            ("j,a", ChordEventTrigger::Keydown, "down"),
            ("j,a", ChordEventTrigger::Keyup, "up"),
            ("j,", ChordEventTrigger::Keydown, "down"),
            ("j,", ChordEventTrigger::Input, "input"),
        ] {
            let e = event(input);
            let next = sequence
                .dispatch(&e, &contexts(&package, &e), &[trigger], None, 1)
                .unwrap();
            assert_eq!(handlers(&next), [expected]);
            let ChordTaskAction::Handler(action) = &next.tasks[0].action else {
                panic!()
            };
            assert_eq!(action.sequence_id, id);
        }
        let mismatch = event("j");
        let exited = sequence
            .dispatch(
                &mismatch,
                &contexts(&package, &mismatch),
                &[ChordEventTrigger::Keydown],
                None,
                1,
            )
            .unwrap();
        assert!(exited.tasks.is_empty());
        assert_eq!(exited.expired, [id.clone().unwrap()]);
        let reentered = sequence
            .dispatch(
                &e,
                &contexts(&package, &e),
                &[ChordEventTrigger::Keydown],
                None,
                1,
            )
            .unwrap();
        assert_eq!(handlers(&reentered), ["prepare", "down"]);
        let ChordTaskAction::Handler(init) = &reentered.tasks[0].action else {
            panic!()
        };
        assert_ne!(init.sequence_id, id);
        assert_eq!(sequence.reset(), [init.sequence_id.clone().unwrap()]);
    }

    #[test]
    fn overlapping_lifecycle_chords_do_not_shadow_input_and_initialize_independently() {
        let package = fixture(
            r#"
name = "fixture"
[chords]
'j,' = { name = "Keyboard callbacks", '@init:prepare' = [], '@keydown:down' = [] }
'j,(.*)' = { name = "Execute input", '@init:prepare' = [], '@keydown:down' = [], '@input:input' = ['$1'] }
"#,
        );
        let e = event("j,");
        let references = contexts(&package, &e);
        let resolved = package.resolve_chord_from_input_event(&e).unwrap();
        assert_eq!(resolved.chord.raw_trigger, "j,(.*)");
        let mut sequence = ChordSequence::default();
        let dispatch = sequence
            .dispatch(
                &e,
                &references,
                &[ChordEventTrigger::Keydown, ChordEventTrigger::Input],
                None,
                1,
            )
            .unwrap();
        assert_eq!(
            handlers(&dispatch),
            ["prepare", "prepare", "down", "down", "input"]
        );
        let ChordTaskAction::Handler(first) = &dispatch.tasks[0].action else {
            panic!()
        };
        let ChordTaskAction::Handler(second) = &dispatch.tasks[1].action else {
            panic!()
        };
        assert_ne!(first.sequence_id, second.sequence_id);
        assert!(dispatch.input_event.is_some());
        assert_eq!(dispatch.description.as_deref(), Some("Execute input"));
    }

    #[test]
    fn shortcut_only_chords_do_not_create_javascript_scopes() {
        let package = fixture("name = 'fixture'\n[chords]\nab = { shortcut = 'cmd+a' }");
        let e = event("ab");
        let mut sequence = ChordSequence::default();
        let dispatch = sequence
            .dispatch(
                &e,
                &contexts(&package, &e),
                &[ChordEventTrigger::Keydown, ChordEventTrigger::Input],
                None,
                1,
            )
            .unwrap();
        assert_eq!(dispatch.tasks.len(), 1);
        assert!(matches!(
            dispatch.tasks[0].action,
            ChordTaskAction::Shortcut(_)
        ));
        assert!(sequence.reset().is_empty());
    }

    #[test]
    fn repeated_input_keeps_capture_arguments_and_initializes_only_once() {
        let package = fixture(
            r#"name = 'fixture'
[chords]
'hsr(\d+)' = { '@init:prepare=state' = [], '@input:input' = ['$state', '$1'] }
"#,
        );
        let mut sequence = ChordSequence::default();
        let raw = event("12hsr34");
        let dispatch = sequence
            .dispatch_command(raw.clone(), &[ChordEventTrigger::Input], None, |e| {
                contexts(&package, e)
            })
            .unwrap();
        assert_eq!(handlers(&dispatch), ["prepare", "input"]);
        assert_eq!(dispatch.tasks[0].num_times, 1);
        assert_eq!(dispatch.tasks[1].num_times, 12);
        assert_eq!(dispatch.input_event, Some(raw));
        let ChordTaskAction::Handler(action) = &dispatch.tasks[1].action else {
            panic!()
        };
        assert_eq!(action.input, "hsr34");
        assert_eq!(action.event_args, ["$state".into(), "34".into()]);
    }

    #[test]
    fn loop_scopes_survive_buffer_reset_and_meta_commands_bypass_packages() {
        let package = fixture(
            "name = 'fixture'\n[chords]\nab = { '@init:prepare=state' = [], '@input:input' = ['$state'] }",
        );
        let mut sequence = ChordSequence::default();
        let dispatch = sequence
            .dispatch_command(event("1ab"), &[ChordEventTrigger::Input], None, |e| {
                contexts(&package, e)
            })
            .unwrap();
        assert_eq!(handlers(&dispatch), ["prepare"]);
        let plan = dispatch.plan.unwrap();
        assert_eq!(
            (plan.input.as_str(), plan.repeat),
            ("1ab", ChordRepeat::Loop)
        );
        assert!(plan.delay.is_zero() && plan.intervals.is_empty());
        let [task] = &plan.steps[..] else { panic!() };
        assert_eq!(task.num_times, 1);
        let ChordTaskAction::Handler(action) = &task.action else {
            panic!()
        };
        assert!(action.sequence_id.is_some());
        assert_eq!(plan.scopes, [action.sequence_id.clone().unwrap()]);
        assert!(sequence.reset().is_empty());
        let later = sequence
            .dispatch_command(event("ab"), &[ChordEventTrigger::Input], None, |e| {
                contexts(&package, e)
            })
            .unwrap();
        let ChordTaskAction::Handler(later_action) = &later.tasks[0].action else {
            panic!()
        };
        assert_ne!(later_action.sequence_id, action.sequence_id);
        let typing = sequence
            .dispatch_command(event("0s"), &[ChordEventTrigger::Keydown], None, |_| {
                panic!("meta commands must not visit packages")
            })
            .unwrap();
        assert!(typing.meta_command.is_none());
    }

    fn run_with(
        sequence: &mut ChordSequence,
        package: &ChordPackage,
        input: &str,
        trigger: ChordEventTrigger,
    ) -> ChordSequenceDispatch {
        sequence
            .dispatch_command(event(input), &[trigger], None, |e| contexts(package, e))
            .unwrap()
    }

    #[test]
    fn configured_and_separated_runs_become_haltable_plans() {
        let package = fixture(
            r#"name = 'fixture'
[chords]
'/w]' = { name = 'Close tab', shortcut = 'cmd+w' }
'a\n' = { name = 'Confirm', shortcut = 'enter' }
"#,
        );
        let mut sequence = ChordSequence::default();
        let input = r"3`i13`/w]\n/w]";
        let dispatch = run_with(&mut sequence, &package, input, ChordEventTrigger::Input);
        assert_eq!(dispatch.input_event, Some(event(input)));
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Repeat 3 times · Interval 100ms, 300ms · Close tab → Close tab")
        );
        assert!(dispatch.tasks.is_empty());
        let plan = dispatch.plan.unwrap();
        assert_eq!(
            (plan.input.as_str(), plan.repeat),
            (input, ChordRepeat::Times(3))
        );
        assert!(plan.delay.is_zero());
        assert_eq!(
            plan.intervals,
            [100, 300].map(std::time::Duration::from_millis)
        );
        assert_eq!(plan.steps.len(), 2);
        for step in &plan.steps {
            assert!(matches!(step.action, ChordTaskAction::Shortcut(_)));
            assert_eq!(step.num_times, 1);
            assert_eq!(step.event.input, Key::parse_sequence("/w]").unwrap());
        }
        assert!(plan.scopes.is_empty());

        // A configured single chord still waits between its own runs.
        let dispatch = run_with(
            &mut sequence,
            &package,
            "3`i5`/w]",
            ChordEventTrigger::Input,
        );
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Repeat 3 times · Interval 500ms · Close tab")
        );
        let plan = dispatch.plan.unwrap();
        assert_eq!((plan.steps.len(), plan.steps[0].num_times), (1, 1));
        // A delay waits once, before the first run, with or without a count.
        let dispatch = run_with(
            &mut sequence,
            &package,
            "3`d2,i130`/w]",
            ChordEventTrigger::Input,
        );
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Repeat 3 times · Delay 200ms · Interval 100ms, 300ms, 0ms · Close tab")
        );
        let plan = dispatch.plan.unwrap();
        assert_eq!(plan.delay, std::time::Duration::from_millis(200));
        assert_eq!(
            plan.intervals,
            [100, 300, 0].map(std::time::Duration::from_millis)
        );
        let dispatch = run_with(&mut sequence, &package, "`d5`/w]", ChordEventTrigger::Input);
        assert!(dispatch.tasks.is_empty());
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Delay 500ms · Close tab")
        );
        let plan = dispatch.plan.unwrap();
        assert_eq!(
            (plan.delay, plan.repeat, plan.steps.len()),
            (
                std::time::Duration::from_millis(500),
                ChordRepeat::Times(1),
                1
            )
        );
        // Unconfigured repeats keep the fast in-task repetition.
        let dispatch = run_with(&mut sequence, &package, "3/w]", ChordEventTrigger::Input);
        assert!(dispatch.plan.is_none());
        assert_eq!(dispatch.tasks[0].num_times, 3);

        // While the next chord is being typed, nothing is executable.
        let typing = run_with(
            &mut sequence,
            &package,
            r"3`i13`/w]\n/w",
            ChordEventTrigger::Keydown,
        );
        assert!(typing.plan.is_none() && typing.input_event.is_none());
        // Enter stays an ordinary key outside a count or config block.
        let dispatch = run_with(&mut sequence, &package, r"a\n", ChordEventTrigger::Input);
        assert!(dispatch.plan.is_none());
        assert_eq!(dispatch.description.as_deref(), Some("Confirm"));
    }

    #[test]
    fn sequence_steps_own_independent_handler_scopes_and_need_every_chord() {
        let package = fixture(
            r#"name = 'fixture'
[chords]
'hsr(\d+)' = { name = 'Reply to story $1', '@init:prepare=state' = [], '@input:input' = ['$state', '$1'] }
'/w]' = { name = 'Close tab', shortcut = 'cmd+w' }
"#,
        );
        let mut sequence = ChordSequence::default();
        let typed = run_with(
            &mut sequence,
            &package,
            r"2`i0`hsr12\nhsr34",
            ChordEventTrigger::Keydown,
        );
        assert_eq!(handlers(&typed), ["prepare"]);
        assert_eq!(
            typed.description.as_deref(),
            Some("Repeat 2 times · Interval 0ms · Reply to story 12 → Reply to story 34")
        );
        let dispatch = run_with(
            &mut sequence,
            &package,
            r"2`i0`hsr12\nhsr34",
            ChordEventTrigger::Input,
        );
        // The typed chord reuses its scope; the earlier chord initializes its own.
        assert_eq!(handlers(&dispatch), ["prepare"]);
        let plan = dispatch.plan.unwrap();
        let steps: Vec<_> = plan
            .steps
            .iter()
            .map(|task| match &task.action {
                ChordTaskAction::Handler(action) => action.clone(),
                _ => panic!("expected handler"),
            })
            .collect();
        assert_eq!(steps[0].event_args, ["$state".into(), "12".into()]);
        assert_eq!(steps[1].event_args, ["$state".into(), "34".into()]);
        let ChordTaskAction::Handler(init) = &dispatch.tasks[0].action else {
            panic!()
        };
        assert_eq!(init.sequence_id, steps[0].sequence_id);
        assert_ne!(steps[0].sequence_id, steps[1].sequence_id);
        assert_eq!(
            plan.scopes,
            [
                steps[0].sequence_id.clone().unwrap(),
                steps[1].sequence_id.clone().unwrap()
            ]
        );
        // The plan owns both scopes, so resetting the input releases neither.
        assert!(sequence.reset().is_empty());

        // An unknown chord anywhere in the sequence prevents every step from running.
        let dispatch = run_with(
            &mut sequence,
            &package,
            r"2xx\n/w]",
            ChordEventTrigger::Input,
        );
        assert!(dispatch.input_event.is_none() && dispatch.plan.is_none());
        assert!(dispatch.tasks.is_empty());
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Repeat 2 times · xx → Close tab")
        );
    }

    #[test]
    fn unresolved_input_explains_why_nothing_runs() {
        let package = fixture(
            r#"name = 'fixture'
[chords]
'/w]' = { name = 'Close tab', shortcut = 'cmd+w' }
'j,' = { name = 'Keyboard callbacks', '@keydown:down' = [] }
"#,
        );
        let mut sequence = ChordSequence::default();
        for (input, error) in [
            ("xx", Some("No chord found")),
            (r"2xx\n/w]", Some(r#"No chord found for "xx""#)),
            (r"2/w]\nxx", Some(r#"No chord found for "xx""#)),
            ("12", Some("Incomplete or invalid chord")),
            // Chords that only handle keyboard events have no input action to miss.
            ("j,", None),
        ] {
            let dispatch = run_with(&mut sequence, &package, input, ChordEventTrigger::Input);
            assert!(dispatch.input_event.is_none(), "{input}");
            assert_eq!(dispatch.error.as_deref(), error, "{input}");
            // Input that is still being typed is never an error.
            let typing = run_with(&mut sequence, &package, input, ChordEventTrigger::Keydown);
            assert_eq!(typing.error, None, "{input}");
        }
        let escape = sequence
            .dispatch_command(typed(r"/w\"), &[ChordEventTrigger::Input], None, |e| {
                contexts(&package, e)
            })
            .unwrap();
        assert_eq!(
            escape.error.as_deref(),
            Some("Press Tab to expand the escape")
        );
        // Typing expands every defined macro reference, so one that stays is undefined.
        for (input, error) in [
            (r"/w\=nope=", r"`\=nope=` references an undefined macro"),
            (r"/w\nope=", r"`\nope=` references an undefined macro"),
            (r"\4", r"`\4` references an undefined macro"),
        ] {
            let undefined = sequence
                .dispatch_command(typed(input), &[ChordEventTrigger::Input], None, |e| {
                    contexts(&package, e)
                })
                .unwrap();
            assert_eq!(undefined.error.as_deref(), Some(error), "{input}");
        }
        let core = core_fixture(include_str!(concat!(
            env!("OUT_DIR"),
            "/chords-chord.macos.toml"
        )));
        let meta = sequence
            .dispatch_command(event("0unknown"), &[ChordEventTrigger::Input], None, |e| {
                core.matching_global_chords(e)
                    .into_iter()
                    .map(|r| (core.clone(), r))
                    .collect()
            })
            .unwrap();
        assert_eq!(meta.error.as_deref(), Some("No chord found"));
        let resolved = run_with(&mut sequence, &package, "/w]", ChordEventTrigger::Input);
        assert!(resolved.input_event.is_some() && resolved.error.is_none());
    }

    #[test]
    fn quotes_dispatch_typing_tasks_alone_repeated_or_as_sequence_steps() {
        let package = fixture(
            r#"name = 'fixture'
[chords]
'/w]' = { name = 'Close tab', shortcut = 'cmd+w' }
"'hi'" = { name = 'Never matched', shortcut = 'cmd+h' }
"#,
        );
        let typed =
            |dispatch: &ChordSequenceDispatch, index: usize| match &dispatch.tasks[index].action {
                ChordTaskAction::Type(action) => (
                    action.text.clone(),
                    action.intervals_ms.clone(),
                    dispatch.tasks[index].num_times,
                ),
                _ => panic!("expected typing"),
            };
        let mut sequence = ChordSequence::default();

        let dispatch = run_with(&mut sequence, &package, "'hello'", ChordEventTrigger::Input);
        assert_eq!(dispatch.input_event, Some(event("'hello'")));
        assert_eq!(dispatch.description.as_deref(), Some("Type 'hello'"));
        assert_eq!(typed(&dispatch, 0), ("hello".into(), vec![], 1));
        assert!(dispatch.plan.is_none());
        // Typing never consults packages, even for a trigger spelled with quotes.
        let dispatch = sequence
            .dispatch_command(event("'hi'"), &[ChordEventTrigger::Input], None, |_| {
                panic!("quotes must not visit packages")
            })
            .unwrap();
        assert_eq!(typed(&dispatch, 0), ("hi".into(), vec![], 1));

        let dispatch = run_with(
            &mut sequence,
            &package,
            r"3'a\nb'`i50`",
            ChordEventTrigger::Input,
        );
        assert_eq!(
            dispatch.description.as_deref(),
            Some(r"Repeat 3 times · Type 'a\nb' (500ms, 0ms apart)")
        );
        // Text notation's `\n` is the Enter key, typed as a newline.
        assert_eq!(typed(&dispatch, 0), ("a\nb".into(), vec![500, 0], 3));

        let input = r"2`i1`'hi'\n/w]";
        let dispatch = run_with(&mut sequence, &package, input, ChordEventTrigger::Input);
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Repeat 2 times · Interval 100ms · Type 'hi' → Close tab")
        );
        let plan = dispatch.plan.unwrap();
        assert_eq!(
            (plan.input.as_str(), plan.repeat),
            (input, ChordRepeat::Times(2))
        );
        let ChordTaskAction::Type(first) = &plan.steps[0].action else {
            panic!("expected typing");
        };
        assert_eq!(first.text, "hi");
        assert!(matches!(plan.steps[1].action, ChordTaskAction::Shortcut(_)));

        // Typing a quote previews it without running anything.
        let typing = run_with(
            &mut sequence,
            &package,
            "'hello'",
            ChordEventTrigger::Keydown,
        );
        assert!(typing.tasks.is_empty() && typing.input_event.is_none());
        assert_eq!(typing.description.as_deref(), Some("Type 'hello'"));
    }

    #[test]
    fn insert_mode_opens_at_a_step_start_and_reopens_a_quote_with_its_text() {
        let sequence = ChordSequence::default();
        let quotes =
            |items: &[&str]| -> Vec<String> { items.iter().map(|q| (*q).into()).collect() };
        let keys = |input: &str| Key::parse_sequence(input).unwrap();
        // Typing `'` at a step start opens an empty quote.
        assert_eq!(
            sequence.edit_quotes(&keys("3'"), vec![], true).unwrap(),
            (quotes(&[""]), true)
        );
        assert_eq!(
            sequence
                .edit_quotes(&keys(r"2''\n'"), quotes(&["A", "stale"]), true)
                .unwrap(),
            (quotes(&["A", ""]), true)
        );
        // Deleting a closing quote reopens it with its text.
        assert_eq!(
            sequence
                .edit_quotes(&keys("3'"), quotes(&["Hello"]), false)
                .unwrap(),
            (quotes(&["Hello"]), true)
        );
        // Elsewhere `'` is an ordinary key, and a closed quote is not insert mode.
        for input in ["ab'", "3''", "0'"] {
            assert!(
                !sequence
                    .edit_quotes(&keys(input), quotes(&["x"]), true)
                    .unwrap()
                    .1,
                "{input}"
            );
        }
    }

    #[test]
    fn typed_quotes_dispatch_their_insert_mode_text() {
        let package = fixture(
            "name = 'fixture'\n[chords]\n'/w]' = { name = 'Close tab', shortcut = 'cmd+w' }",
        );
        let mut sequence = ChordSequence::default();
        let raw = |input: &str, quotes: &[&str]| ChordInputEvent {
            quotes: quotes.iter().map(|q| (*q).into()).collect(),
            ..event(input)
        };
        let typing = sequence
            .dispatch_command(
                raw("3'", &[""]),
                &[ChordEventTrigger::Keydown],
                None,
                |_| panic!("insert mode must not visit packages"),
            )
            .unwrap();
        assert!(typing.tasks.is_empty() && typing.input_event.is_none() && typing.error.is_none());
        assert!(typing.description.unwrap().starts_with("Insert mode"));

        let input = raw(r"2`i5`''\n/w]", &["Hello, 'World'!"]);
        let dispatch = sequence
            .dispatch_command(input.clone(), &[ChordEventTrigger::Input], None, |e| {
                contexts(&package, e)
            })
            .unwrap();
        assert_eq!(dispatch.input_event, Some(input));
        assert_eq!(
            dispatch.description.as_deref(),
            Some("Repeat 2 times · Interval 500ms · Type 'Hello, \\'World\\'!' → Close tab")
        );
        let plan = dispatch.plan.unwrap();
        assert_eq!(plan.input, r"2`i5`'Hello, 'World'!'\n/w]");
        let ChordTaskAction::Type(first) = &plan.steps[0].action else {
            panic!("expected typing");
        };
        assert_eq!(first.text, "Hello, 'World'!");
    }

    #[test]
    fn meta_dispatch_uses_only_the_protected_file_and_its_configured_bindings() {
        let source = include_str!(concat!(env!("OUT_DIR"), "/chords-chord.macos.toml"))
            .replace("'0,'", "'0x'");
        let core = core_fixture(&source);
        let other = fixture("name = 'fixture'\n[chords]\n'(.*)' = { '@input:input' = [] }");
        let mut sequence = ChordSequence::default();
        for (input, expected) in [
            ("0x", Some(MetaChordCommand::Settings)),
            ("0,", None),
            ("0,r", Some(MetaChordCommand::Reload)),
            ("0q", Some(MetaChordCommand::Quit)),
            ("0h12", Some(MetaChordCommand::Halt(12))),
            ("0h12x", None),
            ("0unknown", None),
        ] {
            let dispatch = sequence
                .dispatch_command(event(input), &[ChordEventTrigger::Input], None, |e| {
                    let mut matches = contexts(&other, e);
                    matches.extend(
                        core.matching_global_chords(e)
                            .into_iter()
                            .map(|r| (core.clone(), r)),
                    );
                    matches
                })
                .unwrap();
            assert_eq!(dispatch.meta_command, expected);
            assert_eq!(dispatch.input_event.is_some(), expected.is_some());
            assert!(dispatch.tasks.is_empty() && dispatch.plan.is_none());
        }
        let overflow = sequence.dispatch_command(
            event("0h4294967296"),
            &[ChordEventTrigger::Input],
            None,
            |e| {
                core.matching_global_chords(e)
                    .into_iter()
                    .map(|r| (core.clone(), r))
                    .collect()
            },
        );
        assert!(overflow.is_err());
    }

    #[test]
    fn core_macro_setters_expand_before_matching_captures_and_keyboard_handlers() {
        let core = core_fixture(include_str!(concat!(
            env!("OUT_DIR"),
            "/chords-chord.macos.toml"
        )));
        let package = fixture(
            r#"name = 'fixture'
[chords]
'/wd(\d+)x(\d+)' = { '@keydown:down' = ['$1', '$2'], '@input:input' = ['$1', '$2'] }
"#,
        );
        let mut sequence = ChordSequence::default();
        for digit in 1..=9 {
            save_macro(&mut sequence, &core, &format!("0b{digit}1720x864"));
        }
        save_macro(&mut sequence, &core, r"0=window-dimensions=\1");
        sequence.reset();
        // A typed named reference waits for Tab, even after its closing delimiter.
        for input in [
            r"/wd\=window-dimensions",
            r"/wd\=window-dimensions=",
            r"/wd\window-dimensions=",
        ] {
            let dispatch = sequence
                .dispatch_command(
                    typed(input),
                    &[ChordEventTrigger::Keydown, ChordEventTrigger::Input],
                    None,
                    |_| panic!("unselected named macros must not reach handlers"),
                )
                .unwrap();
            assert!(dispatch.tasks.is_empty() && dispatch.input_event.is_none());
        }
        for raw in (1..=9).map(|digit| typed(&format!(r"/wd\{digit}"))).chain([
            tab(&sequence, r"/wd\=window-d"),
            tab(&sequence, r"/wd\window-d"),
        ]) {
            let dispatch = sequence
                .dispatch_command(
                    raw.clone(),
                    &[ChordEventTrigger::Keydown, ChordEventTrigger::Input],
                    None,
                    |e| contexts(&package, e),
                )
                .unwrap();
            assert_eq!(handlers(&dispatch), ["down", "input"]);
            assert_eq!(dispatch.input_event, Some(raw));
            for task in &dispatch.tasks {
                let ChordTaskAction::Handler(action) = &task.action else {
                    panic!()
                };
                assert_eq!(action.input, "/wd1720x864");
                assert_eq!(action.event_args, ["1720".into(), "864".into()]);
            }
        }
        // Selecting expands nested references with their current values.
        save_macro(&mut sequence, &core, "0b1800x600");
        let dispatch = sequence
            .dispatch_command(
                tab(&sequence, r"2/wd\=window-dimensions="),
                &[ChordEventTrigger::Input],
                None,
                |e| contexts(&package, e),
            )
            .unwrap();
        let ChordTaskAction::Handler(action) = &dispatch.tasks[0].action else {
            panic!()
        };
        assert_eq!(action.input, "/wd800x600");
        assert_eq!(action.event_args, ["800".into(), "600".into()]);
        assert_eq!(dispatch.tasks[0].num_times, 2);

        save_macro(&mut sequence, &core, "0b212");
        save_macro(&mut sequence, &core, "0=preferences=0,");
        for (input, expected) in [
            (typed(r"0h\2"), MetaChordCommand::Halt(12)),
            (tab(&sequence, r"\=pref"), MetaChordCommand::Settings),
            (tab(&sequence, r"\pref"), MetaChordCommand::Settings),
        ] {
            let dispatch = sequence
                .dispatch_command(input, &[ChordEventTrigger::Input], None, |e| {
                    core.matching_global_chords(e)
                        .into_iter()
                        .map(|r| (core.clone(), r))
                        .collect()
                })
                .unwrap();
            assert_eq!(dispatch.meta_command, Some(expected));
        }

        // A fresh app session has no definitions and cannot dispatch literal references.
        let dispatch = ChordSequence::default()
            .dispatch_command(event(r"/wd\1"), &[ChordEventTrigger::Input], None, |_| {
                panic!("undefined macros must not reach handlers")
            })
            .unwrap();
        assert!(dispatch.input_event.is_none() && dispatch.tasks.is_empty());
    }

    #[test]
    fn macro_setters_wait_for_completion_and_reject_invalid_targets() {
        let core = core_fixture(include_str!(concat!(
            env!("OUT_DIR"),
            "/chords-chord.macos.toml"
        )));
        let mut sequence = ChordSequence::default();
        let typing = sequence
            .dispatch_command(event("0b1ab"), &[ChordEventTrigger::Keydown], None, |_| {
                panic!("typing a setter must not visit handlers")
            })
            .unwrap();
        assert!(typing.meta_command.is_none() && typing.input_event.is_none());
        for input in ["0b0ab", "0b", "0=", "0==ab"] {
            let dispatch = sequence
                .dispatch_command(event(input), &[ChordEventTrigger::Input], None, |e| {
                    core.matching_global_chords(e)
                        .into_iter()
                        .map(|r| (core.clone(), r))
                        .collect()
                })
                .unwrap();
            assert!(
                dispatch.meta_command.is_none() && dispatch.input_event.is_none(),
                "{input}"
            );
        }
        save_macro(&mut sequence, &core, "0b1");
        save_macro(&mut sequence, &core, "0=empty=");
        assert_eq!(
            sequence.macros.expand(r"a\1\=empty=b").unwrap().as_deref(),
            Some("ab")
        );

        // Setters follow core TOML bindings even when their prefixes change.
        let source = include_str!(concat!(env!("OUT_DIR"), "/chords-chord.macos.toml"))
            .replace("'0b(", "'0z(")
            .replace("'0=(", "'0m(");
        let remapped = core_fixture(&source);
        save_macro(&mut sequence, &remapped, "0z2ab");
        save_macro(&mut sequence, &remapped, r"0mmy.name=\2");
        assert_eq!(
            sequence.macros.expand(r"\=my.name=").unwrap().as_deref(),
            Some("ab")
        );
    }

    #[tokio::test]
    async fn async_callbacks_cannot_overtake_init_or_earlier_events() {
        let queue = ChordTaskQueue::default();
        let calls = Arc::new(Mutex::new(Vec::new()));
        let (started, ready) = oneshot::channel();
        let (release, gate) = oneshot::channel();
        let first_calls = calls.clone();
        let first = queue.enqueue(async move {
            first_calls.lock().push("init-start");
            started.send(()).unwrap();
            gate.await.unwrap();
            first_calls.lock().push("init-end");
        });
        let second_calls = calls.clone();
        let second = queue.enqueue(async move {
            second_calls.lock().push("keydown");
        });
        ready.await.unwrap();
        assert_eq!(*calls.lock(), ["init-start"]);
        release.send(()).unwrap();
        first.await.unwrap();
        second.await.unwrap();
        assert_eq!(*calls.lock(), ["init-start", "init-end", "keydown"]);
    }
}
