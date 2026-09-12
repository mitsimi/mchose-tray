# MCHOSE Tray

A small Windows tray app for the MCHOSE K7 V2 Ultra+ wireless receiver.
It shows either a large battery percentage or an upright, colour-coded battery icon.

## Use

Install `MCHOSE Tray Setup.exe`, then launch **MCHOSE Tray** from the Start menu. The icon appears next to the clock, possibly under the hidden-icons arrow.

- **Percentage** shows large digits; the tooltip includes the exact percentage.
- **Colored battery** fills bottom to top: green above 50%, amber at 21–50%, red at 20% or below.
- **Refresh now** requests a new reading.
- **Quit** closes the app.

Your display choice is saved automatically. `?` means the mouse did not return a usable reading; `–` means the receiver is not connected.

Select **Start with Windows** from the tray menu to run it automatically after sign-in. The installer also offers this option.

## Build

Requires stable Rust with the Windows MSVC toolchain.

```powershell
cargo run --release
cargo test
cargo clippy --all-targets -- -D warnings
```


`mchose-check` reads the battery without starting the tray app:

```powershell
cargo run --release --bin mchose-check
```

The battery protocol is documented in [PROTOCOL.md](PROTOCOL.md).

## Creating the installer

Install [Inno Setup](https://jrsoftware.org/isinfo.php), build the release executable, then open `installer/MchoseTray.iss` in Inno Setup and choose **Build → Compile**. The installer is written to `dist` and installs per user, without requiring administrator rights.

## Credits

This project is based on [Mouse Tray Charge](https://github.com/Fan4Metal/mouse_tray) by Fan4Metal. Its driver structure and the original Python implementation provided the foundation for validating the MCHOSE receiver protocol before this Rust tray app was built.
