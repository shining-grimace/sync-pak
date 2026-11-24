mod preview;

use std::{sync::mpsc, time::Duration};

use preview::{Preview, load_local, load_remote, preview_type, text_file};

use slint::{ComponentHandle, Image, Rgba8Pixel, SharedPixelBuffer};

use crate::{
    AppWindow, FileVersion,
    inventory::{InventoryEntry, InventoryEntryKind, remote::normalize_prefix},
    operations::transfer::paths::LocalTransferRoot,
};

const TEXT_LIMIT: u64 = 1_000_000;

pub(super) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_close_comparison(move || {
        if let Some(window) = weak.upgrade() {
            window.set_comparison_open(false);
            window.set_comparison_local(Default::default());
            window.set_comparison_remote(Default::default());
        }
    });
    let weak = window.as_weak();
    window.on_compare_file(move |path| {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 11
            || window.get_comparison_open()
            || window.get_confirmation() != crate::ConfirmationKind::None
        {
            return;
        }
        let Some((request, local, remote)) = super::mirror_controller::comparison_target(&path)
        else {
            return;
        };
        let root = LocalTransferRoot::from_config(&request.connection.local_path);
        let Ok(prefix) = normalize_prefix(&request.connection.remote_path) else {
            return;
        };
        let local_path = local.as_ref().map(|entry| {
            root.native_path(&entry.path)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| {
                    format!(
                        "{}/{}",
                        request.connection.local_path.trim_end_matches('/'),
                        entry.path.as_str()
                    )
                })
        });
        let remote_path = remote.as_ref().map(|entry| {
            format!(
                "s3://{}/{}{}",
                request.connection.bucket,
                prefix,
                entry.path.as_str()
            )
        });
        window.set_comparison_local(version(local.as_ref(), local_path));
        window.set_comparison_remote(version(remote.as_ref(), remote_path));
        window.set_comparison_generation(window.get_comparison_generation().wrapping_add(1));
        window.set_comparison_open(true);
        if let Some(entry) = local.filter(|entry| preview_type(entry).is_some()) {
            start(&window, true, move || load_local(&root, &entry));
        }
        if let Some(entry) = remote.filter(|entry| preview_type(entry).is_some()) {
            start(&window, false, move || {
                load_remote(&request, &prefix, &entry)
            });
        }
    });
}

fn version(entry: Option<&InventoryEntry>, path: Option<String>) -> FileVersion {
    let mut version = FileVersion::default();
    version.message = match entry {
        None => "Not present on this side",
        Some(entry) if preview_type(entry).is_some() => "Loading preview…",
        Some(entry) if entry.kind == InventoryEntryKind::Directory => "Folder",
        Some(entry) if text_file(entry.path.as_str()) && entry.byte_size >= TEXT_LIMIT => {
            "Text preview requires a file under 1 MB"
        }
        Some(_) => "Preview unavailable",
    }
    .into();
    if let Some(entry) = entry {
        version.path = path.unwrap_or_default().into();
        if entry.kind == InventoryEntryKind::File {
            version.size = format!("{} bytes", entry.byte_size).into();
        }
    }
    version
}

fn start(window: &AppWindow, local: bool, load: impl FnOnce() -> Option<Preview> + Send + 'static) {
    let ticket = window.get_comparison_generation();
    let weak = window.as_weak();
    let (sender, receiver) = mpsc::sync_channel(1);
    let _ = std::thread::Builder::new()
        .name("file-preview".into())
        .spawn(move || {
            let _ = sender.send(load());
        });
    poll(weak, local, ticket, receiver);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    local: bool,
    ticket: i32,
    receiver: mpsc::Receiver<Option<Preview>>,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if !window.get_comparison_open()
            || window.get_page() != 11
            || window.get_comparison_generation() != ticket
        {
            return;
        }
        let mut version = if local {
            window.get_comparison_local()
        } else {
            window.get_comparison_remote()
        };
        match receiver.try_recv() {
            Ok(Some(Preview::Text(text))) => {
                version.has_text = true;
                version.preview_text = text.into();
            }
            Ok(Some(Preview::Image(image))) => {
                version.preview_image =
                    Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
                        image.as_raw(),
                        image.width(),
                        image.height(),
                    ));
            }
            Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                version.message = "Preview unavailable".into()
            }
            Err(mpsc::TryRecvError::Empty) => {
                poll(weak, local, ticket, receiver);
                return;
            }
        }
        if local {
            window.set_comparison_local(version);
        } else {
            window.set_comparison_remote(version);
        }
    });
}
