use crate::error::InterspireError;
use std::{
    env, fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) const RENDER_OUTPUT_DIR_ENV: &str = "INTERSPIRE_RENDER_ARTIFACT_OUTPUT_DIR";
pub(crate) const RENDER_OUTPUT_ROOTS_ENV: &str = "INTERSPIRE_RENDER_ARTIFACT_ROOTS";

pub(crate) fn safe_render_output_dir(raw: Option<&str>) -> Result<PathBuf, InterspireError> {
    if raw.is_some_and(|value| !value.trim().is_empty()) {
        return Err(InterspireError::Safety(format!(
            "render artifact output_dir request values are disabled; configure {RENDER_OUTPUT_DIR_ENV} under {RENDER_OUTPUT_ROOTS_ENV}"
        )));
    }
    safe_output_dir_from_env(
        RENDER_OUTPUT_DIR_ENV,
        RENDER_OUTPUT_ROOTS_ENV,
        "render artifact",
    )
}

pub(crate) fn prepare_private_output_dir(path: &Path, label: &str) -> Result<(), InterspireError> {
    fs::create_dir_all(path).map_err(|err| {
        InterspireError::Io(format!("failed to create private {label} directory: {err}"))
    })?;
    set_private_dir_permissions(path)?;
    ensure_output_dir_still_approved(path, RENDER_OUTPUT_ROOTS_ENV, label)
}

pub(crate) fn create_private_file(path: &Path, label: &str) -> Result<fs::File, InterspireError> {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|err| {
            InterspireError::Io(format!(
                "failed to create private {} artifact: {err}",
                label
            ))
        })
}

pub(crate) fn set_private_file_permissions(path: &Path) -> Result<(), InterspireError> {
    let mut perms = fs::metadata(path)
        .map_err(|err| InterspireError::Io(format!("failed to stat private artifact: {err}")))?
        .permissions();
    perms.set_mode(0o600);
    fs::set_permissions(path, perms)
        .map_err(|err| InterspireError::Io(format!("failed to set artifact permissions: {err}")))
}

pub(crate) fn set_private_dir_permissions(path: &Path) -> Result<(), InterspireError> {
    let mut perms = fs::metadata(path)
        .map_err(|err| InterspireError::Io(format!("failed to stat private directory: {err}")))?
        .permissions();
    perms.set_mode(0o700);
    fs::set_permissions(path, perms)
        .map_err(|err| InterspireError::Io(format!("failed to set directory permissions: {err}")))
}

pub(crate) fn safe_prefix(raw: Option<&str>, default: &str) -> String {
    let raw = raw.unwrap_or(default);
    let mut out = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    let out = out.trim_matches('-');
    if out.is_empty() {
        default.to_string()
    } else {
        out.chars().take(80).collect()
    }
}

pub(crate) fn unix_timestamp_nanos() -> Result<u128, InterspireError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|err| InterspireError::Io(format!("system time before unix epoch: {err}")))
}

fn safe_output_dir_from_env(
    output_dir_env: &str,
    output_roots_env: &str,
    label: &str,
) -> Result<PathBuf, InterspireError> {
    let raw_path = env::var(output_dir_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            InterspireError::Safety(format!(
                "{output_dir_env} must be set to an approved private {label} output directory"
            ))
        })?;
    let path = PathBuf::from(&raw_path);
    if !path.is_absolute() {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir must be absolute"
        )));
    }
    if raw_path_has_dot_component(&raw_path)
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir must not contain dot path components"
        )));
    }
    let repo_root = canonical_path(Path::new(env!("CARGO_MANIFEST_DIR")))?;
    let allowed_roots = approved_output_roots(output_roots_env)?;
    if allowed_roots.contains(&path) {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir must be a subdirectory, not an allowed root"
        )));
    }
    if !allowed_roots.iter().any(|root| path.starts_with(root)) {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir must be under one of the private roots listed in {output_roots_env}"
        )));
    }

    if let Ok(canonical_target) = path.canonicalize() {
        if allowed_roots.contains(&canonical_target) {
            return Err(InterspireError::Safety(format!(
                "{label} output_dir must be a subdirectory, not an allowed root"
            )));
        }
        if canonical_target.starts_with(&repo_root) {
            return Err(InterspireError::Safety(format!(
                "{label} artifacts must be outside the repository"
            )));
        }
        if !allowed_roots
            .iter()
            .any(|root| canonical_target.starts_with(root))
        {
            return Err(InterspireError::Safety(format!(
                "{label} output_dir resolved outside the approved private artifact roots"
            )));
        }
    }

    let existing_ancestor = nearest_existing_ancestor(&path, label)?;
    let canonical_ancestor = canonical_path(&existing_ancestor)?;
    if canonical_ancestor.starts_with(&repo_root) {
        return Err(InterspireError::Safety(format!(
            "{label} artifacts must be outside the repository"
        )));
    }
    if !allowed_roots
        .iter()
        .any(|root| canonical_ancestor.starts_with(root))
    {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir resolved outside the approved private artifact roots"
        )));
    }

    Ok(path)
}

fn ensure_output_dir_still_approved(
    path: &Path,
    output_roots_env: &str,
    label: &str,
) -> Result<(), InterspireError> {
    let metadata = fs::symlink_metadata(path).map_err(|err| {
        InterspireError::Io(format!("failed to stat private {label} directory: {err}"))
    })?;
    if metadata.file_type().is_symlink() {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir must not be a symlink"
        )));
    }

    let repo_root = canonical_path(Path::new(env!("CARGO_MANIFEST_DIR")))?;
    let allowed_roots = approved_output_roots(output_roots_env)?;
    let canonical_target = canonical_path(path)?;
    if allowed_roots.contains(&canonical_target) {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir must be a subdirectory, not an allowed root"
        )));
    }
    if canonical_target.starts_with(&repo_root) {
        return Err(InterspireError::Safety(format!(
            "{label} artifacts must be outside the repository"
        )));
    }
    if !allowed_roots
        .iter()
        .any(|root| canonical_target.starts_with(root))
    {
        return Err(InterspireError::Safety(format!(
            "{label} output_dir resolved outside the approved private artifact roots"
        )));
    }
    Ok(())
}

fn approved_output_roots(output_roots_env: &str) -> Result<Vec<PathBuf>, InterspireError> {
    let roots = env_output_roots(output_roots_env)?;
    #[cfg(test)]
    {
        let mut roots = roots;
        roots.push(canonical_path(&std::env::temp_dir())?);
        Ok(roots)
    }
    #[cfg(not(test))]
    {
        if roots.is_empty() {
            return Err(InterspireError::Safety(format!(
                "{output_roots_env} must list at least one existing private absolute artifact root"
            )));
        }
        Ok(roots)
    }
}

fn env_output_roots(output_roots_env: &str) -> Result<Vec<PathBuf>, InterspireError> {
    let Some(raw) = env::var(output_roots_env)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(Vec::new());
    };

    raw.split(':')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(InterspireError::Safety(format!(
                    "{output_roots_env} entries must be absolute paths"
                )));
            }
            if raw_path_has_dot_component(value)
                || path
                    .components()
                    .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
            {
                return Err(InterspireError::Safety(format!(
                    "{output_roots_env} entries must not contain dot path components"
                )));
            }
            canonical_path(&path)
        })
        .collect()
}

fn nearest_existing_ancestor(path: &Path, label: &str) -> Result<PathBuf, InterspireError> {
    let mut current = path;
    while !current.exists() {
        current = current.parent().ok_or_else(|| {
            InterspireError::Safety(format!(
                "{} output_dir has no existing parent directory",
                label
            ))
        })?;
    }
    Ok(current.to_path_buf())
}

fn raw_path_has_dot_component(raw: &str) -> bool {
    raw.split('/')
        .any(|component| matches!(component, "." | ".."))
}

fn canonical_path(path: &Path) -> Result<PathBuf, InterspireError> {
    path.canonicalize()
        .map_err(|err| InterspireError::Safety(format!("failed to canonicalize path: {err}")))
}
