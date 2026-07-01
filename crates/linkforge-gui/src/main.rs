#![cfg_attr(windows, windows_subsystem = "windows")]

mod backend;

fn main() {
    let launch_context = backend::LaunchContext::from_env();
    if backend::handle_direct_context_action(&launch_context) {
        return;
    }

    tauri::Builder::default()
        .manage(launch_context)
        .setup(|app| {
            backend::configure_initial_window(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend::initial_context,
            backend::show_drop_window,
            backend::close_drop_window,
            backend::expand_to_full_window,
            backend::prepare_direct_drop,
            backend::create_direct_link_step,
            backend::list_directory,
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
