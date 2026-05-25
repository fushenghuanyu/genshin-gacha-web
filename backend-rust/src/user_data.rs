use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

const META_FILE: &str = "accounts_cache.json";
const HISTORY_FILE: &str = "read_history.json";
const RECORDS_SUBDIR: &str = "records";

fn records_dir(user_data_root: &Path) -> PathBuf {
    user_data_root.join(RECORDS_SUBDIR)
}

fn meta_path(user_data_root: &Path) -> PathBuf {
    user_data_root.join(META_FILE)
}

fn history_path(user_data_root: &Path) -> PathBuf {
    user_data_root.join(HISTORY_FILE)
}

fn record_file_path(user_data_root: &Path, uid: &str) -> PathBuf {
    let safe: String = uid
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let name = if safe.is_empty() { "unknown" } else { safe.as_str() };
    records_dir(user_data_root).join(format!("{name}.json"))
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountMeta {
    pub uid: String,
    pub source: String,
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wish_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct MetaAccountsFile {
    #[serde(default)]
    default_uid: Option<String>,
    #[serde(default)]
    accounts: HashMap<String, AccountMeta>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct HistoryFile {
    #[serde(default)]
    history: Vec<Value>,
}

pub fn load_bootstrap(user_data_root: &Path) -> Result<Value, String> {
    let ud = user_data_root;
    if !ud.is_dir() {
        return Ok(json!({
            "defaultUid": null,
            "accounts": {},
            "history": [],
        }));
    }

    let meta: MetaAccountsFile = if meta_path(user_data_root).is_file() {
        let s = fs::read_to_string(meta_path(user_data_root)).map_err(|e| e.to_string())?;
        serde_json::from_str(&s).unwrap_or_default()
    } else {
        MetaAccountsFile::default()
    };

    let history: Vec<Value> = if history_path(user_data_root).is_file() {
        let s = fs::read_to_string(history_path(user_data_root)).map_err(|e| e.to_string())?;
        let hf: HistoryFile = serde_json::from_str(&s).unwrap_or_default();
        hf.history
    } else {
        vec![]
    };

    let mut accounts_out: HashMap<String, Value> = HashMap::new();
    for (uid, m) in &meta.accounts {
        let records = read_records_only(user_data_root, uid)?;
        let account_json = json!({
            "uid": m.uid,
            "source": m.source,
            "updatedAt": m.updated_at,
            "result": {
                "uid": m.uid,
                "wish_url": m.wish_url,
                "records": records,
            }
        });
        accounts_out.insert(uid.clone(), account_json);
    }

    Ok(json!({
        "defaultUid": meta.default_uid,
        "accounts": accounts_out,
        "history": history,
    }))
}

fn read_records_only(user_data_root: &Path, uid: &str) -> Result<Vec<Value>, String> {
    let p = record_file_path(user_data_root, uid);
    if !p.is_file() {
        return Ok(vec![]);
    }
    let s = fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&s).map_err(|e| e.to_string())?;
    if let Some(arr) = v.as_array() {
        return Ok(arr.clone());
    }
    v.get("records")
        .and_then(|x| x.as_array())
        .cloned()
        .ok_or_else(|| "records 文件格式无效".into())
}

pub fn latest_ids_by_pool(user_data_root: &Path, uid: &str) -> Result<HashMap<String, String>, String> {
    let records = read_records_only(user_data_root, uid)?;
    let mut out: HashMap<String, String> = HashMap::new();
    for item in records {
        let gacha_type = item
            .get("gacha_type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .unwrap_or_default();
        let id = item
            .get("id")
            .map(|v| v.to_string().trim_matches('"').to_string())
            .unwrap_or_default();
        if gacha_type.is_empty() || id.is_empty() {
            continue;
        }
        let entry = out.entry(gacha_type).or_default();
        if entry.is_empty() || id > *entry {
            *entry = id;
        }
    }
    Ok(out)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistBody {
    #[serde(default)]
    pub default_uid: Option<String>,
    #[serde(default)]
    pub history: Vec<Value>,
    #[serde(default)]
    pub accounts: HashMap<String, Value>,
}

pub fn persist(user_data_root: &Path, body: PersistBody) -> Result<(), String> {
    let ud = user_data_root;
    let rd = records_dir(user_data_root);
    fs::create_dir_all(&ud).map_err(|e| e.to_string())?;
    fs::create_dir_all(&rd).map_err(|e| e.to_string())?;

    let mut meta = MetaAccountsFile {
        default_uid: body.default_uid.clone(),
        accounts: HashMap::new(),
    };

    for (uid_key, acc_val) in &body.accounts {
        let uid = acc_val
            .get("uid")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| acc_val.get("uid").and_then(|v| v.as_i64().map(|i| i.to_string())))
            .unwrap_or_else(|| uid_key.clone());
        let uid = uid.trim().to_string();
        if uid.is_empty() {
            continue;
        }

        let source = acc_val
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let updated_at = acc_val
            .get("updatedAt")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let wish_url = acc_val
            .get("result")
            .and_then(|r| r.get("wish_url"))
            .and_then(|v| v.as_str())
            .map(String::from);

        if let Some(result) = acc_val.get("result") {
            if let Some(records) = result.get("records").and_then(|r| r.as_array()) {
                let rf = record_file_path(user_data_root, &uid);
                let payload = json!({ "records": records });
                fs::write(&rf, serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            }
        }

        meta.accounts.insert(
            uid.clone(),
            AccountMeta {
                uid: uid.clone(),
                source,
                updated_at,
                wish_url,
            },
        );
    }

    let meta_s = serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?;
    fs::write(meta_path(user_data_root), meta_s).map_err(|e| e.to_string())?;

    let hist = HistoryFile {
        history: body.history.clone(),
    };
    let hist_s = serde_json::to_string_pretty(&hist).map_err(|e| e.to_string())?;
    fs::write(history_path(user_data_root), hist_s).map_err(|e| e.to_string())?;

    Ok(())
}
