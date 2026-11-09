use super::{Error, Phase, Plan, PlannedFile, Progress, Result, analyze, check_cancel};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
    sync::atomic::AtomicBool,
};

pub fn execute(plan: &Plan, cancel: &AtomicBool, mut report: impl FnMut(Progress)) -> Result<()> {
    // Rebuild the snapshot before making any destination changes. This detects new,
    // deleted, changed, and newly duplicated inputs after the user's preview.
    let current = analyze(&plan.request, cancel, |mut progress| {
        progress.phase = Phase::Verifying;
        report(progress);
    })?;
    if current != *plan {
        return Err(Error::Invalid(
            "Sources changed after the preview. Analyze again before copying.".into(),
        ));
    }
    check_cancel(cancel)?;
    let destination = &plan.request.destination;
    // create_dir is exclusive: an existing destination is never reused or cleared.
    fs::create_dir(destination).map_err(|e| Error::io("create new destination", destination, e))?;
    for relative in &plan.directories {
        check_cancel(cancel)?;
        let directory = destination.join(relative);
        fs::create_dir_all(&directory).map_err(|e| Error::io("create folder", &directory, e))?;
    }
    let mut bytes = 0;
    for (index, file) in plan.files.iter().enumerate() {
        check_cancel(cancel)?;
        let output = destination.join(&file.output_path);
        copy_verified(file, &output, cancel, |copied| {
            report(Progress {
                phase: Phase::Copying,
                files: index,
                bytes: bytes + copied,
                current_path: file.output_path.clone(),
            });
        })?;
        bytes += file.bytes;
        report(Progress {
            phase: Phase::Copying,
            files: index + 1,
            bytes,
            current_path: file.output_path.clone(),
        });
    }
    Ok(())
}

fn copy_verified(
    file: &PlannedFile,
    output: &Path,
    cancel: &AtomicBool,
    mut report: impl FnMut(u64),
) -> Result<()> {
    let metadata = fs::symlink_metadata(&file.source)
        .map_err(|e| Error::io("inspect source", &file.source, e))?;
    if !metadata.is_file() {
        return Err(Error::Invalid(format!(
            "Source is no longer a regular file: {}",
            file.source.display()
        )));
    }
    let mut input =
        File::open(&file.source).map_err(|e| Error::io("read source", &file.source, e))?;
    // Never truncate a pre-existing file, including a file created during copying.
    let mut target = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| Error::io("create output file", output, e))?;
    let result = write_verified(&mut input, &mut target, file, output, cancel, &mut report)
        .and_then(|()| {
            target
                .set_permissions(metadata.permissions())
                .map_err(|e| Error::io("preserve file permissions", output, e))
        });
    drop(target);
    if result.is_err() {
        // Only this incomplete file is removed. Completed copies remain available.
        let _ = fs::remove_file(output);
    }
    result
}

fn write_verified(
    input: &mut File,
    target: &mut File,
    file: &PlannedFile,
    output: &Path,
    cancel: &AtomicBool,
    report: &mut impl FnMut(u64),
) -> Result<()> {
    let mut hasher = blake3::Hasher::new();
    let mut bytes = 0;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        check_cancel(cancel)?;
        let count = input
            .read(&mut buffer)
            .map_err(|e| Error::io("read source", &file.source, e))?;
        if count == 0 {
            break;
        }
        target
            .write_all(&buffer[..count])
            .map_err(|e| Error::io("write output", output, e))?;
        hasher.update(&buffer[..count]);
        bytes += count as u64;
        report(bytes);
    }
    if hasher.finalize() != file.hash || bytes != file.bytes {
        return Err(Error::Invalid(format!(
            "Source changed while copying: {}. Analyze again into a new destination.",
            file.source.display()
        )));
    }
    target
        .sync_all()
        .map_err(|e| Error::io("flush output", output, e))?;
    Ok(())
}
