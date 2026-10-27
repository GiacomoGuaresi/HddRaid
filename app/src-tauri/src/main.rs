// Evita la finestra della console su Windows nelle build di release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    hddraid_app_lib::run()
}
