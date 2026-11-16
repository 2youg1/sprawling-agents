// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors


import { diff } from "@codemirror/merge";

import { fill, say } from "../../../core/lang";
import type { Lang } from "../../../core/lang";

export interface Side {
  
  readonly label: string;
  readonly text: string;
}

export interface Comparison {
  readonly name: string;
  readonly from: Side;
  readonly to: Side;
  
  
  readonly about: readonly string[];
}


export function comparedName(name: string): string {
  return `${name}.comparison.md`;
}

export function comparedMarkdown(lang: Lang, comparison: Comparison): string {
  const { from, to } = comparison;
  const changes = diff(from.text, to.text);
  const head = [
    `# ${fill(say(lang, "compared_title"), { name: comparison.name })}`,
    "",
    `- ${fill(say(lang, "compared_from"), { version: from.label })}`,
    `- ${fill(say(lang, "compared_to"), { version: to.label })}`,
    ...comparison.about.map((line) => `- ${line}`),
    "",
  ];
  if (changes.length === 0) return [...head, say(lang, "compared_none"), ""].join("\n");
  const passages = changes.flatMap((change, index) => [
    `## ${fill(say(lang, "compared_change"), { n: String(index + 1) })}`,
    "",
    say(lang, "compared_removed"),
    "",
    ...fenced(from.text.slice(change.fromA, change.toA)),
    "",
    say(lang, "compared_added"),
    "",
    ...fenced(to.text.slice(change.fromB, change.toB)),
    "",
  ]);
  return [...head, ...passages].join("\n");
}


function fenced(text: string): readonly string[] {
  const longest = Math.max(0, ...[...text.matchAll(/`+/gu)].map((run) => run[0].length));
  const fence = "`".repeat(Math.max(3, longest + 1));
  return [fence, ...(text === "" ? [] : [text.endsWith("\n") ? text.slice(0, -1) : text]), fence];
}
