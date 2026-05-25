import { buildOverview, buildPoolSummary } from "./uigf.js";

function recordDedupeKey(item) {
  const id = item?.id;
  if (id != null && `${id}`.length > 0) {
    return `id:${id}`;
  }
  return `fallback:${item?.time ?? ""}|${item?.name ?? ""}|${item?.gacha_type ?? ""}`;
}

function compareRecordId(m, n) {
  const sm = m?.id;
  const sn = n?.id;
  if (sm != null && `${sm}`.length > 0 && sn != null && `${sn}`.length > 0) {
    try {
      const num = BigInt(String(sm)) - BigInt(String(sn));
      if (num > 0n) return 1;
      if (num < 0n) return -1;
      return 0;
    } catch {
      /* fall through */
    }
  }
  return String(sm ?? "").localeCompare(String(sn ?? ""));
}

export function mergeRecordLists(local, origin) {
  const a = local || [];
  const b = origin || [];
  if (!a.length) return b.slice();
  if (!b.length) return a.slice();

  const list = [...b, ...a];
  const out = [];
  const seen = new Set();
  for (const item of list) {
    const k = recordDedupeKey(item);
    if (!seen.has(k)) {
      out.push(item);
    }
    seen.add(k);
  }
  return out.sort(compareRecordId);
}

export function mergeGachaResult(local, incoming) {
  if (!local || !incoming) return incoming;
  const localUid = local.uid != null ? `${local.uid}`.trim() : "";
  const incomingUid = incoming.uid != null ? `${incoming.uid}`.trim() : "";
  if (!localUid || localUid !== incomingUid) {
    return incoming;
  }

  const localRecords = Array.isArray(local.records) ? local.records : [];
  const incomingRecords = Array.isArray(incoming.records) ? incoming.records : [];
  const mergedRecords = mergeRecordLists(localRecords, incomingRecords);
  const prevN = localRecords.length;
  const curN = incomingRecords.length;
  const mergedN = mergedRecords.length;

  const baseLogs = Array.isArray(incoming.logs) ? incoming.logs : [];
  const mergeLog =
    prevN > 0 && curN > 0
      ? `[合并] 本地 ${prevN} 条 + 本次 ${curN} 条 → 去重后 ${mergedN} 条（重复 ${prevN + curN - mergedN} 条）`
      : null;

  return {
    ...incoming,
    uid: incoming.uid,
    wish_url: incoming.wish_url != null && incoming.wish_url !== "" ? incoming.wish_url : local.wish_url,
    records: mergedRecords,
    overview: buildOverview(mergedRecords),
    pool_summary: buildPoolSummary(mergedRecords),
    logs: mergeLog ? [mergeLog, ...baseLogs] : baseLogs,
  };
}
