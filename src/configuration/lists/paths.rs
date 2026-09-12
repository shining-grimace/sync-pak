use std::path::Path;

pub fn relative_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Ok(());
    }
    if path.contains(['\\', ':', '\0'])
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(
            "Use a relative path with / separators and no empty, . or .. components.".into(),
        );
    }
    Ok(())
}

pub fn local_path(root: &str, relative: &str) -> Result<String, String> {
    relative_path(relative)?;
    if root.starts_with("content://") {
        // A fragment carries a descendant path; Android resolves names through DocumentsContract.
        if root.contains('#') {
            return Err("Select a document tree without a descendant fragment.".into());
        }
        return Ok(if relative.is_empty() {
            root.into()
        } else {
            format!("{root}#{relative}")
        });
    }
    if !Path::new(root).is_absolute() {
        return Err("Choose an absolute local folder for this root.".into());
    }
    let joined = if relative.is_empty() {
        Path::new(root).to_path_buf()
    } else {
        Path::new(root).join(relative)
    };
    if let Ok(base) = std::fs::canonicalize(root) {
        let mut ancestor = joined.as_path();
        loop {
            if let Ok(destination) = std::fs::canonicalize(ancestor) {
                if !destination.starts_with(&base) {
                    return Err("The local path escapes its selected root.".into());
                }
                break;
            }
            let Some(parent) = ancestor.parent() else {
                break;
            };
            ancestor = parent;
        }
    }
    joined
        .to_str()
        .map(String::from)
        .ok_or_else(|| "Local path is not UTF-8.".into())
}

pub fn local_relative(root: &str, full: &str) -> Option<String> {
    if root == full {
        return Some(String::new());
    }
    if root.starts_with("content://") {
        return full.strip_prefix(&format!("{root}#")).map(String::from);
    }
    let value = Path::new(full)
        .strip_prefix(root)
        .ok()?
        .to_str()?
        .replace('\\', "/");
    relative_path(&value).ok()?;
    Some(value)
}

/// Initial root is the platform home directory; Android users can select a granted tree.
pub fn default_local_root() -> String {
    #[cfg(target_os = "windows")]
    let home = std::env::var_os("USERPROFILE");
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var_os("HOME");
    home.and_then(|p| p.into_string().ok())
        .filter(|p| Path::new(p).is_absolute())
        .unwrap_or_else(|| {
            #[cfg(target_os = "android")]
            {
                "/storage/emulated/0".into()
            }
            #[cfg(not(target_os = "android"))]
            {
                std::env::temp_dir().to_string_lossy().into_owned()
            }
        })
}
