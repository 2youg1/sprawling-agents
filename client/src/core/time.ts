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

// Whole numbers with a thin separator, and money from micro-dollars.
export function count(n: number): string {
  return n.toLocaleString("en-US");
}

export function usd(micros: number): string {
  return `$${(micros / 1_000_000).toFixed(micros >= 1_000_000 ? 2 : 3)}`;
}

export function kib(bytes: number): string {
  if (bytes < 1024) return `${String(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MiB`;
}
