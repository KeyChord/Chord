use crate::app::AppHandleExt;
use crate::git::GitHubRepoRef;
use crate::tauri_app::play_failure_sound;
use anyhow::{Context, Result, bail};
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static PENDING_CHORDS: Mutex<PendingChords> = Mutex::new(PendingChords {
    ready: false,
    sequences: Vec::new(),
    packages: Vec::new(),
});

#[derive(Default)]
struct PendingChords {
    ready: bool,
    sequences: Vec<String>,
    packages: Vec<GitHubRepoRef>,
}

impl PendingChords {
    fn add_package(&mut self, repo: GitHubRepoRef) -> Option<GitHubRepoRef> {
        if self.ready {
            return Some(repo);
        }
        if !self.packages.contains(&repo) {
            self.packages.push(repo);
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScriptCommand {
    OpenSettings,
    ReloadConfigs,
    AddPackage(GitHubRepoRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAppCommand {
    Chord(String),
}

pub fn init(handle: AppHandle) {
    let _ = APP_HANDLE.set(handle);

    #[cfg(target_os = "macos")]
    macos::init_url_handler();
}

pub fn handle_url(url: &str) -> Result<()> {
    let command = parse_command(url)?;
    let handle = APP_HANDLE
        .get()
        .cloned()
        .context("app handle is not initialized")?;

    match command {
        ScriptCommand::OpenSettings => {
            let settings = handle.app_state().settings();
            settings.ui.open()?
        }
        ScriptCommand::ReloadConfigs => reload_configs(handle),
        ScriptCommand::AddPackage(repo) => {
            let window = handle.app_state().settings().ui.open_chords()?;
            handle
                .dialog()
                .message(format!("Add the package from {} to Chord?", repo.slug()))
                .title("Add to Chord")
                .parent(&window)
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Add Chords".into(),
                    "Cancel".into(),
                ))
                .show(move |confirmed| {
                    if confirmed {
                        match PENDING_CHORDS
                            .lock()
                            .map(|mut pending| pending.add_package(repo))
                        {
                            Ok(Some(repo)) => install_package(handle, repo),
                            Ok(None) => {}
                            Err(_) => log::error!("Pending chord queue is poisoned"),
                        }
                    }
                });
        }
    }

    Ok(())
}

pub fn handle_cli_app_command(handle: &AppHandle, command: CliAppCommand) -> Result<()> {
    match command {
        CliAppCommand::Chord(sequence) => execute_or_queue_chord(handle, sequence),
    }
}

pub fn cli_app_command_from_args(args: &[String]) -> Option<CliAppCommand> {
    match args.get(1..)? {
        [sequence] if !sequence.starts_with('-') => Some(CliAppCommand::Chord(sequence.clone())),
        [command, sequence] if command == "chord" || command == "__app-chord" => {
            Some(CliAppCommand::Chord(sequence.clone()))
        }
        _ => None,
    }
}

fn execute_or_queue_chord(handle: &AppHandle, sequence: String) -> Result<()> {
    {
        let mut pending = PENDING_CHORDS
            .lock()
            .map_err(|_| anyhow::anyhow!("pending chord queue is poisoned"))?;
        if !pending.ready {
            pending.sequences.push(sequence);
            return Ok(());
        }
    }

    handle
        .app_state()
        .chord_mode_manager()
        .execute_sequence(&sequence)
}

pub fn mark_chord_packages_ready(handle: &AppHandle) {
    let (sequences, packages) = {
        let Ok(mut pending) = PENDING_CHORDS.lock() else {
            log::error!("Pending chord queue is poisoned");
            return;
        };
        pending.ready = true;
        (
            std::mem::take(&mut pending.sequences),
            std::mem::take(&mut pending.packages),
        )
    };

    for sequence in sequences {
        if let Err(error) = handle
            .app_state()
            .chord_mode_manager()
            .execute_sequence(&sequence)
        {
            log::error!("Failed to execute queued CLI chord `{sequence}`: {error:#}");
            play_failure_sound(handle);
        }
    }
    for repo in packages {
        install_package(handle.clone(), repo);
    }
}

fn install_package(handle: AppHandle, repo: GitHubRepoRef) {
    tauri::async_runtime::spawn(async move {
        let install_handle = handle.clone();
        let slug = repo.slug();
        let result = async {
            // Git downloads must not block the native URL-event/main thread or Tokio workers.
            tauri::async_runtime::spawn_blocking(move || {
                install_handle
                    .app_state()
                    .chord_package_manager()
                    .registry
                    .git
                    .git_repos_store
                    .add_repo(repo)
            })
            .await??;
            handle
                .app_state()
                .chord_package_manager()
                .reload_all()
                .await
        }
        .await;
        if let Err(error) = result {
            log::error!("Failed to add {slug} from a deep link: {error:#}");
            handle
                .dialog()
                .message(format!("Could not add {slug} to Chord.\n\n{error:#}"))
                .title("Add to Chord")
                .kind(MessageDialogKind::Error)
                .show(|_| {});
        }
    });
}

pub fn reload_configs(handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let chord_pm = handle.app_state().chord_package_manager();
        if let Err(e) = chord_pm.reload_all().await {
            log::error!("Failed to reload packages: {}", e);
        }
    });
}

fn parse_command(url: &str) -> Result<ScriptCommand> {
    let command = normalize_command(url)?;

    match command.as_str() {
        "ui" | "settings" | "open-settings" | "show-settings" => Ok(ScriptCommand::OpenSettings),
        "reload-config" | "reload-configs" => Ok(ScriptCommand::ReloadConfigs),
        "add-package" => {
            let parsed = tauri::Url::parse(url).context("Invalid Add to Chord URL")?;
            let mut repos = parsed.query_pairs().filter(|(key, _)| key == "repo");
            let (_, repo) = repos
                .next()
                .context("Add to Chord requires a repo parameter")?;
            anyhow::ensure!(repos.next().is_none(), "Specify exactly one repo parameter");
            let mut requested = GitHubRepoRef::parse(&repo)?;
            let name = requested.name.to_ascii_lowercase();
            // The website discovers mirrors as they are published. Keep the owner and mirror
            // naming rule local, and confirm in the app before a link can install executable code.
            anyhow::ensure!(
                requested.owner.eq_ignore_ascii_case("KeyChord")
                    && name.strip_prefix("chords-").is_some_and(|suffix| {
                        suffix.starts_with(|c: char| c.is_ascii_alphanumeric())
                    }),
                "Add to Chord links only support KeyChord package mirrors; use Add Repo in Settings for other repositories"
            );
            requested.owner = "KeyChord".into();
            requested.name = name;
            Ok(ScriptCommand::AddPackage(requested))
        }
        _ => bail!("Unsupported chord URL command: {command}"),
    }
}

fn normalize_command(url: &str) -> Result<String> {
    let remainder = url
        .strip_prefix("chord:")
        .context("URL must start with chord:")?;

    let remainder = remainder.strip_prefix("//").unwrap_or(remainder);
    let command = remainder
        .split_once('?')
        .map(|(command, _)| command)
        .unwrap_or(remainder)
        .trim_matches('/');

    if command.is_empty() {
        bail!("Missing chord URL command");
    }

    Ok(command.to_ascii_lowercase())
}

#[cfg(target_os = "macos")]
mod macos {
    use super::handle_url;
    use objc2::declare::ClassBuilder;
    use objc2::runtime::{AnyClass, AnyObject, NSObject, Sel};
    use objc2::{ClassType, msg_send, sel};
    use objc2_foundation::NSString;
    use std::sync::OnceLock;

    const INTERNET_EVENT_CLASS: u32 = u32::from_be_bytes(*b"GURL");
    const GET_URL_EVENT_ID: u32 = u32::from_be_bytes(*b"GURL");
    const KEY_DIRECT_OBJECT: u32 = u32::from_be_bytes(*b"----");

    static URL_HANDLER_INIT: OnceLock<()> = OnceLock::new();
    static URL_HANDLER_INSTANCE: OnceLock<usize> = OnceLock::new();

    pub fn init_url_handler() {
        if URL_HANDLER_INIT.set(()).is_err() {
            return;
        }

        let mut builder = ClassBuilder::new(c"ChordUrlEventHandler", NSObject::class())
            .expect("a class with name ChordUrlEventHandler likely already exists");

        unsafe extern "C" fn init(this: *mut NSObject, _sel: Sel) -> *mut NSObject {
            unsafe { msg_send![super(this, NSObject::class()), init] }
        }

        unsafe extern "C" fn handle_get_url_event(
            _this: *mut NSObject,
            _sel: Sel,
            event: *mut AnyObject,
            _reply_event: *mut AnyObject,
        ) {
            let Some(url) = apple_event_url(event) else {
                log::warn!("Ignoring chord URL event without a URL payload");
                return;
            };

            if let Err(error) = handle_url(&url) {
                log::error!("Failed to handle chord URL {url:?}: {error:#}");
            }
        }

        unsafe {
            builder.add_method(
                sel!(init),
                init as unsafe extern "C" fn(*mut NSObject, Sel) -> *mut NSObject,
            );
            builder.add_method(
                sel!(handleGetURLEvent:withReplyEvent:),
                handle_get_url_event
                    as unsafe extern "C" fn(*mut NSObject, Sel, *mut AnyObject, *mut AnyObject),
            );
        }

        let handler_class = builder.register();

        unsafe {
            let handler: *mut NSObject = msg_send![handler_class, alloc];
            let handler: *mut NSObject = msg_send![handler, init];
            let manager_class = AnyClass::get(c"NSAppleEventManager")
                .expect("NSAppleEventManager should exist on macOS");
            let manager: *mut AnyObject = msg_send![manager_class, sharedAppleEventManager];

            let _: () = msg_send![
                manager,
                setEventHandler: &*handler,
                andSelector: sel!(handleGetURLEvent:withReplyEvent:),
                forEventClass: INTERNET_EVENT_CLASS,
                andEventID: GET_URL_EVENT_ID
            ];

            let _ = URL_HANDLER_INSTANCE.set(handler as usize);
        }
    }

    fn apple_event_url(event: *mut AnyObject) -> Option<String> {
        unsafe {
            let descriptor: *mut AnyObject =
                msg_send![event, paramDescriptorForKeyword: KEY_DIRECT_OBJECT];
            let descriptor = descriptor.as_ref()?;
            let value: *mut NSString = msg_send![descriptor, stringValue];
            value.as_ref().map(NSString::to_string)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CliAppCommand, ScriptCommand, cli_app_command_from_args, normalize_command, parse_command,
    };

    #[test]
    fn add_to_chord_links_accept_new_mirrors_without_a_catalog_update() {
        let expected = ScriptCommand::AddPackage(
            crate::git::GitHubRepoRef::parse("KeyChord/chords-new.package").unwrap(),
        );
        for url in [
            "chord://add-package?repo=KeyChord%2Fchords-new.package",
            "chord://add-package?repo=keychord%2FCHORDS-NEW.PACKAGE",
            "chord://add-package?repo=https%3A%2F%2Fgithub.com%2FKeyChord%2Fchords-new.package",
        ] {
            assert_eq!(parse_command(url).unwrap(), expected);
        }
    }

    #[test]
    fn add_to_chord_rejects_missing_ambiguous_and_untrusted_sources() {
        for url in [
            "chord://add-package",
            "chord://add-package?repo=",
            "chord://add-package?repo=someone%2Fchords-vscode",
            "chord://add-package?repo=KeyChord%2FChord",
            "chord://add-package?repo=KeyChord%2Fchords-",
            "chord://add-package?repo=KeyChord%2F..",
            "chord://add-package?repo=..%2Fchords-vscode",
            "chord://add-package?repo=https%3A%2F%2Fevil.example%2FKeyChord%2Fchords-vscode",
            "chord://add-package?repo=KeyChord%2Fchords-vscode&repo=KeyChord%2Fchords-terminal",
        ] {
            assert!(parse_command(url).is_err(), "{url}");
        }
    }

    #[test]
    fn cold_start_links_wait_for_packages_and_duplicate_requests_are_coalesced() {
        let mut pending = super::PendingChords::default();
        let repo = crate::git::GitHubRepoRef::parse("KeyChord/chords-vscode").unwrap();
        assert!(pending.add_package(repo.clone()).is_none());
        assert!(pending.add_package(repo.clone()).is_none());
        assert_eq!(pending.packages, vec![repo.clone()]);
        pending.ready = true;
        assert_eq!(pending.add_package(repo.clone()), Some(repo));
    }

    #[test]
    fn parses_direct_scheme_command() {
        assert_eq!(
            parse_command("chord:reload-config").unwrap(),
            ScriptCommand::ReloadConfigs
        );
    }

    #[test]
    fn parses_host_style_command() {
        assert_eq!(
            parse_command("chord://settings").unwrap(),
            ScriptCommand::OpenSettings
        );
    }

    #[test]
    fn parses_ui_command() {
        assert_eq!(
            parse_command("chord:ui").unwrap(),
            ScriptCommand::OpenSettings
        );
    }

    #[test]
    fn strips_query_parameters() {
        assert_eq!(
            normalize_command("chord://reload-configs?source=raycast").unwrap(),
            "reload-configs"
        );
    }

    #[test]
    fn parses_explicit_cli_chord() {
        assert_eq!(
            cli_app_command_from_args(&["/Applications/Chord".into(), "chord".into(), "fq".into()]),
            Some(CliAppCommand::Chord("fq".into()))
        );
    }

    #[test]
    fn parses_internal_detached_chord() {
        assert_eq!(
            cli_app_command_from_args(&[
                "/Applications/Chord".into(),
                "__app-chord".into(),
                "fq".into()
            ]),
            Some(CliAppCommand::Chord("fq".into()))
        );
    }

    #[test]
    fn parses_shorthand_cli_chord() {
        assert_eq!(
            cli_app_command_from_args(&["/Applications/Chord".into(), "fq".into()]),
            Some(CliAppCommand::Chord("fq".into()))
        );
    }

    #[test]
    fn ignores_non_chord_instance_arguments() {
        assert_eq!(cli_app_command_from_args(&["chord".into()]), None);
        assert_eq!(
            cli_app_command_from_args(&["chord".into(), "--autostart".into()]),
            None
        );
    }
}
