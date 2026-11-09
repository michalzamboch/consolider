# Consolider

A native Rust desktop app that combines several folder trees into a new folder. Linux is the primary platform; Windows and macOS are supported too. The interface has source folders on the left and the destination and output preview on the right.

## Run on Linux

Install a current stable Rust toolchain using [rustup](https://rustup.rs/).

On Ubuntu / Debian, install the native build dependencies:

```sh
sudo apt-get install build-essential pkg-config libx11-dev libxi-dev libxrandr-dev libxcursor-dev libxinerama-dev libxkbcommon-dev libwayland-dev libegl1-mesa-dev libgl1-mesa-dev
cargo run --release --locked
```

Both Wayland and X11 are enabled. Folder dialogs use XDG desktop portals rather than a GTK build dependency. Your desktop needs `xdg-desktop-portal` and a suitable backend, such as `xdg-desktop-portal-gtk`, `xdg-desktop-portal-gnome`, or `xdg-desktop-portal-kde`, plus a running desktop D-Bus session. Most desktop distributions provide these already. The app needs an OpenGL-capable desktop session.

On Fedora, the equivalent build packages include `gcc-c++`, `pkgconf-pkg-config`, `libX11-devel`, `libXi-devel`, `libXrandr-devel`, `libXcursor-devel`, `libXinerama-devel`, `libxkbcommon-devel`, `wayland-devel`, `mesa-libEGL-devel`, and `mesa-libGL-devel`.

## Windows and macOS

On Windows, install stable Rust with the MSVC toolchain and Visual Studio C++ Build Tools, then run `cargo run --release --locked`. The executable is `target/release/consolider.exe` and has no console window in release mode.

On macOS, install the Xcode Command Line Tools (`xcode-select --install`) and stable Rust, then run `cargo run --release --locked`. The executable is `target/release/consolider`. This project builds a native executable, not an installer or signed `.app` bundle.

## Use

1. Add source folders using **+ Add folders**, or drag folders into the window.
2. Select an existing destination parent and name a **new** folder.
3. Click **Analyze** and inspect the preview.
4. Click **Copy**. Scanning, verification, and copying run in a background thread. Click **Cancel** to stop the current operation; completed copies remain in the destination.

## Union rules

Paths are relative to each source root. Source folder names are not added to the output.

| Inputs | Output |
| --- | --- |
| Same relative path, same contents | One copy |
| Same relative path, different contents | All versions; later versions receive a conflict suffix |
| Different relative paths, same contents | A copy at each path |
| A file and a folder at the same path | Keep the folder structure and give the file a conflict suffix |

For example, `A/docs/note.txt` and `B/docs/note.txt` merge into `Output/docs/note.txt`. If their contents differ, the second version becomes `Output/docs/note (conflict 012345abcdef).txt`. The suffix derives from the content hash; a counter is added if that name is already reserved. The first source in the list keeps the original filename. Linux path matching is case-sensitive; Windows and macOS path matching is case-insensitive. On macOS this follows the usual case-insensitive APFS configuration, even if you use a case-sensitive volume.

Contents are compared with streaming BLAKE3 hashes, including files of the same size. Large files are read in bounded chunks. Empty directories, hidden files, binary files, and original relative paths are retained. File permissions are copied, including executable bits on Linux. Symbolic links and special files are skipped with visible warnings. Directory permissions, ownership, timestamps, ACLs, extended attributes, and hard-link relationships are not replicated; this is a file-content union, not a filesystem backup tool.

The destination must not already exist and must be outside all source folders. Source files are only read. Every input is scanned again before copying to detect changes since the preview; each copied file is also checked against its planned hash. Keep sources unchanged during copying. If copying fails or is cancelled, completed output files remain and the current incomplete file is removed where possible. Use a new destination name to retry; existing output is never overwritten or cleared.

## Development and tests

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets
cargo build --release --locked
```

The reusable consolidation engine is in `src/consolidate/`; the desktop interface and background worker are in `src/app/`. Integration tests exercise union rules, collision handling, changed sources, cancellation, and destination safety. Unix tests cover executable permissions, symlinks, and non-UTF-8 filenames. Headless UI tests check the minimum window layout and exercise Analyze and Copy through the background worker.
