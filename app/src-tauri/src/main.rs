// Windows のリリースビルドでコンソールを出さない
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    mondeep_app_lib::run()
}
