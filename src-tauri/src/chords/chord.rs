use crate::chords::shortcut::{press_shortcut, release_shortcut, Shortcut};
use crate::chords::{AppChordsFile, AppChordsFileConfig, ChordFolder};
use crate::input::Key;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;
use mlua::Lua;

#[derive(Debug, Clone)]
pub struct Chord {
    pub keys: Vec<Key>,
    pub name: String,
    pub command: Option<String>,
    pub shortcut: Option<Shortcut>,
    pub shell: Option<String>,
}

pub struct LoadedAppChords {
    pub global_runtime: ChordRuntime,
    pub app_runtime_map: HashMap<String, ChordRuntime>,
}

pub struct ChordRuntime {
    pub chords: HashMap<Vec<Key>, Chord>,
    pub lua: Option<Lua>
}

impl ChordRuntime {

    pub fn from_chords_no_lua(chords: HashMap<Vec<Key>, Chord>) -> Self {
        Self {
            chords,
            lua: None,
        }
    }

    // Doesn't resolve _config.extends
    pub fn from_file_shallow(chord_file: AppChordsFile) -> Result<Self> {
        let mut chords = chord_file.get_chords_shallow()?;
        // Filters out global chords
        chords.retain(|sequence, _| {
            sequence
                .first()
                .is_some_and(|c| c.is_digit() || c.is_letter())
        });;

        let lua = if let Some(AppChordsFileConfig { lua: Some(lua_config), .. }) = chord_file.config {
            let lua = Lua::new();
            if let Some(init_script) = &lua_config.init {
                lua.load(init_script).exec()?;
            }
            Some(lua)
        } else {
            None
        };

        Ok(Self {
            chords,
            lua
        })
    }

    pub fn merge_chords_from(&mut self, base: &Self) {
        // TODO: implement
    }
}

fn application_id_from_chords_path(file_path: &Path) -> Option<String> {
    let application_path = file_path.parent()?;
    if application_path.as_os_str().is_empty() {
        return None;
    }

    Some(
        application_path
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("."),
    )
}

impl LoadedAppChords {
    pub fn from_folder(chord_folder: ChordFolder) -> Result<Self> {
        let mut global_chords = HashMap::new();
        let mut app_chord_runtime_map = HashMap::new();

        for (file_path, file) in chord_folder.files_map {
            let Some(application_id) = application_id_from_chords_path(Path::new(&file_path)) else {
                log::error!("Invalid application ID for file {:?}", file_path);
                continue;
            };

            // Loading global chords into `global_chords`
            let chords = file.get_chords_shallow()?;
            for (sequence, chord) in &chords {
                if sequence.first().is_some_and(|c| !c.is_digit() && !c.is_letter()) {
                    global_chords.insert(sequence.clone(), chord.clone());
                }
            }

            let app_chord_runtime = ChordRuntime::from_file_shallow(file)?;
            app_chord_runtime_map.insert(application_id.clone(), (app_chord_runtime, file.config.clone()));
        }

        // Loop through each config and resolve _extends
        for (_, (mut app_chord_runtime, config)) in app_chord_runtime_map.iter_mut() {
            if let Some(AppChordsFileConfig { extends: Some(extends), .. }) = &config {
                if let Some(base_runtime) = app_chord_runtime_map.get(extends).map(|(r, _)| r) {
                    app_chord_runtime.merge_chords_from(base_runtime);
                } else {
                    log::warn!("Invalid extends for application ID: {extends}");
                }
            }
        }

        // let mut resolved_chords = HashMap::new();
        // for application_id in app_chord_runtime_map.keys() {
        //     resolve_app_chords(
        //         application_id,
        //         &app_chords_files,
        //         &direct_app_chords,
        //         &mut resolved_chords,
        //         &mut HashSet::new(),
        //     );
        // }

        Ok(LoadedAppChords {
            global_runtime: ChordRuntime::from_chords_no_lua(global_chords),
            app_runtime_map: app_chord_runtime_map,
        })
    }

    // No application = global chord
    pub fn get_chord_runtime(&self, sequence: &[Key], application_id: Option<String>) -> Option<&ChordRuntime> {
        // Prefer app chord, fall back to global
        let chord_runtime = if let Some(app_id) = application_id {
            self.app_runtime_map
                .get(&app_id).unwrap_or(&self.global_runtime)
        } else {
            &self.global_runtime
        };

        if chord_runtime.chords.contains_key(sequence) {
            Some(chord_runtime)
        } else {
            None
        }
    }
}

pub fn press_chord(handle: AppHandle, chord: &Chord) -> anyhow::Result<()> {
    let shortcut = chord.shortcut.clone();
    let shell = chord.shell.clone();
    handle.clone().run_on_main_thread(move || {
        if let Some(shell) = shell {
            std::thread::spawn(move || {
                let mut command = Command::new("sh");
                command.arg("-c").arg(&shell);
                log::debug!("Running shell command: {:?}", command);

                match command.output() {
                    Ok(output) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        let exit_code = output.status.code();

                        if output.status.success() {
                            log::debug!(
                                "shell command succeeded with exit code {:?}: {}",
                                exit_code,
                                shell
                            );
                        } else {
                            log::error!(
                                "shell command failed with exit code {:?}: {}",
                                exit_code,
                                shell
                            );
                        }

                        if !stdout.is_empty() {
                            log::debug!("shell stdout: {stdout}");
                        }

                        if !stderr.is_empty() {
                            log::debug!("shell stderr: {stderr}");
                        }
                    }
                    Err(e) => {
                        log::error!("failed to run shell command `{shell}`: {e}");
                    }
                }
            });
        } else {
            if let Some(shortcut) = shortcut {
                if let Err(e) = press_shortcut(shortcut.clone()) {
                    log::error!("failed to press shortcut: {e}");
                }
            } else {
                log::error!("no shortcut to execute");
            }
        }
    })?;

    Ok(())
}

pub fn release_chord(handle: AppHandle, chord: &Chord) -> anyhow::Result<()> {
    let shortcut = chord.shortcut.clone();
    handle.clone().run_on_main_thread(move || {
        if let Some(shortcut) = shortcut {
            if let Err(e) = release_shortcut(shortcut.clone()) {
                log::error!("failed to release shortcut: {e}");
            }
        }
    })?;

    Ok(())
}
