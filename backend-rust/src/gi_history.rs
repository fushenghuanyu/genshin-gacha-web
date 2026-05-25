//! 从 `resources/gacha/` 下的 `character.json`、`weapon.json` 加载历史祈愿，并转为前端沿用的卡池行格式。

use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;

use crate::paths;

#[derive(Debug, Deserialize)]
struct GiPoolBlock {
    version: String,
    items: Vec<GiItem>,
    start: String,
    end: String,
}

#[derive(Debug, Deserialize)]
struct GiItem {
    name: String,
    #[serde(rename = "rankType")]
    rank_type: u8,
}

/// 将 `resources/gacha` 下的角色/武器卡池 JSON 合并为前端 UP 判定所需的卡池行数组（供 `/api/history`）。
pub fn load_history_rows(project_root: &Path) -> Result<Vec<Value>, String> {
    let gacha_dir = paths::gacha_dir(project_root);
    let mut rows = Vec::new();

    let char_path = gacha_dir.join("character.json");
    if char_path.is_file() {
        let blocks = read_pool_blocks(&char_path)?;
        rows.extend(blocks_to_rows(&blocks, "角色"));
    }

    let weapon_path = gacha_dir.join("weapon.json");
    if weapon_path.is_file() {
        let blocks = read_pool_blocks(&weapon_path)?;
        rows.extend(blocks_to_rows(&blocks, "武器"));
    }

    if rows.is_empty() {
        return Err(format!(
            "未找到历史卡池数据（期望 {} 或 {}）",
            char_path.display(),
            weapon_path.display()
        ));
    }

    Ok(rows)
}

fn read_pool_blocks(path: &Path) -> Result<Vec<GiPoolBlock>, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("读取 {}: {e}", path.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("解析 {}: {e}", path.display()))
}

fn blocks_to_rows(blocks: &[GiPoolBlock], pool_type: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    for block in blocks {
        let timer = format_timer(&block.start, &block.end);
        let fours: Vec<&str> = block
            .items
            .iter()
            .filter(|i| i.rank_type == 4)
            .map(|i| i.name.as_str())
            .collect();
        let a: Vec<&str> = fours;

        for item in block.items.iter().filter(|i| i.rank_type == 5) {
            rows.push(json!({
                "img": "",
                "title": "",
                "type": pool_type,
                "version": block.version,
                "timer": timer,
                "s": item.name,
                "a": a,
                "img_path": ""
            }));
        }
    }
    rows
}

/// `2026-05-13 12:00:00` → `2026/05/13 12:00`（与 `poolHistory.js` 的 `parseDateTime` 一致）
fn format_datetime_part(s: &str) -> Option<String> {
    let s = s.trim();
    if s.len() < 16 {
        return None;
    }
    let date = s.get(0..10)?.replace('-', "/");
    let time = s.get(11..16)?;
    Some(format!("{date} {time}"))
}

fn format_timer(start: &str, end: &str) -> String {
    let left = format_datetime_part(start).unwrap_or_else(|| start.trim().to_string());
    let right = format_datetime_part(end).unwrap_or_else(|| end.trim().to_string());
    format!("{left} ~ {right}")
}
