# CS2 Config Manager

A lightweight Rust application to manage Counter-Strike 2 (CS2) configurations and crosshair profiles across Steam accounts.

![cs2man](https://github.com/jbuck95/cs2man/blob/main/image.jpg?raw=true)


## Features
- Copy CS2 configs between Steam accounts with backup support.
- Import, edit, and save crosshair profiles using CS2 share codes.
- Linux/Windows 

## Usage

1. Download Release or manually install
2. Keep CS closed while changing config
3. keep the crosshair_profiles.json in the same folder as your binary.

## manual Installation

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


1. Clone the repository:
   ```bash
   git clone https://github.com/jbuck95/cs2man
   cd cs2man
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

## Build
1. Build and run for Linux:
   ```bash
   cargo run --release
   ```
2. Build for Windows:
   ```bash
   cargo build --release --target x86_64-pc-windows-gnu
   ```
   Output: `target/x86_64-pc-windows-gnu/release/cs2man.exe`
3. Features:
   - Select source/target Steam accounts to copy configs.
   - Import crosshair codes (e.g., `CSGO-H3Wb2-YV2FB-VPipW-dx2td-hej5P`).
   - Edit and preview (rendering super buggy) crosshairs, then apply to `config.cfg` or simply copy code.

## Notes
- Ensure Steam is installed (Linux: `~/.steam/steam` or `~/.local/share/Steam`; Windows: `C:\Program Files (x86)\Steam`).
- Close CS2 before applying configs.

## License
MIT License
