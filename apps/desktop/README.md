# Toonvara desktop

The current desktop shell uses Rust with eframe/egui. The official artwork is
[`public/favicon.png`](../../public/favicon.png). It is embedded in the binary
at compile time, so icons and the About dialog do not depend on the launch
directory or files installed alongside the executable.

The same image supplies the native window icon on supported platforms and the
macOS Dock icon through eframe's native application-icon support. macOS does
not normally show application icons in window title bars; the window header
also displays the logo. Select **About Toonvara** to see the full About dialog.

Development:

```sh
cargo run -p toonvara-desktop
```

Generate icons after changing the source artwork (Python 3 with Pillow):

```sh
python3 scripts/generate_icons.py
```

Generated files in `assets/icons/` include a macOS `.icns`, all ten standard
1x/2x `.iconset` PNGs (16–1024 pixels), and a multi-resolution Windows `.ico`.
PNG variants use Lanczos resizing directly from the source and retain alpha
and any embedded ICC profile. Nonsquare artwork is fitted onto a transparent
square without cropping or distortion. The source is never modified.

Build a release macOS application bundle:

```sh
python3 scripts/package_macos.py
open target/release/bundle/Toonvara.app
```

For a development bundle, pass `--debug`; output is under
`target/debug/bundle/Toonvara.app`. A custom Cargo target directory is respected.
The packager regenerates the icons, builds the binary, writes `Info.plist`
with the bundle icon/name/version, copies the `.icns` into bundle resources,
and ad-hoc signs and verifies the result. Finder and the Dock use this icon
before launch; the embedded logo also supplies the runtime icon.

Release bundles are built for the host architecture. Developer ID signing and
Apple notarization for public distribution are separate release steps.
