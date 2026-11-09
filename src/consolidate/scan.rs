use super::{
    Error, Phase, Plan, PlannedFile, Progress, Request, Result, ScannedFile, check_cancel, paths,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use walkdir::WalkDir;

pub fn analyze(
    request: &Request,
    cancel: &AtomicBool,
    mut report: impl FnMut(Progress),
) -> Result<Plan> {
    check_cancel(cancel)?;
    let request = paths::normalize(request)?;
    let mut directories = BTreeMap::new();
    let mut scanned = Vec::new();
    let mut warnings = Vec::new();
    let mut total_bytes = 0;
    for root in &request.sources {
        for entry in WalkDir::new(root)
            .follow_links(false)
            .sort_by_file_name()
            .min_depth(1)
        {
            check_cancel(cancel)?;
            let entry = entry
                .map_err(|e| Error::Invalid(format!("Cannot scan {}: {e}", root.display())))?;
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("walked entry is inside root")
                .to_owned();
            let kind = entry.file_type();
            if kind.is_symlink() || (!kind.is_file() && !kind.is_dir()) {
                warnings.push(format!("Skipped link or special file: {}", path.display()));
                continue;
            }
            if kind.is_dir() {
                directories
                    .entry(paths::path_key(&relative))
                    .or_insert(relative);
                continue;
            }
            report(Progress {
                phase: Phase::Scanning,
                files: scanned.len(),
                bytes: total_bytes,
                current_path: relative.clone(),
            });
            let (hash, bytes) = hash_file(path, cancel)?;
            total_bytes += bytes;
            scanned.push(ScannedFile {
                source: path.to_owned(),
                relative,
                bytes,
                hash,
            });
        }
    }
    let mut plan = build_plan(
        request,
        directories.into_values().collect(),
        scanned,
        warnings,
    );
    // Stable output order makes the preview and copy sequence easy to follow.
    plan.files.sort_by(|a, b| a.output_path.cmp(&b.output_path));
    report(Progress {
        phase: Phase::Scanning,
        files: plan.input_files(),
        bytes: total_bytes,
        current_path: PathBuf::new(),
    });
    Ok(plan)
}

fn build_plan(
    request: Request,
    directories: Vec<PathBuf>,
    scanned: Vec<ScannedFile>,
    warnings: Vec<String>,
) -> Plan {
    let directory_keys: HashSet<_> = directories
        .iter()
        .map(|path| paths::path_key(path))
        .collect();
    // Reserve every original name before generating conflict names. A generated name
    // must never steal the path of another input that has not been visited yet.
    let mut reserved = directory_keys.clone();
    reserved.extend(scanned.iter().map(|file| paths::path_key(&file.relative)));
    let mut seen: HashMap<PathBuf, HashSet<blake3::Hash>> = HashMap::new();
    let mut files = Vec::new();
    let mut duplicates = 0;
    for file in &scanned {
        let key = paths::path_key(&file.relative);
        let versions = seen.entry(key.clone()).or_default();
        if versions.contains(&file.hash) {
            duplicates += 1;
            continue;
        }
        let output_path = if versions.is_empty() && !directory_keys.contains(&key) {
            file.relative.clone()
        } else {
            paths::conflict_path(&file.relative, file.hash, &reserved)
        };
        reserved.insert(paths::path_key(&output_path));
        versions.insert(file.hash);
        files.push(PlannedFile {
            source: file.source.clone(),
            original_path: file.relative.clone(),
            output_path,
            bytes: file.bytes,
            hash: file.hash,
        });
    }
    let total_bytes = files.iter().map(|file| file.bytes).sum();
    let renamed_files = files.iter().filter(|file| file.renamed()).count();
    Plan {
        request,
        files,
        directories,
        duplicates,
        warnings,
        total_bytes,
        renamed_files,
        scanned,
    }
}

pub(super) fn hash_file(path: &Path, cancel: &AtomicBool) -> Result<(blake3::Hash, u64)> {
    let mut file = File::open(path).map_err(|e| Error::io("read file", path, e))?;
    let mut hasher = blake3::Hasher::new();
    let mut bytes = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = file
            .read(&mut buffer)
            .map_err(|e| Error::io("read file", path, e))?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
        bytes += count as u64;
    }
    Ok((hasher.finalize(), bytes))
}
