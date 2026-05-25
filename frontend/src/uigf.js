const FIVE_STAR_RANK = "5";

export function buildOverview(records) {
  const total = records.length;
  const fiveStarCount = records.filter((item) => `${item.rank_type}` === FIVE_STAR_RANK).length;
  const fourStarCount = records.filter((item) => `${item.rank_type}` === "4").length;
  return {
    total,
    five_star_count: fiveStarCount,
    four_star_count: fourStarCount,
  };
}

export function buildPoolSummary(records) {
  const grouped = new Map();
  for (const item of records) {
    const type = `${item.gacha_type || ""}`;
    if (!grouped.has(type)) {
      grouped.set(type, []);
    }
    grouped.get(type).push(item);
  }

  const rows = [];
  for (const [gachaType, itemsRaw] of grouped.entries()) {
    const items = [...itemsRaw].sort((a, b) => compareRecordId(a, b));
    const fiveStars = items.filter((x) => `${x.rank_type}` === FIVE_STAR_RANK);
    const upCount = fiveStars.filter((x) => x.is_up === true).length;
    const nonUpCount = fiveStars.filter((x) => x.is_up === false).length;

    const fiveIndexes = [];
    for (let i = 0; i < items.length; i += 1) {
      if (`${items[i].rank_type}` === FIVE_STAR_RANK) {
        fiveIndexes.push(i);
      }
    }

    const pityGaps = [];
    let prev = -1;
    for (const idx of fiveIndexes) {
      pityGaps.push(idx - prev);
      prev = idx;
    }

    const smallPityDenom = fiveStars.length - nonUpCount;
    const smallPityNum = upCount - nonUpCount;
    const smallPityNoMissRate =
      smallPityDenom > 0 && smallPityNum >= 0
        ? Number(((smallPityNum / smallPityDenom) * 100).toFixed(2))
        : null;

    rows.push({
      gacha_type: gachaType,
      total: items.length,
      five_star_count: fiveStars.length,
      up_count: upCount,
      avg_up_pity: upCount ? Number((items.length / upCount).toFixed(2)) : null,
      up_rate: fiveStars.length ? Number(((upCount / fiveStars.length) * 100).toFixed(2)) : 0,
      non_up_count: nonUpCount,
      small_pity_no_miss_rate: smallPityNoMissRate,
      avg_five_star_pity: pityGaps.length
        ? Number((pityGaps.reduce((s, n) => s + n, 0) / pityGaps.length).toFixed(2))
        : 0,
      latest_five_star: fiveStars.length ? fiveStars[fiveStars.length - 1].name : null,
    });
  }

  rows.sort((a, b) => a.gacha_type.localeCompare(b.gacha_type));
  return rows;
}

function compareRecordId(a, b) {
  const sm = a?.id;
  const sn = b?.id;
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

function uigfVersionLabel(info) {
  return info?.version || info?.uigf_version || "未知";
}

function mapUigfItem(item, accountUid, lookup) {
  const gachaType = item.uigf_gacha_type ?? item.gacha_type ?? "";
  let itemId = item.item_id != null ? `${item.item_id}` : "";
  if (!itemId.trim() && lookup) {
    const name = item.name ?? "";
    const itemType = item.item_type ?? "";
    const bucket = lookup[itemType];
    if (bucket && name && bucket[name] != null) {
      itemId = `${bucket[name]}`;
    }
  }
  return {
    id: item.id != null ? `${item.id}` : "",
    item_id: itemId,
    uid: item.uid != null ? `${item.uid}` : `${accountUid}`,
    name: item.name ?? "",
    item_type: item.item_type ?? "",
    rank_type: item.rank_type != null ? `${item.rank_type}` : "",
    time: item.time ?? "",
    gacha_type: `${gachaType}`,
    is_up: typeof item.is_up === "boolean" ? item.is_up : null,
  };
}

function accountListToResult(account, info, versionLabel) {
  const uid = `${account?.uid ?? ""}`.trim();
  if (!uid) {
    throw new Error("UIGF 账号缺少 uid。");
  }
  if (!Array.isArray(account?.list)) {
    throw new Error(`UIGF 账号 ${uid} 缺少 list。`);
  }

  const records = account.list
    .map((item) => mapUigfItem(item, uid))
    .sort(compareRecordId);

  return {
    uid,
    wish_url: null,
    overview: buildOverview(records),
    pool_summary: buildPoolSummary(records),
    records,
    logs: [
      "已从本地 UIGF 文件加载数据",
      `UIGF版本: ${versionLabel}`,
      `当前解析UID: ${uid}`,
      `总记录数: ${records.length}`,
    ],
  };
}

function fixMissingItemIds(payload) {
  if (Array.isArray(payload?.list)) {
    for (const item of payload.list) {
      if (item && !item.item_id) item.item_id = "";
    }
  }
  const accounts = payload?.hk4e || payload?.genshin;
  if (Array.isArray(accounts)) {
    for (const account of accounts) {
      if (!Array.isArray(account?.list)) continue;
      for (const item of account.list) {
        if (item && !item.item_id) item.item_id = "";
      }
    }
  }
}

/**
 * 解析 UIGF 文件，返回每个账号一条结果（v4.1 hk4e / v3.0 根级 list）。
 */
export function parseUigfAccounts(text) {
  let payload;
  try {
    payload = JSON.parse(text);
  } catch {
    throw new Error("不是有效的 JSON 文件。");
  }

  if (!payload || typeof payload !== "object") {
    throw new Error("不是有效的 UIGF 文件：根节点须为对象。");
  }

  fixMissingItemIds(payload);

  const versionLabel = uigfVersionLabel(payload.info);
  const hk4e = payload.hk4e ?? payload.genshin;

  if (Array.isArray(hk4e) && hk4e.length > 0) {
    return hk4e.map((account) => accountListToResult(account, payload.info, versionLabel));
  }

  if (Array.isArray(payload.list) && payload.list.length >= 0 && payload.info) {
    const uid = `${payload.info.uid ?? ""}`.trim();
    if (!uid) {
      throw new Error("UIGF v3.0 文件缺少 info.uid。");
    }
    return [
      accountListToResult(
        { uid, list: payload.list, lang: payload.info.lang },
        payload.info,
        versionLabel,
      ),
    ];
  }

  throw new Error(
    "不是有效的 UIGF 文件：需包含 hk4e[]（v4.x）或 info + list[]（v3.0）。",
  );
}

/** @deprecated 使用 parseUigfAccounts；仅导入单账号时取第一项 */
export function parseUigfToResult(text) {
  const accounts = parseUigfAccounts(text);
  if (accounts.length > 1) {
    const uids = accounts.map((a) => a.uid).join(", ");
    return {
      ...accounts[0],
      logs: [
        ...accounts[0].logs,
        `检测到 ${accounts.length} 个账号（${uids}），当前仅展示第一个；请使用最新导入逻辑加载全部账号。`,
      ],
    };
  }
  return accounts[0];
}
