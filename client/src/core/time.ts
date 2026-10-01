// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Time as a person reads it: a moment relative to now, and a clock
// time for the divider between two runs. `now` is a parameter, so the
// same moment reads the same in a test.

import type { Lang } from "./lang";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

export function ago(lang: Lang, at: number, now: number): string {
  const gap = Math.max(0, now - at);
  if (gap < MINUTE) {
    return lang === "zh" ? "刚刚" : "just now";
  }
  if (gap < HOUR) {
    const minutes = Math.floor(gap / MINUTE);
    return lang === "zh" ? `${String(minutes)} 分钟前` : `${String(minutes)} min ago`;
  }
  if (gap < DAY) {
    const hours = Math.floor(gap / HOUR);
    return lang === "zh" ? `${String(hours)} 小时前` : `${String(hours)} h ago`;
  }
  const days = Math.floor(gap / DAY);
  return lang === "zh" ? `${String(days)} 天前` : `${String(days)} d ago`;
}

export function clock(lang: Lang, at: number): string {
  return new Date(at).toLocaleString(lang === "zh" ? "zh-CN" : "en-GB", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// The time of day alone, for a column where the day is the same on
// every row.
export function hhmm(at: number): string {
  return new Date(at).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

export function hhmmss(at: number): string {
  return new Date(at).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}

// Whole numbers with a thin separator, and money from micro-dollars.
export function count(n: number): string {
  return n.toLocaleString("en-US");
}

// A count of tokens as a reader compares two of them at a glance:
// thousands as `k`, millions as `M`, three significant figures, and no
// trailing zero, so `82.4k / 200k` reads as a fraction rather than as
// two long numbers. Under a thousand the count is exact.
export function kilo(n: number): string {
  if (n < 1_000) return String(Math.round(n));
  // Rounded before the mark is chosen, so 999,999 is `1M` and not `1000k`.
  const thousands = Number((n / 1_000).toPrecision(3));
  if (thousands < 1_000) return `${String(thousands)}k`;
  return `${String(Number((n / 1_000_000).toPrecision(3)))}M`;
}

export function usd(micros: number): string {
  return `$${(micros / 1_000_000).toFixed(micros >= 1_000_000 ? 2 : 3)}`;
}

export function kib(bytes: number): string {
  if (bytes < 1024) return `${String(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}

// A length of time as the largest two units that say it: `42s`,
// `4m 12s`, `1h 03m`. Under ten seconds keeps a tenth, because a tool
// call is often that short.
export function lasted(ms: number): string {
  const seconds = Math.max(0, ms) / 1000;
  if (seconds < 10) return `${seconds.toFixed(1)}s`;
  const whole = Math.round(seconds);
  if (whole < 60) return `${String(whole)}s`;
  const minutes = Math.floor(whole / 60);
  if (minutes < 60) return `${String(minutes)}m ${String(whole % 60).padStart(2, "0")}s`;
  return `${String(Math.floor(minutes / 60))}h ${String(minutes % 60).padStart(2, "0")}m`;
}
