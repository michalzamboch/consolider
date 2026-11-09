use consolider::consolidate::{self, Error, Phase, Plan, Request};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};
use tempfile::TempDir;

struct Fixture {
    _temporary: TempDir,
    first: PathBuf,
    second: PathBuf,
    destination: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let first = temporary.path().join("first");
        let second = temporary.path().join("second");
        let destination = temporary.path().join("union");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        Self {
            _temporary: temporary,
            first,
            second,
            destination,
        }
    }

    fn request(&self) -> Request {
        Request {
            sources: vec![self.first.clone(), self.second.clone()],
            destination: self.destination.clone(),
        }
    }

    fn analyze(&self) -> Plan {
        consolidate::analyze(&self.request(), &AtomicBool::new(false), |_| {}).unwrap()
    }

    fn execute(&self, plan: &Plan) {
        consolidate::execute(plan, &AtomicBool::new(false), |_| {}).unwrap();
    }
}

fn write(root: &Path, relative: impl AsRef<Path>, bytes: impl AsRef<[u8]>) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn unions_nested_files_and_preserves_empty_directories_and_sources() {
    let fixture = Fixture::new();
    write(&fixture.first, "photos/one.jpg", b"photo one");
    write(&fixture.second, "photos/two.jpg", b"photo two");
    fs::create_dir_all(fixture.second.join("empty/nested")).unwrap();
    let plan = fixture.analyze();
    assert_eq!(plan.files.len(), 2);
    assert!(!fixture.destination.exists(), "analysis must be read-only");
    fixture.execute(&plan);
    assert_eq!(
        fs::read(fixture.destination.join("photos/one.jpg")).unwrap(),
        b"photo one"
    );
    assert_eq!(
        fs::read(fixture.destination.join("photos/two.jpg")).unwrap(),
        b"photo two"
    );
    assert!(fixture.destination.join("empty/nested").is_dir());
    assert_eq!(
        fs::read(fixture.first.join("photos/one.jpg")).unwrap(),
        b"photo one"
    );
    assert!(!fixture.destination.join("first").exists());
}

#[test]
fn deduplicates_only_when_both_relative_path_and_content_match() {
    let fixture = Fixture::new();
    write(&fixture.first, "notes/today.txt", b"same");
    write(&fixture.second, "notes/today.txt", b"same");
    write(&fixture.second, "archive/today.txt", b"same");
    let plan = fixture.analyze();
    assert_eq!(plan.input_files(), 3);
    assert_eq!(plan.duplicates, 1);
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.total_bytes(), 8);
    fixture.execute(&plan);
    assert!(fixture.destination.join("notes/today.txt").is_file());
    assert!(fixture.destination.join("archive/today.txt").is_file());
}

#[test]
fn retains_same_size_conflicts_and_deduplicates_repeated_versions() {
    let fixture = Fixture::new();
    write(&fixture.first, "report.txt", b"version A");
    write(&fixture.second, "report.txt", b"version B");
    let third = fixture._temporary.path().join("third");
    write(&third, "report.txt", b"version B");
    let mut request = fixture.request();
    request.sources.push(third);
    let plan = consolidate::analyze(&request, &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.duplicates, 1);
    assert_eq!(plan.renamed_files(), 1);
    let conflict = plan.files.iter().find(|file| file.renamed()).unwrap();
    assert!(
        conflict
            .output_path
            .to_string_lossy()
            .contains(" (conflict ")
    );
    assert_eq!(conflict.output_path.extension().unwrap(), "txt");
    fixture.execute(&plan);
    assert_eq!(
        fs::read(fixture.destination.join("report.txt")).unwrap(),
        b"version A"
    );
    assert_eq!(
        fs::read(fixture.destination.join(&conflict.output_path)).unwrap(),
        b"version B"
    );
}

#[test]
fn generated_conflict_names_cannot_take_an_original_files_path() {
    let fixture = Fixture::new();
    write(&fixture.first, "report.txt", b"A");
    write(&fixture.second, "report.txt", b"B");
    let hash = blake3::hash(b"B").to_hex();
    let reserved = format!("report (conflict {}).txt", &hash[..12]);
    write(&fixture.second, &reserved, b"real file");
    let plan = fixture.analyze();
    assert_eq!(plan.files.len(), 3);
    assert!(
        plan.files
            .iter()
            .any(|file| file.output_path.to_string_lossy().contains("-2).txt"))
    );
    fixture.execute(&plan);
    assert_eq!(
        fs::read(fixture.destination.join(reserved)).unwrap(),
        b"real file"
    );
}

#[test]
fn file_directory_collisions_keep_directory_structure_and_rename_file() {
    let fixture = Fixture::new();
    write(&fixture.first, "documents", b"a file named documents");
    write(&fixture.second, "documents/sub/note.txt", b"nested note");
    let plan = fixture.analyze();
    assert_eq!(plan.renamed_files(), 1);
    fixture.execute(&plan);
    assert!(fixture.destination.join("documents").is_dir());
    assert_eq!(
        fs::read(fixture.destination.join("documents/sub/note.txt")).unwrap(),
        b"nested note"
    );
    let renamed = plan.files.iter().find(|file| file.renamed()).unwrap();
    assert_eq!(
        fs::read(fixture.destination.join(&renamed.output_path)).unwrap(),
        b"a file named documents"
    );
}

#[test]
fn keeps_empty_binary_hidden_and_unicode_files() {
    let fixture = Fixture::new();
    write(&fixture.first, ".hidden", []);
    let bytes: Vec<u8> = (0..200_000).map(|index| (index % 256) as u8).collect();
    write(&fixture.second, "Příliš žluťoučký/数据.bin", &bytes);
    let plan = fixture.analyze();
    fixture.execute(&plan);
    assert!(
        fs::read(fixture.destination.join(".hidden"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        fs::read(fixture.destination.join("Příliš žluťoučký/数据.bin")).unwrap(),
        bytes
    );
}

#[test]
fn repeated_source_folders_are_normalized_once() {
    let fixture = Fixture::new();
    write(&fixture.first, "file", b"once");
    let mut request = fixture.request();
    request.sources.push(fixture.first.join("."));
    let plan = consolidate::analyze(&request, &AtomicBool::new(false), |_| {}).unwrap();
    assert_eq!(plan.request.sources.len(), 2);
    assert_eq!(plan.input_files(), 1);
}

#[test]
fn overlapping_source_roots_preserve_their_distinct_relative_paths() {
    let fixture = Fixture::new();
    write(&fixture.first, "nested/file", b"data");
    let request = Request {
        sources: vec![fixture.first.clone(), fixture.first.join("nested")],
        destination: fixture.destination.clone(),
    };
    let plan = consolidate::analyze(&request, &AtomicBool::new(false), |_| {}).unwrap();
    fixture.execute(&plan);
    assert!(fixture.destination.join("nested/file").is_file());
    assert!(fixture.destination.join("file").is_file());
}

#[test]
fn accepts_empty_sources_and_creates_a_new_empty_destination() {
    let fixture = Fixture::new();
    let plan = fixture.analyze();
    assert_eq!(plan.files.len(), 0);
    fixture.execute(&plan);
    assert!(fixture.destination.is_dir());
}

#[test]
fn rejects_an_empty_source_list_and_sources_that_are_files() {
    let fixture = Fixture::new();
    let mut request = fixture.request();
    request.sources.clear();
    assert!(consolidate::analyze(&request, &AtomicBool::new(false), |_| {}).is_err());
    write(&fixture.first, "file", b"x");
    request.sources.push(fixture.first.join("file"));
    assert!(consolidate::analyze(&request, &AtomicBool::new(false), |_| {}).is_err());
}

#[test]
fn rejects_destinations_inside_sources_without_writing_anything() {
    let fixture = Fixture::new();
    let mut request = fixture.request();
    request.destination = fixture.first.join("new-output");
    assert!(consolidate::analyze(&request, &AtomicBool::new(false), |_| {}).is_err());
    assert!(!request.destination.exists());
}

#[test]
fn refuses_existing_destinations_even_when_empty() {
    let fixture = Fixture::new();
    fs::create_dir(&fixture.destination).unwrap();
    assert!(consolidate::analyze(&fixture.request(), &AtomicBool::new(false), |_| {}).is_err());
    write(&fixture.destination, "valuable", b"keep me");
    assert!(consolidate::analyze(&fixture.request(), &AtomicBool::new(false), |_| {}).is_err());
    assert_eq!(
        fs::read(fixture.destination.join("valuable")).unwrap(),
        b"keep me"
    );
}

#[test]
fn refuses_a_destination_created_after_preview() {
    let fixture = Fixture::new();
    write(&fixture.first, "valuable", b"source");
    let plan = fixture.analyze();
    write(&fixture.destination, "valuable", b"existing output");
    assert!(consolidate::execute(&plan, &AtomicBool::new(false), |_| {}).is_err());
    assert_eq!(
        fs::read(fixture.destination.join("valuable")).unwrap(),
        b"existing output"
    );
}

#[test]
fn detects_changed_duplicates_before_creating_destination() {
    let fixture = Fixture::new();
    write(&fixture.first, "file", b"A");
    write(&fixture.second, "file", b"A");
    let plan = fixture.analyze();
    write(&fixture.second, "file", b"B");
    let error = consolidate::execute(&plan, &AtomicBool::new(false), |_| {}).unwrap_err();
    assert!(error.to_string().contains("Sources changed"));
    assert!(!fixture.destination.exists());
}

#[test]
fn detects_new_and_removed_files_after_preview() {
    let fixture = Fixture::new();
    write(&fixture.first, "old", b"data");
    let plan = fixture.analyze();
    write(&fixture.second, "new", b"new data");
    assert!(consolidate::execute(&plan, &AtomicBool::new(false), |_| {}).is_err());
    fs::remove_file(fixture.second.join("new")).unwrap();
    fs::remove_file(fixture.first.join("old")).unwrap();
    assert!(consolidate::execute(&plan, &AtomicBool::new(false), |_| {}).is_err());
    assert!(!fixture.destination.exists());
}

#[test]
fn detects_changes_between_verification_and_copy_and_removes_bad_output() {
    let fixture = Fixture::new();
    write(&fixture.first, "file", b"original");
    let plan = fixture.analyze();
    let result = consolidate::execute(&plan, &AtomicBool::new(false), |progress| {
        if progress.phase == Phase::Verifying && progress.current_path.as_os_str().is_empty() {
            write(&fixture.first, "file", b"modified");
        }
    });
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("changed while copying")
    );
    assert!(!fixture.destination.join("file").exists());
}

#[test]
fn cancelled_analysis_and_copy_before_start_leave_no_destination() {
    let fixture = Fixture::new();
    let plan = fixture.analyze();
    let cancel = AtomicBool::new(true);
    assert!(matches!(
        consolidate::analyze(&fixture.request(), &cancel, |_| {}),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        consolidate::execute(&plan, &cancel, |_| {}),
        Err(Error::Cancelled)
    ));
    assert!(!fixture.destination.exists());
}

#[test]
fn cancelling_mid_file_removes_only_the_incomplete_copy() {
    let fixture = Fixture::new();
    write(&fixture.first, "a-complete", b"small");
    write(&fixture.first, "b-incomplete", vec![42; 200_000]);
    let plan = fixture.analyze();
    let cancel = AtomicBool::new(false);
    let result = consolidate::execute(&plan, &cancel, |progress| {
        if progress.phase == Phase::Copying
            && progress.current_path == Path::new("b-incomplete")
            && progress.bytes > 5
        {
            cancel.store(true, Ordering::Relaxed);
        }
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(
        fs::read(fixture.destination.join("a-complete")).unwrap(),
        b"small"
    );
    assert!(!fixture.destination.join("b-incomplete").exists());
    assert_eq!(
        fs::metadata(fixture.first.join("b-incomplete"))
            .unwrap()
            .len(),
        200_000
    );
}

#[test]
fn reports_final_copy_counts_and_bytes() {
    let fixture = Fixture::new();
    write(&fixture.first, "one", b"123");
    write(&fixture.second, "two", b"45");
    let plan = fixture.analyze();
    let mut last = None;
    consolidate::execute(&plan, &AtomicBool::new(false), |progress| {
        last = Some(progress)
    })
    .unwrap();
    let last = last.unwrap();
    assert_eq!(last.phase, Phase::Copying);
    assert_eq!(last.files, 2);
    assert_eq!(last.bytes, 5);
}

#[cfg(target_os = "linux")]
#[test]
fn linux_paths_are_case_sensitive() {
    let fixture = Fixture::new();
    write(&fixture.first, "Readme", b"same");
    write(&fixture.second, "README", b"same");
    let plan = fixture.analyze();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.duplicates, 0);
    fixture.execute(&plan);
    assert!(fixture.destination.join("Readme").is_file());
    assert!(fixture.destination.join("README").is_file());
}

#[cfg(any(windows, target_os = "macos"))]
#[test]
fn windows_and_macos_paths_match_case_insensitively_and_keep_different_contents() {
    let fixture = Fixture::new();
    write(&fixture.first, "Readme", b"first");
    write(&fixture.second, "README", b"second");
    let plan = fixture.analyze();
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.renamed_files(), 1);
    fixture.execute(&plan);
}

#[cfg(unix)]
#[test]
fn preserves_executable_file_permissions_on_unix() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    write(&fixture.first, "script.sh", b"#!/bin/sh\nprintf hello\n");
    fs::set_permissions(
        fixture.first.join("script.sh"),
        fs::Permissions::from_mode(0o751),
    )
    .unwrap();
    fixture.execute(&fixture.analyze());
    assert_eq!(
        fs::metadata(fixture.destination.join("script.sh"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o751
    );
}

#[cfg(unix)]
#[test]
fn symbolic_links_including_cycles_and_broken_links_are_skipped() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    write(&fixture.first, "real", b"data");
    symlink(&fixture.first, fixture.first.join("cycle")).unwrap();
    symlink(fixture.first.join("real"), fixture.first.join("link")).unwrap();
    symlink(fixture.first.join("missing"), fixture.first.join("broken")).unwrap();
    let plan = fixture.analyze();
    assert_eq!(plan.warnings.len(), 3);
    assert_eq!(plan.files.len(), 1);
    fixture.execute(&plan);
    assert!(!fixture.destination.join("cycle").exists());
}

#[cfg(unix)]
#[test]
fn handles_non_utf8_file_names_without_lossy_conversion() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let fixture = Fixture::new();
    let name = OsString::from_vec(vec![b'f', 0xff]);
    write(&fixture.first, &name, b"data");
    let plan = fixture.analyze();
    fixture.execute(&plan);
    assert_eq!(fs::read(fixture.destination.join(name)).unwrap(), b"data");
}
