// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import type { Snippet } from "svelte";

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { noticeTitle, recoveryWords } from "./notice_title";


export type Weight = "info" | "alert";


export type Seat = "inline" | "toast" | "drawer";


export type Said =
  | {
      readonly action: string;
      readonly code: string;
      readonly subject: string;
      readonly recovery: string;
      readonly heading?: undefined;
      readonly next?: undefined;
    }
  | {
      readonly heading: Key;
      readonly next: Key;
      readonly action?: undefined;
      readonly code?: undefined;
      readonly subject?: undefined;
      readonly recovery?: undefined;
    };

export interface Common {
  readonly seat: Seat;
  
  readonly weight?: Weight | undefined;
  
  readonly at?: string | undefined;
  
  readonly count?: number | undefined;
  
  readonly actions?: Snippet | undefined;
}

export type NoticeProps = Common & Said;


export interface NoticeLook {
  readonly seat: Seat;
  readonly weight: Weight;
  
  readonly region: { readonly role: "alert" | "status" };
  readonly title: string;
  readonly at: string | undefined;
  
  readonly count: string | undefined;
  
  readonly code: string | undefined;
  
  
  readonly next: string;
  
  readonly fold: { readonly summary: string; readonly lines: readonly string[] } | undefined;
  readonly actions: Snippet | undefined;
}

export function noticeOf(props: NoticeProps, lang: Lang): NoticeLook {
  const weight = props.weight ?? "info";
  const next =
    props.heading === undefined
      ? recoveryWords(lang, props.code, props.recovery)
      : say(lang, props.next);
  return {
    seat: props.seat,
    weight,
    region: { role: weight === "alert" ? "alert" : "status" },
    title:
      props.heading === undefined
        ? noticeTitle(lang, props.code, props.subject)
        : say(lang, props.heading),
    at: props.at,
    count: props.count !== undefined && props.count > 1 ? `×${String(props.count)}` : undefined,
    code: props.code,
    next,
    fold:
      props.heading === undefined
        ? {
            summary: say(lang, "notices_detail"),
            
            
            lines: [
              props.action,
              props.subject,
              ...(props.recovery !== "" && props.recovery !== next ? [props.recovery] : []),
            ],
          }
        : undefined,
    actions: props.actions,
  };
}
