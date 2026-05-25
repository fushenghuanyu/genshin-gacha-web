//! 从 `resources/dict/` 米游社字典 + `resources/gacha/` 卡池补全名称 → item_id，供 `/icon` 与前端展示使用。

use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use crate::paths;

static LOOKUP: OnceLock<ItemLookup> = OnceLock::new();

#[derive(Debug, Default, Clone)]
pub struct ItemLookup {
    character: HashMap<String, String>,
    weapon: HashMap<String, String>,
}

impl ItemLookup {
    pub fn global(project_root: &Path) -> &'static ItemLookup {
        LOOKUP.get_or_init(|| ItemLookup::load(project_root))
    }

    pub fn load(project_root: &Path) -> Self {
        let mut lookup = ItemLookup::default();
        let dict = paths::dict_dir(project_root);
        lookup.merge_miyoushe_dict_file(&dict.join("character.json"), "角色");
        lookup.merge_miyoushe_dict_file(&dict.join("weapon.json"), "武器");
        let gacha = paths::gacha_dir(project_root);
        lookup.merge_pool_file(&gacha.join("character.json"), "角色");
        lookup.merge_pool_file(&gacha.join("weapon.json"), "武器");
        tracing::info!(
            "item_lookup 已加载：角色 {} 条，武器 {} 条",
            lookup.character.len(),
            lookup.weapon.len()
        );
        lookup
    }

    pub fn as_json(&self) -> Value {
        json!({
            "角色": self.character,
            "武器": self.weapon,
        })
    }

    fn merge_miyoushe_dict_file(&mut self, path: &Path, item_type: &str) {
        let Ok(s) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(data) = serde_json::from_str::<Value>(&s) else {
            return;
        };
        let Some(obj) = data.as_object() else {
            return;
        };
        let map = if item_type == "角色" {
            &mut self.character
        } else {
            &mut self.weapon
        };
        for (key, item) in obj {
            let Some(item) = item.as_object() else {
                continue;
            };
            let name = item
                .get("cn")
                .and_then(|v| v.as_str())
                .unwrap_or(key.as_str())
                .trim();
            if name.is_empty() {
                continue;
            }
            let id = item.get("id").and_then(value_to_id_string);
            if let Some(id) = id {
                map.insert(name.to_string(), id);
            }
        }
    }

    fn merge_pool_file(&mut self, path: &Path, item_type: &str) {
        let Ok(s) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(blocks) = serde_json::from_str::<Vec<Value>>(&s) else {
            return;
        };
        let map = if item_type == "角色" {
            &mut self.character
        } else {
            &mut self.weapon
        };
        for block in blocks {
            let Some(items) = block.get("items").and_then(|x| x.as_array()) else {
                continue;
            };
            for item in items {
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name.is_empty() {
                    continue;
                }
                let id = item
                    .get("itemId")
                    .map(|v| match v {
                        Value::Number(n) => n.to_string(),
                        Value::String(s) => s.clone(),
                        _ => String::new(),
                    })
                    .unwrap_or_default();
                if !id.is_empty() {
                    map.insert(name.to_string(), id);
                }
            }
        }
    }

    pub fn resolve(&self, item_type: &str, name: &str) -> Option<String> {
        let name = name.trim();
        if name.is_empty() {
            return None;
        }
        match item_type.trim() {
            "角色" => self.character.get(name).cloned(),
            "武器" => self.weapon.get(name).cloned(),
            _ => None,
        }
    }

    pub fn enrich_record(&self, item: &mut Value) {
        let empty = item
            .get("item_id")
            .map(|v| v.as_str().unwrap_or("").trim().is_empty())
            .unwrap_or(true);
        if !empty {
            return;
        }
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let item_type = item.get("item_type").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(id) = self.resolve(item_type, name) {
            if let Some(obj) = item.as_object_mut() {
                obj.insert("item_id".into(), json!(id));
            }
        }
    }

    pub fn enrich_records(&self, records: &mut [Value]) {
        for item in records.iter_mut() {
            self.enrich_record(item);
        }
    }

    pub fn enrich_bootstrap(&self, root: &mut Value) {
        let Some(accounts) = root.get_mut("accounts").and_then(|v| v.as_object_mut()) else {
            return;
        };
        for (_, acc) in accounts.iter_mut() {
            if let Some(records) = acc
                .get_mut("result")
                .and_then(|r| r.get_mut("records"))
                .and_then(|r| r.as_array_mut())
            {
                self.enrich_records(records);
            }
        }
    }

    pub fn enrich_persist_body(&self, body: &mut crate::user_data::PersistBody) {
        for (_, acc) in body.accounts.iter_mut() {
            if let Some(records) = acc
                .get_mut("result")
                .and_then(|r| r.get_mut("records"))
                .and_then(|r| r.as_array_mut())
            {
                self.enrich_records(records);
            }
        }
    }
}

fn value_to_id_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
