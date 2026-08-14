#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    dsh_agent_lib::run();
}
