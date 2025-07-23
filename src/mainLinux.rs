use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use eframe::egui;
use serde::{Deserialize, Serialize};
use num_bigint::BigUint;
use num_traits::{Zero, One};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SteamAccount {
    id: String,
    name: Option<String>,
    cs2_config_path: Option<PathBuf>,
    has_cs2_config: bool,
    config_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrosshairProfile {
    gap: f32,
    outline_thickness: f32,
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
    dynamic_splitdist: u8,
    recoil: bool,
    fixed_gap: f32,
    color: u8,
    draw_outline: bool,
    dynamic_splitalpha_innermod: f32,
    dynamic_splitalpha_outermod: f32,
    dynamic_maxdist_split_ratio: f32,
    thickness: f32,
    style: u8,
    dot: bool,
    gap_use_weapon_value: bool,
    use_alpha: bool,
    t: bool,
    size: f32,
    name: String,
    original_code: Option<String>,
}

#[derive(Debug, Clone)]
enum AppState {
    Loading,
    Ready,
    Copying,
    Error(String),
}

#[derive(Debug, Clone)]
struct CopyOperation {
    from_id: String,
    to_id: String,
    backup: bool,
    progress: f32,
    status: String,
}

pub struct CS2ConfigApp {
    steam_path: Option<PathBuf>,
    accounts: Vec<SteamAccount>,
    selected_source: Option<usize>,
    selected_target: Option<usize>,
    state: AppState,
    error_message: String,
    success_message: String,
    show_backup_option: bool,
    create_backup: bool,
    copy_operation: Option<CopyOperation>,
    search_filter: String,
    show_only_with_configs: bool,
    crosshair_library: Vec<CrosshairProfile>,
    selected_library_idx: Option<usize>,
    active_profile: CrosshairProfile,
    crosshair_code_input: String,
}

impl Default for CS2ConfigApp {
    fn default() -> Self {
        Self {
            steam_path: None,
            accounts: Vec::new(),
            selected_source: None,
            selected_target: None,
            state: AppState::Loading,
            error_message: String::new(),
            success_message: String::new(),
            show_backup_option: true,
            create_backup: true,
            copy_operation: None,
            search_filter: String::new(),
            show_only_with_configs: false,
            crosshair_library: Vec::new(),
            selected_library_idx: None,
            active_profile: CrosshairProfile {
                gap: 0.0,
                outline_thickness: 1.0,
                red: 255,
                green: 255,
                blue: 255,
                alpha: 255,
                dynamic_splitdist: 0,
                recoil: false,
                fixed_gap: 0.0,
                color: 1,
                draw_outline: true,
                dynamic_splitalpha_innermod: 0.5,
                dynamic_splitalpha_outermod: 0.5,
                dynamic_maxdist_split_ratio: 0.5,
                thickness: 0.5,
                style: 4,
                dot: false,
                gap_use_weapon_value: false,
                use_alpha: true,
                t: false,
                size: 5.0,
                name: "Default".to_string(),
                original_code: None,
            },
            crosshair_code_input: String::new(),
        }
    }
}

impl CS2ConfigApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();
        let ctx = cc.egui_ctx.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            ctx.request_repaint();
        });
        app.load_steam_data();
        app.load_crosshair_profiles();
        app
    }

    fn load_steam_data(&mut self) {
        match self.find_steam_path() {
            Ok(path) => {
                self.steam_path = Some(path.clone());
                match self.scan_accounts(&path) {
                    Ok(accounts) => {
                        self.accounts = accounts;
                        self.state = AppState::Ready;
                        self.success_message = format!("Found {} Steam accounts", self.accounts.len());
                    }
                    Err(e) => self.state = AppState::Error(format!("Failed to scan accounts: {}", e)),
                }
            }
            Err(e) => self.state = AppState::Error(format!("Steam not found: {}", e)),
        }
    }

    fn find_steam_path(&self) -> Result<PathBuf, String> {
        if cfg!(target_os = "linux") {
            if let Ok(home) = std::env::var("HOME") {
                let possible_paths = vec![
                    format!("{}/.steam/steam", home),
                    format!("{}/.local/share/Steam", home),
                    format!("{}/.var/app/com.valvesoftware.Steam/.local/share/Steam", home),
                ];
                for path_str in possible_paths {
                    let path = PathBuf::from(path_str);
                    if path.exists() && path.join("userdata").exists() {
                        return Ok(path);
                    }
                }
            }
        }
        Err("Steam installation not found".to_string())
    }

    fn scan_accounts(&self, steam_path: &Path) -> Result<Vec<SteamAccount>, String> {
        let userdata_path = steam_path.join("userdata");
        if !userdata_path.exists() { return Err("Steam userdata directory not found".to_string()); }
        let mut accounts = Vec::new();
        for entry in fs::read_dir(&userdata_path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(account_id) = path.file_name().and_then(|n| n.to_str()) {
                    if account_id.chars().all(|c| c.is_ascii_digit()) {
                        match self.analyze_account(account_id, &path) {
                            Ok(account) => accounts.push(account),
                            Err(_) => continue,
                        }
                    }
                }
            }
        }
        accounts.sort_by(|a, b| match (a.has_cs2_config, b.has_cs2_config) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.id.cmp(&b.id),
        });
        Ok(accounts)
    }

    fn analyze_account(&self, account_id: &str, account_path: &Path) -> Result<SteamAccount, String> {
        let cs2_config_path = account_path.join("730").join("local").join("cfg");
        let has_cs2_config = cs2_config_path.exists();
        let mut config_files = Vec::new();
        if has_cs2_config {
            if let Ok(entries) = fs::read_dir(&cs2_config_path) {
                for entry in entries.flatten() {
                    if let Some(file_name) = entry.file_name().to_str() {
                        if file_name.ends_with(".cfg") || file_name.ends_with(".txt") {
                            config_files.push(file_name.to_string());
                        }
                    }
                }
            }
            config_files.sort();
        }
        let name = self.get_account_name(account_path);
        Ok(SteamAccount {
            id: account_id.to_string(),
            name,
            cs2_config_path: if has_cs2_config { Some(cs2_config_path) } else { None },
            has_cs2_config,
            config_files,
        })
    }

    fn get_account_name(&self, account_path: &Path) -> Option<String> {
        let localconfig_path = account_path.join("config").join("localconfig.vdf");
        if let Ok(content) = fs::read_to_string(&localconfig_path) {
            for line in content.lines() {
                if line.contains("PersonaName") {
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line.rfind('"') {
                            if start != end {
                                let name = &line[start + 1..end];
                                if !name.is_empty() && name != "PersonaName" {
                                    return Some(name.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    fn copy_config_async(&mut self, from_idx: usize, to_idx: usize, backup: bool) {
        if from_idx >= self.accounts.len() || to_idx >= self.accounts.len() {
            self.error_message = "Invalid account selection".to_string();
            return;
        }
        let source = self.accounts[from_idx].clone();
        let target = self.accounts[to_idx].clone();
        let source_config = match &source.cs2_config_path {
            Some(path) => path.clone(),
            None => {
                self.error_message = "Source account has no CS2 config".to_string();
                return;
            }
        };
        self.state = AppState::Copying;
        self.copy_operation = Some(CopyOperation {
            from_id: source.id.clone(),
            to_id: target.id.clone(),
            backup,
            progress: 0.0,
            status: "Starting copy operation...".to_string(),
        });
        let result = self.perform_copy(&source_config, from_idx, to_idx, backup);
        match result {
            Ok(_) => {
                self.success_message = format!("Successfully copied CS2 config from {} to {}", source.name.as_deref().unwrap_or(&source.id), target.name.as_deref().unwrap_or(&target.id));
                self.state = AppState::Ready;
                self.copy_operation = None;
                if let Some(steam_path) = &self.steam_path.clone() {
                    if let Ok(accounts) = self.scan_accounts(steam_path) {
                        self.accounts = accounts;
                    }
                }
            }
            Err(e) => {
                self.error_message = format!("Copy failed: {}", e);
                self.state = AppState::Error(e);
                self.copy_operation = None;
            }
        }
    }

    fn perform_copy(&mut self, source_config: &Path, from_idx: usize, to_idx: usize, backup: bool) -> Result<(), String> {
        let target_account = &self.accounts[to_idx];
        let steam_path = self.steam_path.as_ref().ok_or("No Steam path")?;
        let target_config = if let Some(ref existing_path) = target_account.cs2_config_path {
            existing_path.clone()
        } else {
            let userdata_path = steam_path.join("userdata");
            let target_path = userdata_path.join(&target_account.id).join("730").join("local").join("cfg");
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            target_path
        };
        if let Some(ref mut op) = self.copy_operation { op.progress = 0.1; op.status = "Preparing directories...".to_string(); }
        if backup && target_config.exists() {
            let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let backup_path = target_config.with_extension(format!("backup.{}", timestamp));
            if let Some(ref mut op) = self.copy_operation { op.progress = 0.3; op.status = format!("Creating backup at {}...", backup_path.display()); }
            self.copy_dir_recursive(source_config, &backup_path)?;
        }
        if !target_config.exists() { fs::create_dir_all(&target_config).map_err(|e| e.to_string())?; }
        if let Some(ref mut op) = self.copy_operation { op.progress = 0.5; op.status = "Copying configuration files...".to_string(); }
        self.copy_dir_recursive(source_config, &target_config)?;
        if let Some(ref mut op) = self.copy_operation { op.progress = 1.0; op.status = "Copy completed successfully!".to_string(); }
        Ok(())
    }

    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<(), String> {
        if !dst.exists() { fs::create_dir_all(dst).map_err(|e| e.to_string())?; }
        for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                self.copy_dir_recursive(&src_path, &dst_path)?;
            } else {
                fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    fn get_filtered_accounts(&self) -> Vec<(usize, SteamAccount)> {
        self.accounts.iter().enumerate().filter(|(_, account)| {
            if self.show_only_with_configs && !account.has_cs2_config { return false; }
            if self.search_filter.is_empty() { return true; }
            let filter = self.search_filter.to_lowercase();
            account.id.to_lowercase().contains(&filter) || account.name.as_ref().map_or(false, |n| n.to_lowercase().contains(&filter))
        }).map(|(idx, account)| (idx, account.clone())).collect()
    }

    fn load_crosshair_profiles(&mut self) {
        let profile_path = PathBuf::from("crosshair_profiles.json");
        if profile_path.exists() {
            if let Ok(content) = fs::read_to_string(&profile_path) {
                if let Ok(profiles) = serde_json::from_str(&content) {
                    self.crosshair_library = profiles;
                }
            }
        }
    }

    fn save_crosshair_profiles(&self) {
        let profile_path = PathBuf::from("crosshair_profiles.json");
        if let Ok(content) = serde_json::to_string_pretty(&self.crosshair_library) {
            let _ = fs::write(profile_path, content);
        }
    }

    fn apply_crosshair_to_config(&self, profile: &CrosshairProfile, config_path: &Path) {
        if let Ok(mut content) = fs::read_to_string(config_path) {
            let commands = format!(
                "cl_crosshairgap {}\ncl_crosshair_outlinethickness {}\ncl_crosshaircolor_r {}\ncl_crosshaircolor_g {}\ncl_crosshaircolor_b {}\ncl_crosshairalpha {}\ncl_crosshair_dynamic_splitdist {}\ncl_crosshair_recoil {}\ncl_fixedcrosshairgap {}\ncl_crosshaircolor {}\ncl_crosshair_drawoutline {}\ncl_crosshair_dynamic_splitalpha_innermod {}\ncl_crosshair_dynamic_splitalpha_outermod {}\ncl_crosshair_dynamic_maxdist_splitratio {}\ncl_crosshairthickness {}\ncl_crosshairstyle {}\ncl_crosshairdot {}\ncl_crosshairgap_useweaponvalue {}\ncl_crosshairusealpha {}\ncl_crosshair_t {}\ncl_crosshairsize {}",
                profile.gap, profile.outline_thickness, profile.red, profile.green, profile.blue, profile.alpha,
                profile.dynamic_splitdist, profile.recoil, profile.fixed_gap, profile.color, profile.draw_outline,
                profile.dynamic_splitalpha_innermod, profile.dynamic_splitalpha_outermod, profile.dynamic_maxdist_split_ratio,
                profile.thickness, profile.style, profile.dot, profile.gap_use_weapon_value, profile.use_alpha, profile.t,
                profile.size
            );
            if !content.contains("cl_crosshairgap") {
                content.push_str(&format!("\n{}", commands));
            }
            let _ = fs::write(config_path, content);
        }
    }

    fn parse_crosshair_code(&mut self, code: &str) -> Option<CrosshairProfile> {
        const DICTIONARY: &str = "ABCDEFGHJKLMNOPQRSTUVWXYZabcdefhijkmnopqrstuvwxyz23456789";
        const DICTIONARY_LENGTH: u64 = 57;

        if !code.starts_with("CSGO-") || code.matches('-').count() != 5 {
            eprintln!("Invalid code format: {}", code);
            return None;
        }
        let parts: Vec<&str> = code.split('-').collect();
        if parts.len() != 6 || parts[0] != "CSGO" {
            eprintln!("Invalid parts: {:?}", parts);
            return None;
        }
        let chars: String = parts[1..].join("");
        if chars.len() != 25 {
            eprintln!("Invalid character length: {}", chars.len());
            return None;
        }

        let mut num = BigUint::zero();
        let base = BigUint::from(DICTIONARY_LENGTH);
        for (i, c) in chars.chars().rev().enumerate() {
            let idx = match DICTIONARY.find(c) {
                Some(idx) => idx as u64,
                None => {
                    eprintln!("Invalid character '{}' at position {}", c, i);
                    return None;
                }
            };
            num = num * &base + BigUint::from(idx);
        }

        let hexnum = format!("{:x}", num);
        let padded_hex = format!("{:0>36}", hexnum);
        let bytes: Vec<u8> = (0..padded_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&padded_hex[i..i + 2], 16).unwrap_or(0))
            .collect();

        if bytes.len() < 18 {
            eprintln!("Insufficient bytes: {}", bytes.len());
            return None;
        }

        let checksum = bytes[1..18]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(b as u16)) as u8;
        if bytes[0] != checksum {
            eprintln!("Checksum mismatch: expected {}, got {}", checksum, bytes[0]);
        }

        Some(CrosshairProfile {
            gap: (bytes[2] as i8) as f32 / 10.0,
            outline_thickness: bytes[3] as f32 / 2.0,
            red: bytes[4],
            green: bytes[5],
            blue: bytes[6],
            alpha: bytes[7],
            dynamic_splitdist: bytes[8] & 0x7f,
            recoil: (bytes[8] >> 7) != 0,
            fixed_gap: (bytes[9] as i8) as f32 / 10.0,
            color: bytes[10] & 0x07,
            draw_outline: (bytes[10] & 0x08) != 0,
            dynamic_splitalpha_innermod: ((bytes[10] >> 4) as f32) / 10.0,
            dynamic_splitalpha_outermod: ((bytes[11] & 0x0f) as f32) / 10.0,
            dynamic_maxdist_split_ratio: ((bytes[11] >> 4) as f32) / 10.0,
            thickness: bytes[12] as f32 / 10.0,
            style: (bytes[13] & 0x0f) >> 1,
            dot: (bytes[13] & 0x10) != 0,
            gap_use_weapon_value: (bytes[13] & 0x20) != 0,
            use_alpha: (bytes[13] & 0x40) != 0,
            t: (bytes[13] & 0x80) != 0,
            size: (((bytes[15] & 0x1f) as u16) << 8 | bytes[14] as u16) as f32 / 10.0,
            name: format!("Imported_{}", parts[1]),
            original_code: Some(code.to_string()),
        })
    }

    fn signed_byte(x: u8) -> i8 {
        ((x ^ 0x80u8) as i8) - (0x80u8 as i8)
    }

    fn generate_crosshair_code(&self, profile: &CrosshairProfile) -> String {
        if let Some(ref original_code) = profile.original_code {
            return original_code.clone();
        }

        const DICTIONARY: &str = "ABCDEFGHJKLMNOPQRSTUVWXYZabcdefhijkmnopqrstuvwxyz23456789";
        const DICTIONARY_LENGTH: u64 = 57;

        let mut bytes = vec![
            0, // Checksum placeholder
            1, // Version/ID byte
            ((profile.gap * 10.0) as i8) as u8,
            (profile.outline_thickness * 2.0).min(255.0) as u8,
            profile.red,
            profile.green,
            profile.blue,
            profile.alpha,
            profile.dynamic_splitdist | ((profile.recoil as u8) << 7),
            ((profile.fixed_gap * 10.0) as i8) as u8,
            (profile.color & 0x07) | ((profile.draw_outline as u8) << 3) | (((profile.dynamic_splitalpha_innermod * 10.0).min(15.0) as u8) << 4),
            ((profile.dynamic_splitalpha_outermod * 10.0).min(15.0) as u8 & 0x0F) | (((profile.dynamic_maxdist_split_ratio * 10.0).min(15.0) as u8) << 4),
            (profile.thickness * 10.0).min(255.0) as u8,
            (profile.style << 1) |
            ((profile.dot as u8) << 4) |
            ((profile.gap_use_weapon_value as u8) << 5) |
            ((profile.use_alpha as u8) << 6) |
            ((profile.t as u8) << 7),
            (profile.size * 10.0).min(65535.0) as u16 as u8,
            (((profile.size * 10.0).min(65535.0) as u16) >> 8) as u8 & 0x1f,
            0,
            0,
        ];

        bytes[0] = bytes[1..]
            .iter()
            .fold(0u16, |acc, &b| acc.wrapping_add(b as u16)) as u8;

        let mut num = BigUint::zero();
        let base = BigUint::from(256u64);
        for &byte in bytes.iter().rev() {
            num = num * &base + BigUint::from(byte as u64);
        }

        let mut code = String::with_capacity(25);
        let dict_base = BigUint::from(DICTIONARY_LENGTH);
        if num.is_zero() {
            code.push_str(&"a".repeat(25));
        } else {
            for _ in 0..25 {
                let remainder = (&num % &dict_base).to_u64_digits().get(0).copied().unwrap_or(0) as usize;
                num /= &dict_base;
                code.insert(0, DICTIONARY.chars().nth(remainder).unwrap_or('a'));
            }
        }

        format!("CSGO-{}-{}-{}-{}-{}", &code[0..5], &code[5..10], &code[10..15], &code[15..20], &code[20..25])
    }
}

impl eframe::App for CS2ConfigApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎮 CS2 Config Manager");
            ui.separator();

            match &self.state {
                AppState::Loading => {
                    ui.horizontal(|ui| { ui.spinner(); ui.label("Loading Steam accounts..."); });
                    return;
                }
                AppState::Error(err) => {
                    ui.colored_label(egui::Color32::RED, format!("❌ Error: {}", err));
                    if ui.button("🔄 Retry").clicked() { self.state = AppState::Loading; self.load_steam_data(); }
                    return;
                }
                AppState::Copying => {
                    if let Some(ref op) = self.copy_operation {
                        ui.label(format!("Copying from {} to {}", op.from_id, op.to_id));
                        ui.add(egui::ProgressBar::new(op.progress).text(&op.status));
                    }
                    return;
                }
                AppState::Ready => {
                    if !self.success_message.is_empty() { ui.colored_label(egui::Color32::GREEN, format!("✅ {}", self.success_message)); }
                    if !self.error_message.is_empty() { ui.colored_label(egui::Color32::RED, format!("❌ {}", self.error_message)); }
                }
            }

            if ui.button("Clear Messages").clicked() { self.success_message.clear(); self.error_message.clear(); }
            ui.separator();

            if let Some(ref path) = self.steam_path { ui.label(format!("📁 Steam Path: {}", path.display())); }
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("🔍 Search:");
                ui.text_edit_singleline(&mut self.search_filter);
                ui.checkbox(&mut self.show_only_with_configs, "Only show accounts with CS2 configs");
            });

            ui.separator();

            let filtered_accounts = self.get_filtered_accounts();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.heading("Source Account");
                    ui.label("Select account to copy FROM:");
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (idx, account) in &filtered_accounts {
                            let selected = self.selected_source == Some(*idx);
                            let label = format!("{} {} ({})", if account.has_cs2_config { "✅" } else { "❌" }, account.name.as_deref().unwrap_or("Unknown"), account.id);
                            if ui.selectable_label(selected, &label).clicked() { self.selected_source = Some(*idx); }
                            if account.has_cs2_config && !account.config_files.is_empty() {
                                ui.indent(format!("source_files_{}", idx), |ui| { ui.small(format!("Files: {}", account.config_files.join(", "))); });
                            }
                        }
                    });
                });

                ui.separator();

                ui.vertical(|ui| {
                    ui.heading("Target Account");
                    ui.label("Select account to copy TO:");
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (idx, account) in &filtered_accounts {
                            let selected = self.selected_target == Some(*idx);
                            let label = format!("{} {} ({})", if account.has_cs2_config { "✅" } else { "❌" }, account.name.as_deref().unwrap_or("Unknown"), account.id);
                            if ui.selectable_label(selected, &label).clicked() { self.selected_target = Some(*idx); }
                            if account.has_cs2_config && !account.config_files.is_empty() {
                                ui.indent(format!("target_files_{}", idx), |ui| { ui.small(format!("Files: {}", account.config_files.join(", "))); });
                            }
                        }
                    });
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.create_backup, "Create backup of target config");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_copy = self.selected_source.is_some() && self.selected_target.is_some() && self.selected_source != self.selected_target && matches!(self.state, AppState::Ready);
                    if !can_copy {
                        ui.add_enabled(false, egui::Button::new("🚫 Select different source and target"));
                    } else if ui.button("📋 Copy Configuration").clicked() {
                        let from = self.selected_source.unwrap();
                        let to = self.selected_target.unwrap();
                        self.copy_config_async(from, to, self.create_backup);
                    }
                });
            });

            if let Some(source_idx) = self.selected_source {
                if let Some(account) = self.accounts.get(source_idx) {
                    ui.separator();
                    ui.collapsing("📄 Source Account Details", |ui| {
                        ui.label(format!("ID: {}", account.id));
                        if let Some(ref name) = account.name { ui.label(format!("Name: {}", name)); }
                        ui.label(format!("Has CS2 Config: {}", if account.has_cs2_config { "Yes" } else { "No" }));
                        if let Some(ref path) = account.cs2_config_path { ui.label(format!("Config Path: {}", path.display())); }
                        if !account.config_files.is_empty() {
                            ui.label("Config Files:");
                            for file in &account.config_files { ui.label(format!("  • {}", file)); }
                        }
                    });
                }
            }

            ui.separator();

            // Crosshair Profile Manager
            ui.heading("🎯 Crosshair Profile Manager");
            ui.vertical(|ui| {
                // Crosshair Code Input
                ui.horizontal(|ui| {
                    ui.label("Paste Crosshair Code:");
                    ui.text_edit_singleline(&mut self.crosshair_code_input);
                    if ui.button("Import").clicked() {
                        let code = self.crosshair_code_input.clone();
                        if let Some(mut profile) = self.parse_crosshair_code(&code) {
                            if profile.original_code.is_none() {
                                profile.original_code = Some(code.clone());
                            }
                            self.crosshair_library.push(profile);
                            self.save_crosshair_profiles();
                            self.crosshair_code_input.clear();
                        } else {
                            self.error_message = "Invalid crosshair code".to_string();
                        }
                    }
                });

                // Crosshair Library
                ui.label("Crosshair Library:");
                let profiles: Vec<(usize, CrosshairProfile)> = self.crosshair_library.iter().cloned().enumerate().collect();
                egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    let mut to_delete: Option<usize> = None;
                    for (idx, profile) in profiles.iter() {
                        ui.horizontal(|ui| {
                            let label = format!("{} (R:{}, G:{}, B:{})", profile.name, profile.red, profile.green, profile.blue);
                            if ui.selectable_label(self.selected_library_idx == Some(*idx), &label).clicked() {
                                self.selected_library_idx = Some(*idx);
                                self.active_profile = profile.clone();
                            }
                            if ui.button("🖨 Copy Code").clicked() {
                                let code = self.generate_crosshair_code(profile);
                                ui.output_mut(|o| o.copied_text = code);
                                self.success_message = "Crosshair code copied to clipboard!".to_string();
                            }
                            let mut name = self.crosshair_library[*idx].name.clone();
                            let rename_response = ui.text_edit_singleline(&mut name);
                            if rename_response.changed() {
                                self.crosshair_library[*idx].name = name;
                                self.save_crosshair_profiles();
                            }
                            if ui.button("🗑 Delete").clicked() {
                                to_delete = Some(*idx);
                            }
                        });
                    }
                    if let Some(idx) = to_delete {
                        self.crosshair_library.remove(idx);
                        self.save_crosshair_profiles();
                        self.selected_library_idx = None;
                    }
                });

                if ui.button("➕ Add New Crosshair").clicked() {
                    self.crosshair_library.push(self.active_profile.clone());
                    self.save_crosshair_profiles();
                }

                // Active Profile Editor
                ui.separator();
                ui.label("Active Profile Settings:");
                ui.add(egui::Slider::new(&mut self.active_profile.gap, -12.8..=12.7).text("Gap"));
                ui.add(egui::Slider::new(&mut self.active_profile.outline_thickness, 0.0..=3.0).text("Outline Thickness"));
                ui.add(egui::Slider::new(&mut self.active_profile.red, 0..=255).text("Red"));
                ui.add(egui::Slider::new(&mut self.active_profile.green, 0..=255).text("Green"));
                ui.add(egui::Slider::new(&mut self.active_profile.blue, 0..=255).text("Blue"));
                ui.add(egui::Slider::new(&mut self.active_profile.alpha, 0..=255).text("Alpha"));
                ui.add(egui::Slider::new(&mut self.active_profile.dynamic_splitdist, 0..=127).text("Dynamic Split Dist"));
                ui.checkbox(&mut self.active_profile.recoil, "Recoil");
                ui.add(egui::Slider::new(&mut self.active_profile.fixed_gap, -12.8..=12.7).text("Fixed Gap"));
                ui.add(egui::Slider::new(&mut self.active_profile.color, 0..=5).text("Color"));
                ui.checkbox(&mut self.active_profile.draw_outline, "Draw Outline");
                ui.add(egui::Slider::new(&mut self.active_profile.dynamic_splitalpha_innermod, 0.0..=1.0).text("Dynamic Split Alpha Inner"));
                ui.add(egui::Slider::new(&mut self.active_profile.dynamic_splitalpha_outermod, 0.3..=1.0).text("Dynamic Split Alpha Outer"));
                ui.add(egui::Slider::new(&mut self.active_profile.dynamic_maxdist_split_ratio, 0.0..=1.0).text("Max Dist Split Ratio"));
                ui.add(egui::Slider::new(&mut self.active_profile.thickness, 0.0..=6.3).text("Thickness"));
                ui.add(egui::Slider::new(&mut self.active_profile.style, 0..=5).text("Style"));
                ui.checkbox(&mut self.active_profile.dot, "Dot");
                ui.checkbox(&mut self.active_profile.gap_use_weapon_value, "Gap Use Weapon Value");
                ui.checkbox(&mut self.active_profile.use_alpha, "Use Alpha");
                ui.checkbox(&mut self.active_profile.t, "T-Style");
                ui.add(egui::Slider::new(&mut self.active_profile.size, 0.0..=819.1).text("Size"));

                // Crosshair Preview
                ui.separator();
                ui.label("Crosshair Preview:");
                let painter = ui.painter();
                let rect = ui.available_rect_before_wrap();
                let center = rect.center();

                // Scaling factor to match CS2's pixel-based rendering (assuming 1920x1080 as reference)
                const SCALE_FACTOR: f32 = 2.0; // Maps cl_crosshairsize 1.0 to ~10 pixels
                let size = self.active_profile.size * SCALE_FACTOR;
                let thickness = (self.active_profile.thickness * SCALE_FACTOR).max(1.0); // Ensure minimum thickness for visibility
                let gap = if self.active_profile.gap_use_weapon_value && self.active_profile.fixed_gap != 0.0 {
                    self.active_profile.fixed_gap * SCALE_FACTOR
                } else {
                    self.active_profile.gap * SCALE_FACTOR
                };
                let color = egui::Color32::from_rgba_unmultiplied(
                    self.active_profile.red,
                    self.active_profile.green,
                    self.active_profile.blue,
                    if self.active_profile.use_alpha { self.active_profile.alpha } else { 255 },
                );

                // Adjust rendering based on crosshair style
                match self.active_profile.style {
                    // Classic Static (style 4) or similar
                    2 | 3 | 4 | 5 => {
                        if !self.active_profile.t {
                            // Standard crosshair: four lines
                            painter.line_segment(
                                [center + egui::vec2(-size - gap, 0.0), center + egui::vec2(-gap, 0.0)],
                                (thickness, color),
                            );
                            painter.line_segment(
                                [center + egui::vec2(gap, 0.0), center + egui::vec2(size + gap, 0.0)],
                                (thickness, color),
                            );
                            painter.line_segment(
                                [center + egui::vec2(0.0, -size - gap), center + egui::vec2(0.0, -gap)],
                                (thickness, color),
                            );
                            painter.line_segment(
                                [center + egui::vec2(0.0, gap), center + egui::vec2(0.0, size + gap)],
                                (thickness, color),
                            );
                        } else {
                            // T-style: horizontal line and vertical line starting from gap
                            painter.line_segment(
                                [center + egui::vec2(-size, 0.0), center + egui::vec2(size, 0.0)],
                                (thickness, color),
                            );
                            painter.line_segment(
                                [center + egui::vec2(0.0, gap), center + egui::vec2(0.0, size + gap)],
                                (thickness, color),
                            );
                        }
                    }
                    // Dot-only or other styles
                    _ => {
                        // For simplicity, render a dot if style doesn't support lines
                        if self.active_profile.dot {
                            let dot_size = thickness * 0.5;
                            painter.circle_filled(center, dot_size, color);
                        }
                    }
                }

                // Draw dot if enabled
                if self.active_profile.dot {
                    let dot_size = thickness * 0.5; // CS2 dot is typically half the thickness
                    painter.circle_filled(center, dot_size, color);
                }

                // Draw outline if enabled
                if self.active_profile.draw_outline {
                    let outline_thickness = (self.active_profile.outline_thickness * SCALE_FACTOR).max(1.0);
                    let outline_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, if self.active_profile.use_alpha { self.active_profile.alpha } else { 255 });
                    let offset = thickness * 0.5 + outline_thickness * 0.5; // Tighten outline to hug lines

                    match self.active_profile.style {
                        2 | 3 | 4 | 5 => {
                            if !self.active_profile.t {
                                // Outline for standard crosshair
                                painter.line_segment(
                                    [center + egui::vec2(-size - gap - offset, 0.0), center + egui::vec2(-gap + offset, 0.0)],
                                    (outline_thickness, outline_color),
                                );
                                painter.line_segment(
                                    [center + egui::vec2(gap - offset, 0.0), center + egui::vec2(size + gap + offset, 0.0)],
                                    (outline_thickness, outline_color),
                                );
                                painter.line_segment(
                                    [center + egui::vec2(0.0, -size - gap - offset), center + egui::vec2(0.0, -gap + offset)],
                                    (outline_thickness, outline_color),
                                );
                                painter.line_segment(
                                    [center + egui::vec2(0.0, gap - offset), center + egui::vec2(0.0, size + gap + offset)],
                                    (outline_thickness, outline_color),
                                );
                            } else {
                                // Outline for T-style
                                painter.line_segment(
                                    [center + egui::vec2(-size - offset, 0.0), center + egui::vec2(size + offset, 0.0)],
                                    (outline_thickness, outline_color),
                                );
                                painter.line_segment(
                                    [center + egui::vec2(0.0, gap - offset), center + egui::vec2(0.0, size + gap + offset)],
                                    (outline_thickness, outline_color),
                                );
                            }
                        }
                        _ => {}
                    }

                    // Outline for dot
                    if self.active_profile.dot {
                        let dot_size = thickness * 0.5;
                        painter.circle(center, dot_size + outline_thickness * 0.5, outline_color, (outline_thickness, outline_color));
                    }
                }

                if let Some(target_idx) = self.selected_target {
                    if let Some(account) = self.accounts.get(target_idx) {
                        if let Some(config_path) = &account.cs2_config_path {
                            let config_file = config_path.join("config.cfg");
                            if ui.button("Apply to Config").clicked() {
                                self.apply_crosshair_to_config(&self.active_profile, &config_file);
                                self.success_message = "Crosshair applied to config!".to_string();
                            }
                        }
                    }
                }
            });

            ui.small("💡 Tip: Make sure CS2 is closed before applying configurations.");
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([600.0, 400.0])
            .with_icon(eframe::icon_data::from_png_bytes(&[]).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native(
        "CS2 Config Manager",
        options,
        Box::new(|cc| Ok(Box::new(CS2ConfigApp::new(cc)))),
    )
}