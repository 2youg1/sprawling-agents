// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Something that happened, said once where a person is looking and
// kept where they can look again (client/Spec.lean §4-35). One part
// draws every notice the client has - inline beside a field, a toast in
// the corner, an entry in the notification drawer - so a notice cannot
// be shown in one place and shaped differently in another.
//
// **The reader's own title is the heading; the city's way out is the
// body; the rest of its words are the fold.** The heading comes from
// `err_<code>` in `lang.json` (`./notice_title`), and one code covers
// several causes, so the heading names the kind of refusal and never
// the cause. The recovery the city wrote is the one sentence that knows
// the cause and what to do about it, so it stands under the heading
// unfolded; a person who had to open a disclosure to learn that the
// next step is the settings page would first press the buttons that do
// not help. The action and the subject fold into one mono-font
// disclosure and never stand as a heading (UX B8). The stable code
// itself rides the heading's right edge, where a person can cite it.
//
// **The same refusal is one notice with a count.** However many times a
// refusal arrives, a person who reads the same four fields twice cannot
// tell a city that failed once from one that failed twenty times. The
// merging happens where the notices are kept, keyed by the code and the
// subject together; this part receives the strings and the count.
//
// **The page's own answer to a key the person pressed is two phrases**,
// a heading and the next step, both from `lang.json`: it has no code to
// cite and no words of the city's to fold away.

import type { Snippet } from "svelte";

import { say } from "../../core/lang";
import type { Key, Lang } from "../../core/lang";
import { noticeTitle, recoveryWords } from "./notice_title";

// Something that happened, against something that was refused.
export type Weight = "info" | "alert";

// Hugging the field it belongs to, floating over the page for a few
// seconds, or standing in the drawer that keeps it (client/Spec.lean
// §4-35). The seat changes the drawing and nothing else: the role is
// the weight's to decide (7-1).
export type Seat = "inline" | "toast" | "drawer";

// What the notice says. A refusal arrives as the four strings its
// writer gave it - the city, or this page's link and asking: the action
// that failed, its stable code, what it was against, and what the
// caller can do next. Strings rather than the belief record, so this
// draws refusals from anywhere the wire carries one. Each shape names
// the other's fields as absent, so a reader of one field can tell
// which shape it was given.
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
  // Absent is `info`: `alert` is for something that was refused.
  readonly weight?: Weight | undefined;
  // Already formatted by the caller's clock.
  readonly at?: string | undefined;
  // How many times this refusal arrived; drawn past one.
  readonly count?: number | undefined;
  // Usually quiet Buttons carrying the recovery verbs.
  readonly actions?: Snippet | undefined;
}

export type NoticeProps = Common & Said;

// What a look of a notice is given.
export interface NoticeLook {
  readonly seat: Seat;
  readonly weight: Weight;
  // Spread on the notice: an alert announces itself, a status waits.
  readonly region: { readonly role: "alert" | "status" };
  readonly title: string;
  readonly at: string | undefined;
  // "×3", drawn only past one arrival.
  readonly count: string | undefined;
  // The stable code a person can cite, for a refusal.
  readonly code: string | undefined;
  // The next step under the heading; the empty string when there is
  // none.
  readonly next: string;
  // The city's own words, folded: the disclosure's name and its lines.
  readonly fold: { readonly summary: string; readonly lines: readonly string[] } | undefined;
  readonly actions: Snippet | undefined;
}

export function noticeOf(props: NoticeProps, lang: Lang): NoticeLook {
  const weight = props.weight ?? "info";
  const next =
    props.heading === undefined ? recoveryWords(lang, props.code, props.recovery) : say(lang, props.next);
  return {
    seat: props.seat,
    weight,
    region: { role: weight === "alert" ? "alert" : "status" },
    title: props.heading === undefined ? noticeTitle(lang, props.code, props.subject) : say(lang, props.heading),
    at: props.at,
    count: props.count !== undefined && props.count > 1 ? `×${String(props.count)}` : undefined,
    code: props.code,
    next,
    fold:
      props.heading === undefined
        ? {
            summary: say(lang, "notices_detail"),
            // The city's recovery is folded only when the page showed
            // its own sentence in its place.
            lines: [props.action, props.subject, ...(props.recovery !== "" && props.recovery !== next ? [props.recovery] : [])],
          }
        : undefined,
    actions: props.actions,
  };
}
