use super::{Error, Request, Result};
use std::{
    collections::HashSet,
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

pub(super) fn normalize(request: &Request) -> Result<Request> {
    if request.sources.is_empty() {
        return Err(Error::Invalid("Add at least one source folder.".into()));
    }
    let destination = &request.destination;
    let name = destination
        .file_name()
        .ok_or_else(|| Error::Invalid("Choose a name for the new destination folder.".into()))?;
    if !matches!(
        destination.components().next_back(),
        Some(Component::Normal(_))
    ) {
        return Err(Error::Invalid(
            "Destination must be a new, named folder.".into(),
        ));
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent =
        fs::canonicalize(parent).map_err(|e| Error::io("open destination parent", parent, e))?;
    if !parent.is_dir() {
        return Err(Error::Invalid(
            "Destination parent must be a folder.".into(),
        ));
    }
    let destination = parent.join(name);
    require_absent(&destination)?;
    let mut sources: Vec<PathBuf> = Vec::new();
    for path in &request.sources {
        let source =
            fs::canonicalize(path).map_err(|e| Error::io("open source folder", path, e))?;
        if !source.is_dir() {
            return Err(Error::Invalid(format!(
                "Source is not a folder: {}",
                path.display()
            )));
        }
        if path_key(&destination).starts_with(path_key(&source))
            || path_key(&source).starts_with(path_key(&destination))
        {
            return Err(Error::Invalid(
                "Destination must be outside every source folder.".into(),
            ));
        }
        if !sources
            .iter()
            .any(|existing| path_key(existing) == path_key(&source))
        {
            sources.push(source);
        }
    }
    Ok(Request {
        sources,
        destination,
    })
}

pub(super) fn require_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(Error::Invalid(format!(
            "Destination already exists. Choose a new folder: {}",
            path.display()
        ))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io("inspect destination", path, e)),
    }
}

// Match the host filesystem's usual path semantics, retaining the original spelling in output.
pub(super) fn path_key(path: &Path) -> PathBuf {
    #[cfg(any(windows, target_os = "macos"))]
    {
        path.to_str()
            .map(|value| PathBuf::from(value.to_lowercase()))
            .unwrap_or_else(|| PathBuf::from(path.as_os_str().to_ascii_lowercase()))
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        path.to_owned()
    }
}

pub(super) fn conflict_path(
    relative: &Path,
    hash: blake3::Hash,
    reserved: &HashSet<PathBuf>,
) -> PathBuf {
    let digest = hash.to_hex();
    let mut number = 1;
    loop {
        let mut name = OsString::from(relative.file_stem().unwrap_or_default());
        name.push(format!(" (conflict {}", &digest[..12]));
        if number > 1 {
            name.push(format!("-{number}"));
        }
        name.push(")");
        if let Some(extension) = relative.extension() {
            name.push(".");
            name.push(extension);
        }
        let candidate = relative.with_file_name(name);
        if !reserved.contains(&path_key(&candidate)) {
            return candidate;
        }
        number += 1;
    }
}
