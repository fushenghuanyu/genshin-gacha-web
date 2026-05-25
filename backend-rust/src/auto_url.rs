use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static URL_PATTERN: OnceLock<Regex> = OnceLock::new();
static GAME_PATH_PATTERN: OnceLock<Regex> = OnceLock::new();
static LOG_URL_PATTERN: OnceLock<Regex> = OnceLock::new();

fn url_pattern() -> &'static Regex {
    URL_PATTERN.get_or_init(|| {
        Regex::new(
            r"https.+?auth_appid=webview_gacha.+?authkey=.+?game_biz=hk4e_\w+",
        )
        .expect("url")
    })
}
fn game_path_pattern() -> &'static Regex {
    GAME_PATH_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)[A-Za-z]:[/\\].+?(GenshinImpact_Data|YuanShen_Data)")
            .expect("g")
    })
}
fn log_url_pattern() -> &'static Regex {
    LOG_URL_PATTERN.get_or_init(|| Regex::new(r"web:\s*\d+\s*url:\s*(https[^\s]+)").expect("l"))
}

fn normalize_game_path(raw: &str) -> PathBuf {
    let normalized = raw.replace('/', "\\");
    PathBuf::from(normalized)
}

fn candidate_log_paths() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    let base = home.join("AppData").join("LocalLow");
    vec![
        base.join("miHoYo").join("原神").join("output_log.txt"),
        base.join("miHoYo")
            .join("Genshin Impact")
            .join("output_log.txt"),
        base.join("Cognosphere")
            .join("Genshin Impact")
            .join("output_log.txt"),
    ]
}

/// 与 genshin-wish-export 一致：在 webCaches 下查找最新的 `data_2` 缓存文件。
fn find_latest_data2_cache(game_path: &Path) -> Option<PathBuf> {
    let web_caches = game_path.join("webCaches");
    if !web_caches.is_dir() {
        return None;
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    let direct = web_caches
        .join("Cache")
        .join("Cache_Data")
        .join("data_2");
    if direct.is_file() {
        candidates.push(direct);
    }

    if let Ok(rd) = std::fs::read_dir(&web_caches) {
        for entry in rd.flatten() {
            let data2 = entry
                .path()
                .join("Cache")
                .join("Cache_Data")
                .join("data_2");
            if data2.is_file() {
                candidates.push(data2);
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }
    candidates
        .into_iter()
        .max_by_key(|p| std::fs::metadata(p).ok().and_then(|m| m.modified().ok()))
}

fn extract_latest_url_from_cache_text(cache_text: &str) -> Option<String> {
    let v: Vec<String> = url_pattern()
        .find_iter(cache_text)
        .map(|m| m.as_str().to_string())
        .collect();
    v.last().cloned()
}

fn extract_latest_url_from_log(log_text: &str) -> Option<String> {
    let line_ms: Vec<String> = log_url_pattern()
        .captures_iter(log_text)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect();
    for url in line_ms.iter().rev() {
        if url.contains("auth_appid=webview_gacha") && url.contains("authkey=") {
            return Some(url.clone());
        }
    }
    let all: Vec<String> = url_pattern()
        .find_iter(log_text)
        .map(|m| m.as_str().to_string())
        .collect();
    all.last().cloned()
}

pub fn get_wish_url_from_local_logs<F: FnMut(&str)>(mut log: F) -> Result<String, String> {
    let log_paths: Vec<_> = candidate_log_paths()
        .into_iter()
        .filter(|p| p.exists())
        .collect();
    log(&format!("检测到日志文件数量: {}", log_paths.len()));
    if log_paths.is_empty() {
        return Err("未找到游戏日志文件，请先打开一次游戏并进入祈愿历史页面。".to_string());
    }

    for log_path in log_paths {
        log(&format!("读取日志: {}", log_path.display()));
        let log_bytes = match std::fs::read(&log_path) {
            Ok(b) => b,
            Err(_) => {
                log(&format!("读取日志失败: {}", log_path.display()));
                continue;
            }
        };
        let log_text = String::from_utf8_lossy(&log_bytes).into_owned();

        if let Some(url) = extract_latest_url_from_log(&log_text) {
            log("已从日志中直接提取到抽卡链接");
            return Ok(url);
        }

        let Some(m) = game_path_pattern().find(&log_text) else {
            log("日志中未找到游戏 Data 目录路径，跳过当前日志");
            continue;
        };

        let game_path = normalize_game_path(m.as_str());
        log(&format!("定位游戏目录: {}", game_path.display()));
        let Some(cache_file) = find_latest_data2_cache(&game_path) else {
            log("未找到 data_2 缓存文件");
            continue;
        };

        log(&format!("读取缓存文件: {}", cache_file.display()));
        let cache_bytes = match std::fs::read(&cache_file) {
            Ok(b) => b,
            Err(_) => {
                log("读取缓存失败");
                continue;
            }
        };
        let cache_text = String::from_utf8_lossy(&cache_bytes).into_owned();
        if let Some(url) = extract_latest_url_from_cache_text(&cache_text) {
            log("已从缓存中提取到抽卡链接");
            return Ok(url);
        }
    }
    Err("已读取日志，但未找到可用抽卡链接。请在游戏内重新打开祈愿历史后重试。".to_string())
}
