use serde::Serialize;

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BizHawkConfig {
    pub preferred_cores: PreferredCores,
    pub pause_when_menu_activated: bool,
    pub save_window_position: bool,
    pub start_paused: bool,
    pub start_fullscreen: bool,
    
    /// A string containing x and y desktop coordinates seperated by a comma and space.
    /// 
    /// **Example:** `"123, 456"`
    pub main_window_position: Option<String>,
    
    /// A string containing width and height pixel values seperated by a comma and space.
    /// 
    /// **Example:** `"640, 480"`
    pub main_window_size: Option<String>,
    pub main_window_maximized: bool,
    pub single_instance_mode: bool,
    pub first_boot: bool,
    pub update_latest_version: String,
    pub last_written_from: String,
    pub last_written_from_detailed: String,
    pub lua_engine: usize, //TODO: Create enum for this
    pub selected_profile: usize, //TODO: Create enum for this
    pub speed_percent: usize,
    pub speed_percent_alternate: usize,
    pub movies: Option<Movies>,
    pub display_fps: bool,
    pub display_frame_counter: bool,
    pub display_lag_counter: bool,
    pub display_input: bool,
    pub display_rerecord_count: bool,
    pub display_messages: bool,
    pub sound_enabled: bool,
    pub core_sync_settings: CoreSyncSettings,
}
impl BizHawkConfig {
    /// Create a new BizHawk config with preset values for use in VeriTAS.
    pub fn veritas(emu_version: &str) -> Self {
        Self {
            pause_when_menu_activated: true,
            save_window_position: false,
            start_paused: false,
            start_fullscreen: false,
            single_instance_mode: false,
            first_boot: false,
            update_latest_version: emu_version.to_string(),
            last_written_from: emu_version.to_string(),
            last_written_from_detailed: format!("Version {emu_version}"),
            lua_engine: 1,
            selected_profile: 3,
            speed_percent: 400,
            speed_percent_alternate: 400,
            movies: Some(Movies {
                movie_end_action: 3,
            }),
            display_fps: true,
            display_frame_counter: true,
            display_lag_counter: true,
            display_input: true,
            display_rerecord_count: false,
            display_messages: true,
            sound_enabled: false,
            
            ..Default::default()
        }
    }
    
    pub fn to_vec(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("should be serializable")
    }
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct PreferredCores {
    nes: Option<String>,
    snes: Option<String>,
    n64: Option<String>,
    gb: Option<String>,
    gbc: Option<String>,
    gbl: Option<String>,
    sgb: Option<String>,
    pce: Option<String>,
    pcecd: Option<String>,
    sgx: Option<String>,
    sgxcd: Option<String>,
    psx: Option<String>,
    ti83: Option<String>,
    a26: Option<String>,
    bsx: Option<String>,
    #[serde(rename = "GEN")]
    genesis: Option<String>,
    sms: Option<String>,
    gg: Option<String>,
    sg: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Movies {
    pub movie_end_action: usize, //TODO: Create enum for this
}

#[derive(Debug, Default, Serialize)]
pub struct CoreSyncSettings {
    #[serde(rename = "BizHawk.Emulation.Cores.Atari.Atari2600.Atari2600")]
    atari2600: Atari2600SyncSettings,
    #[serde(rename = "BizHawk.Emulation.Cores.Atari.Stella.Stella")]
    stella: StellaSyncSettings,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Atari2600SyncSettings {
    #[serde(rename = "$type")]
    _type: String,
    port1: usize,
    port2: usize,
    #[serde(rename = "BW")]
    bw: bool,
    left_difficulty: bool,
    right_difficulty: bool,
    fast_sc_bios: bool,
}
impl Default for Atari2600SyncSettings {
    fn default() -> Self {
        Self {
            _type: "BizHawk.Emulation.Cores.Atari.Atari2600.Atari2600+A2600SyncSettings, BizHawk.Emulation.Cores".into(),
            port1: 1,
            port2: 1,
            bw: false,
            left_difficulty: true,
            right_difficulty: true,
            fast_sc_bios: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct StellaSyncSettings {
    #[serde(rename = "$type")]
    _type: String,
    port1: usize,
    port2: usize,
}
impl Default for StellaSyncSettings {
    fn default() -> Self {
        Self {
            _type: "BizHawk.Emulation.Cores.Atari.Stella.Stella+A2600SyncSettings, BizHawk.Emulation.Cores".into(),
            port1: 1,
            port2: 1,
        }
    }
}