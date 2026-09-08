use crate::api::{ApiImpl, AppResult};
use crate::app::AppHandleExt;
use crate::app::chord_package_manager::import_monorepo_packages;

pub async fn list_monorepo_packages(
    api: ApiImpl,
    source: String,
) -> AppResult<Vec<(String, bool)>> {
    let handle = api.handle()?;
    let registry = &handle.app_state().chord_package_manager().registry.local;
    let root = registry.monorepo_root(&source)?;
    let selections = registry.monorepo_selections()?;
    let selected = selections
        .get(&source)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut packages: Vec<_> = import_monorepo_packages(&root)?
        .into_keys()
        .map(|name| {
            let enabled = selected.contains(&name);
            (name, enabled)
        })
        .collect();
    packages.sort();
    Ok(packages)
}
