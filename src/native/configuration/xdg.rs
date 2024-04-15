use std::{env, fs::create_dir_all, path::PathBuf};
const HOME_ERROR: &str = "cant find home directory";
const GLOBAL_CONFIG_NAME: &str = "config.json";
const PRIMARY_USER_KEY_NAME: &str = "primary.pem";

/// Grab config path
pub fn xdg_config_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.config/banyan",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect("failed to create XDG config home");
    }
    // Return
    path
}

/// Grab data path
pub fn xdg_data_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.local/share/banyan",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect("failed to create XDG data home");
    }
    // Return
    path
}

/// Grab path to config.json File
pub fn config_path() -> PathBuf {
    xdg_config_home().join(GLOBAL_CONFIG_NAME)
}

/// Grab path to API Key
pub fn default_user_key_path() -> PathBuf {
    xdg_data_home().join("keys").join(PRIMARY_USER_KEY_NAME)
}
