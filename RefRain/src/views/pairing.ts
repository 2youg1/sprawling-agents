// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say, type Key, type Lang } from "../core/lang";
import type { Readable } from "../core/store";
import type { Credential, LocalDoorIssue, Paired, PairWhy } from "../core/local/entering";
import type { AxError } from "../wire";

export interface PairingProps {
  readonly why: PairWhy;
  readonly lang: Readable<Lang>;
  readonly label: string;
  readonly pair: (code: string, label: string) => Promise<Paired>;
  readonly onPaired: (credential: Credential) => void;
  readonly issue?: LocalDoorIssue | undefined;
  readonly check?: (() => Promise<Paired>) | undefined;
  readonly seat?: "page" | "specimen";
}

export const WHY: Readonly<Record<PairWhy, Key | undefined>> = {
  first: undefined,
  open_used: "pair_open_used",
  session_lost: "pair_session_lost",
  keyless: "pair_keyless",
  pair_unknown: "pair_unknown",
  pair_unconfirmed: "pair_unconfirmed",
  session_pending: "pair_session_pending",
  unreachable: "enrol_unreachable",
};

const ISSUE_KEY: Record<LocalDoorIssue["problem"], Key> = {
  timeout: "pair_issue_timeout",
  network: "pair_issue_network",
  response: "pair_issue_response",
  refused: "pair_issue_refused",
  unconfirmed: "pair_issue_unconfirmed",
};

export function doorIssue(lang: Lang, why: PairWhy, issue: LocalDoorIssue): AxError {
  const subject = issue.phase === "challenge" ? "/session/challenge" : `/${issue.phase}`;
  const outcome =
    why === "session_pending"
      ? "paired_session_unconfirmed"
      : why === "pair_unconfirmed"
        ? "no_registration_confirmed"
        : "unknown";
  return {
    code:
      issue.problem === "timeout"
        ? "E_TIMEOUT"
        : issue.problem === "refused" || issue.problem === "unconfirmed"
          ? "E_PAIRING_REFUSED"
          : "E_TOOL_OUTCOME_UNKNOWN",
    action: say(lang, "pair_issue_action"),
    subject,
    recovery: fill(say(lang, ISSUE_KEY[issue.problem]), { path: subject }),
    retry: why === "session_pending" ? "yes" : "unknown",
    nearby: [
      `stage = "${issue.phase}"`,
      `problem = "${issue.problem}"`,
      `outcome = "${outcome}"`,
      ...(issue.status === undefined ? [] : [`http_status = ${String(issue.status)}`]),
    ],
  };
}

export function codeOf(typed: string): string {
  return typed.replace(/\s/g, "").toLowerCase();
}
