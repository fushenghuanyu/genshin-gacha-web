const API_BASE = "http://127.0.0.1:8001";

let cachedLookup = null;

export async function loadItemLookup() {
  if (cachedLookup) return cachedLookup;
  try {
    const resp = await fetch(`${API_BASE}/api/item-lookup`);
    if (!resp.ok) return null;
    cachedLookup = await resp.json();
    return cachedLookup;
  } catch {
    return null;
  }
}

export function resolveItemId(item, lookup) {
  const existing = `${item?.item_id ?? ""}`.trim();
  if (existing) return existing;
  if (!lookup || !item) return "";
  const name = `${item.name ?? ""}`.trim();
  const itemType = `${item.item_type ?? ""}`.trim();
  if (!name || !itemType) return "";
  const bucket = lookup[itemType];
  if (!bucket || typeof bucket !== "object") return "";
  const id = bucket[name];
  return id != null ? `${id}`.trim() : "";
}

export function enrichRecords(records, lookup) {
  if (!lookup || !Array.isArray(records)) return records;
  return records.map((item) => {
    const itemId = resolveItemId(item, lookup);
    if (!itemId) return item;
    return { ...item, item_id: itemId };
  });
}
