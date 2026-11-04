mod engine;
#[cfg(target_os = "linux")]
mod mpris;

use crossbeam_channel::Sender;
use tauri::{Manager, State};
use valser_core::store::Store;

use engine::Cmd;

#[tauri::command]
fn send(cmd: Cmd, tx: State<Sender<Cmd>>) {
    let _ = tx.send(cmd);
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn pick(folder: bool, tx: State<'_, Sender<Cmd>>) -> Result<(), ()> {
    let dialog = rfd::AsyncFileDialog::new();
    let picked = if folder {
        dialog.pick_folder().await.map(|f| vec![f])
    } else {
        dialog
            .add_filter("Audio", &["mp3", "ogg", "opus", "flac", "wav", "m4a", "aac", "aiff"])
            .pick_files()
            .await
    };
    if let Some(files) = picked {
        let paths = files.iter().map(|f| f.path().to_path_buf()).collect();
        let _ = tx.send(Cmd::Add { paths });
    }
    Ok(())
}

#[cfg(target_os = "android")]
mod browse {
    use serde::Serialize;
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};
    use valser_core::playlist::is_supported_format;

    #[derive(Serialize)]
    pub struct Entry {
        path: PathBuf,
        name: String,
        dir: bool,
    }

    fn entry(path: PathBuf, name: String) -> Entry {
        Entry { dir: path.is_dir(), path, name }
    }

    /// Internal storage and every mounted volume such as SD cards.
    /// Apps cannot list /storage, so volumes are read from the mount table.
    fn roots() -> Vec<Entry> {
        let mounts = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
        let volumes: BTreeSet<&str> = mounts
            .lines()
            .filter_map(|line| line.split(' ').nth(4)?.strip_prefix("/storage/"))
            .filter(|name| !name.contains('/') && *name != "emulated" && *name != "self")
            .collect();

        let mut roots = vec![entry("/storage/emulated/0".into(), "Internal storage".into())];
        roots.extend(volumes.into_iter().map(|name| entry(format!("/storage/{name}").into(), name.into())));
        roots
    }

    #[tauri::command]
    pub fn list_dir(path: Option<PathBuf>) -> Vec<Entry> {
        let Some(path) = path else { return roots() };
        let mut entries: Vec<Entry> = std::fs::read_dir(&path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| entry(e.path(), e.file_name().to_string_lossy().into_owned()))
            .filter(|e| !e.name.starts_with('.') && (e.dir || is_supported_format(Path::new(&e.name))))
            .collect();
        entries.sort_by(|a, b| b.dir.cmp(&a.dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        entries
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Avoids a blank window with WebKitGTK on some Linux GPU drivers
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        unsafe { std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1") };
    }

    tauri::Builder::default()
        .setup(|app| {
            let path = app.path();
            let store = Store::open(&path.data_dir()?.join("Valser"), &path.config_dir()?.join("Valser"))?;
            app.manage(engine::spawn(app.handle().clone(), store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send,
            #[cfg(not(target_os = "android"))]
            pick,
            #[cfg(target_os = "android")]
            browse::list_dir,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
