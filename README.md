# CS2 Config Manager

A lightweight Rust application to manage Counter-Strike 2 (CS2) configurations and crosshair profiles across Steam accounts.

## Features
- Copy CS2 configs between Steam accounts with backup support.
- Import, edit, and save crosshair profiles using CS2 share codes.
- Preview crosshairs in the UI, closely matching in-game appearance.
- Supports Linux and Windows (via cross-compilation).

## Requirements
- Rust (`rustup` recommended)
- For Windows builds: `mingw-w64-gcc`
- Dependencies (see `Cargo.toml`):
  ```toml
  num-bigint = "0.4"
  num-traits = "0.2"
  eframe = "0.22"
  serde = { version = "1.0", features = ["derive"] }
  serde_json = "1.0"
  ```

## Installation
1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/cs2-config-manager.git
   cd cs2-config-manager
   ```
2. Install Rust:
   ```bash
   yay -S rustup  # Arch Linux
   rustup toolchain install stable
   rustup default stable
   ```
3. For Windows builds:
   ```bash
   rustup target add x86_64-pc-windows-gnu
   yay -S mingw-w64-gcc
   ```

## Usage
1. Build and run for Linux:
   ```bash
   cargo run --release
   ```
2. Build for Windows:
   ```bash
   cargo build --release --target x86_64-pc-windows-gnu
   ```
   Output: `target/x86_64-pc-windows-gnu/release/cs2_config_app.exe`
3. Features:
   - Select source/target Steam accounts to copy configs.
   - Import crosshair codes (e.g., `CSGO-H3Wb2-YV2FB-VPipW-dx2td-hej5P`).
   - Edit and preview crosshairs, then apply to `config.cfg`.

## Notes
- Ensure Steam is installed (Linux: `~/.steam/steam` or `~/.local/share/Steam`; Windows: `C:\Program Files (x86)\Steam`).
- Windows builds may require `libGLESv2.dll` and `libEGL.dll` for `eframe`.
- Close CS2 before applying configs.

## License
MIT License
