use std::path::{Path, PathBuf};

/// 静态资源目录（`icon/`、`gacha/`、`dict/` 等）相对项目根的路径。
pub fn resources_dir(root: &Path) -> PathBuf {
    root.join("resources")
}

/// 历史卡池 JSON 目录（`resources/gacha/`）。
pub fn gacha_dir(root: &Path) -> PathBuf {
    resources_dir(root).join("gacha")
}

/// 角色/武器字典 JSON 目录（`resources/dict/`）。
pub fn dict_dir(root: &Path) -> PathBuf {
    resources_dir(root).join("dict")
}

fn looks_like_project_root(p: &Path) -> bool {
    let res = resources_dir(p);
    res.join("gacha").join("character.json").is_file()
        || res.join("icon").is_dir()
        || p.join("dist").join("index.html").is_file()
}

fn is_exe_from_this_crate_target(exe: &Path) -> bool {
    let Ok(manifest_dir) = PathBuf::from(env!("CARGO_MANIFEST_DIR")).canonicalize() else {
        return false;
    };
    let target_root = manifest_dir.join("target");
    let Ok(canon) = exe.canonicalize() else {
        return false;
    };
    canon.starts_with(&target_root)
}

/// 项目根下应有 `resources/`（含 `gacha/` 历史卡池、`dict/` 字典、`icon/`）、前端构建产物 `dist/`。
/// `userData/` 默认在 `{project_root}/userData/`，可用环境变量 `GACHA_USER_DATA_DIR` 覆盖。
pub fn project_root() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let base = match dir.canonicalize() {
                Ok(c) => c,
                Err(_) => dir.to_path_buf(),
            };
            if is_exe_from_this_crate_target(&exe) {
                if looks_like_project_root(&base) {
                    return base;
                }
            } else {
                return base;
            }
        }
    }
    if let Some(p) = try_manifest_parent() {
        if looks_like_project_root(&p) {
            return p;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for base in [dir.to_path_buf(), dir.join(".."), dir.join("../..")] {
                let base = base.canonicalize().unwrap_or(base);
                if looks_like_project_root(&base) {
                    return base;
                }
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        if looks_like_project_root(&cwd) {
            return cwd;
        }
    }
    try_manifest_parent().unwrap_or_else(|| PathBuf::from("."))
}

fn try_manifest_parent() -> Option<PathBuf> {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(std::path::Path::to_path_buf)
}

/// 用户数据目录。未设置 `GACHA_USER_DATA_DIR` 时为 `{project_root}/userData/`。
pub fn user_data_dir(project_root: &Path) -> PathBuf {
    const ENV: &str = "GACHA_USER_DATA_DIR";
    if let Ok(raw) = std::env::var(ENV) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    project_root.join("userData")
}
