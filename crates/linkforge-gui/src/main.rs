mod backend;

fn main() {
    let launch_context = backend::LaunchContext::from_env();
    if backend::handle_direct_context_action(&launch_context) {
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(launch_context)
        .invoke_handler(tauri::generate_handler![
            backend::initial_context,
            backend::create_symlink,
            backend::create_hardlink,
            backend::same_file,
            backend::link_count,
            backend::siblings,
            backend::scan_groups,
            backend::clone_tree,
            backend::reveal_path,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run LinkForge GUI");
}
