// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the context report draws from the city's answers: payload
// composition by byte weight, ranked so the largest eater is first,
// and the provider's window fullness as a single header figure.
//
// Fullness stays the provider's `input_tokens` against `context_tokens`
// (glossary: context reminder). Composition is bytes from Prefix and
// PromptShape — never a second tokenizer. Shares of the payload sum to
// 100 by largest remainder, the same rule cost attribution uses.

import type { Lang } from "../../core/lang";
import { fill, say } from "../../core/lang";
import { kilo } from "../../core/time";
import type {
  PrefixAnswer,
  PrefixSegment,
  PrefixSource,
  PromptShapeAnswer,
  PromptTool,
  RunId,
} from "../../wire";
import type { Context } from "./gauge";
import { usedPercent } from "./gauge";

/** The six regions of one request, in cache order. */
export type BucketId = "tools" | "city" | "building" | "resident" | "run" | "conversation";

const BUCKETS: readonly BucketId[] = [
  "tools",
  "city",
  "building",
  "resident",
  "run",
  "conversation",
];

/** How many child rows a bucket may disclose — the rest are noise. */
const CHILD_CAP = 8;

export interface ReportChild {
  /** Stable list key: tool name, or the source's full address. */
  readonly key: string;
  readonly name: string;
  readonly bytes: number;
  readonly bytesSaid: string;
}

export interface ReportBucket {
  readonly id: BucketId;
  readonly said: string;
  readonly bytes: number;
  readonly bytesSaid: string;
  /** Whole percent of the known payload; shares sum to 100. */
  readonly share: number;
  /** Note when the bucket has no child rows (conversation or empty). */
  readonly emptyNote: string;
  readonly children: readonly ReportChild[];
}

export interface ReportLook {
  readonly run: RunId;
  readonly title: string;
  /** Caption under the centre when no bucket is held. */
  readonly fullLabel: string;
  readonly pickHint: string;
  readonly waiting: string;
  readonly openPrompt: string;
  /** Provider fullness, or null when the model stated no window. */
  readonly fullness: Context | null;
  /** Used share of the window, or 0 when there is no window. */
  readonly fullPercent: number;
  readonly usedSaid: string;
  readonly shareSaid: string;
  /** One sentence: which bucket eats the most of the known payload. */
  readonly insight: string;
  /** Quiet alert when a reminder threshold is crossed, else null. */
  readonly warning: string | null;
  /** Non-zero buckets, largest first. */
  readonly buckets: readonly ReportBucket[];
  /** Payload byte total the shares were taken from. */
  readonly payload: number;
  readonly promptHref: string;
}

export function reportOf(
  lang: Lang,
  run: RunId,
  prefix: PrefixAnswer | null,
  shape: PromptShapeAnswer | null,
  fullness: Context | null,
): ReportLook {
  const weights = weightsOf(prefix, shape);
  const payload = weights.reduce((sum, [, bytes]) => sum + bytes, 0);
  const shares = splitPercents(weights);
  const byId = new Map(shares.map((row) => [row.id, row] as const));
  const buckets = BUCKETS.flatMap((id): ReportBucket[] => {
    const row = byId.get(id);
    const bytes = row?.bytes ?? 0;
    const share = row?.share ?? 0;
    // Empty of bytes stays out; a non-empty row that rounded to 0% still
    // stays so the person can open its sources (shown as 0% on the chip).
    if (bytes <= 0) return [];
    const children = childrenOf(id, prefix);
    return [
      {
        id,
        said: bucketSaid(lang, id),
        bytes,
        bytesSaid: kilo(bytes),
        share,
        emptyNote:
          id === "conversation"
            ? say(lang, "context_report_conversation_note")
            : say(lang, "context_report_no_parts"),
        children,
      },
    ];
  }).sort((a, b) => b.bytes - a.bytes || a.id.localeCompare(b.id));

  const top = buckets[0];
  const insight =
    top === undefined
      ? say(lang, "context_report_empty")
      : fill(say(lang, "context_report_insight"), {
          bucket: top.said,
          share: String(top.share),
        });

  const usedSaid =
    fullness === null
      ? say(lang, "context_report_no_window")
      : fill(say(lang, "ring_used"), {
          used: kilo(fullness.used),
          window: kilo(fullness.window),
        });
  const shareSaid =
    fullness === null ? "" : fill(say(lang, "ring_share"), { n: String(usedPercent(fullness)) });

  return {
    run,
    title: say(lang, "context_report_title"),
    fullLabel: say(lang, "context_report_full"),
    pickHint: say(lang, "context_report_pick"),
    waiting: say(lang, "context_report_waiting"),
    openPrompt: say(lang, "context_report_open_prompt"),
    fullness,
    fullPercent: fullness === null ? 0 : usedPercent(fullness),
    usedSaid,
    shareSaid,
    insight,
    warning: warningOf(lang, fullness),
    buckets,
    payload,
    promptHref: `#/run/${run}/prompt`,
  };
}

function bucketKey(id: BucketId): "context_bucket_tools" | "context_bucket_city" | "context_bucket_building" | "context_bucket_resident" | "context_bucket_run" | "context_bucket_conversation" {
  switch (id) {
    case "tools":
      return "context_bucket_tools";
    case "city":
      return "context_bucket_city";
    case "building":
      return "context_bucket_building";
    case "resident":
      return "context_bucket_resident";
    case "run":
      return "context_bucket_run";
    case "conversation":
      return "context_bucket_conversation";
  }
}

function warningOf(lang: Lang, fullness: Context | null): string | null {
  if (fullness === null) return null;
  // Compare the raw share so a 64.6% window does not trip a 65% handoff
  // just because the displayed percent rounds up.
  const used = (fullness.used / fullness.window) * 100;
  if (fullness.second !== null && used >= fullness.second) {
    return fill(say(lang, "context_report_handoff"), { n: String(fullness.second) });
  }
  if (fullness.first !== null && used >= fullness.first) {
    return fill(say(lang, "context_report_filling"), { n: String(fullness.first) });
  }
  return null;
}

function weightsOf(
  prefix: PrefixAnswer | null,
  shape: PromptShapeAnswer | null,
): readonly (readonly [BucketId, number])[] {
  const segment = (slot: PrefixSegment["slot"]): number =>
    prefix?.segments.find((each) => each.slot === slot)?.bytes ?? 0;
  // Prefer the shape's live tool-table bytes when present: that is what
  // the last request carried. Fall back to the frozen table length.
  // Child rows for tools still come from Prefix (below): PromptShape has
  // no per-tool breakdown, so the unfold is the frozen table.
  const tools = shape?.tools.bytes ?? prefix?.tools?.len ?? 0;
  const conversation = shape?.conversation.bytes ?? 0;
  return [
    ["tools", tools],
    ["city", segment("city")],
    ["building", segment("building")],
    ["resident", segment("resident")],
    ["run", segment("run")],
    ["conversation", conversation],
  ];
}

function childrenOf(id: BucketId, prefix: PrefixAnswer | null): readonly ReportChild[] {
  switch (id) {
    case "tools":
      // Frozen table only — see weightsOf. Off stays out; dormant stays in,
      // because a dormant schema still sits in the live tools.bytes share.
      return toolChildren(prefix?.tools?.rows ?? []);
    case "city":
    case "building":
    case "resident":
    case "run":
      return sourceChildren(prefix?.segments.find((each) => each.slot === id)?.sources ?? []);
    case "conversation":
      return [];
  }
}

function toolChildren(rows: readonly PromptTool[]): readonly ReportChild[] {
  return [...rows]
    .filter((row) => row.exposure !== "off" && row.len > 0)
    .sort((a, b) => b.len - a.len || a.name.localeCompare(b.name))
    .slice(0, CHILD_CAP)
    .map((row) => ({ key: row.name, name: row.name, bytes: row.len, bytesSaid: kilo(row.len) }));
}

function sourceChildren(sources: readonly PrefixSource[]): readonly ReportChild[] {
  return [...sources]
    .filter((source) => source.kept > 0)
    .sort((a, b) => b.kept - a.kept || a.addr.localeCompare(b.addr))
    .slice(0, CHILD_CAP)
    .map((source) => ({
      key: source.addr,
      name: source.addr.slice(source.addr.lastIndexOf("/") + 1) || source.addr,
      bytes: source.kept,
      bytesSaid: kilo(source.kept),
    }));
}

/** Whole percents that sum to 100, or all zeros when the payload is empty. */
export function splitPercents(
  weights: readonly (readonly [BucketId, number])[],
): readonly { id: BucketId; bytes: number; share: number }[] {
  const sum = weights.reduce((total, [, bytes]) => total + bytes, 0);
  if (sum === 0) {
    return weights.map(([id, bytes]) => ({ id, bytes, share: 0 }));
  }
  const scaled = weights.map(([id, bytes]) => {
    const product = bytes * 100;
    return { id, bytes, floor: Math.floor(product / sum), rem: product % sum };
  });
  let assigned = scaled.reduce((total, row) => total + row.floor, 0);
  const order = scaled
    .map((row, index) => ({ index, rem: row.rem, id: row.id }))
    .sort((a, b) => b.rem - a.rem || a.id.localeCompare(b.id));
  const shares = scaled.map((row) => row.floor);
  for (const each of order) {
    if (assigned >= 100) break;
    const at = shares[each.index];
    if (at === undefined) continue;
    shares[each.index] = at + 1;
    assigned += 1;
  }
  return scaled.map((row, index) => ({
    id: row.id,
    bytes: row.bytes,
    share: shares[index] ?? 0,
  }));
}

/** Arc segments for a donut with `pathLength="100"`, clockwise from noon. */
export function arcsOf(
  buckets: readonly ReportBucket[],
): readonly { id: BucketId; share: number; offset: number; tone: number }[] {
  let offset = 0;
  return buckets.map((bucket, index) => {
    const arc = { id: bucket.id, share: bucket.share, offset, tone: index };
    offset += bucket.share;
    return arc;
  });
}

export function bucketSaid(lang: Lang, id: BucketId): string {
  return say(lang, bucketKey(id));
}
