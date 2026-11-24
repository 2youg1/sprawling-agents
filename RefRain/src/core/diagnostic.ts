// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { WIRE_V } from "../wire";
import type { AxError } from "../wire";

const frameField = /^(?:stage|frame|wire_v|path|expected|actual_type|position) = /;
const bodyField =
  /^(?:stage = "validate"|minimum = [0-9]+|actual = "-?[0-9]+(?:\.[0-9]+)?"|outcome = "unsent")$/;
const supportedLocation = /^(?:[a-z\d]+(?:[-._/][a-z\d]+)*|[a-z\d/]+:\d+)$/i;
const doorField =
  /^(?:stage = "(?:pair|challenge|session)"|problem = "(?:timeout|network|response|refused|unconfirmed)"|outcome = "(?:paired_session_unconfirmed|no_registration_confirmed|unknown)"|http_status = [1-5][0-9][0-9])$/;
const doorSubjects = new Set(["/pair", "/session/challenge", "/session"]);
const doorCodes = new Set(["E_TIMEOUT", "E_TOOL_OUTCOME_UNKNOWN", "E_PAIRING_REFUSED"]);
const safeRecoveries = new Map<string, readonly string[]>([
  ["answer an approval", ["this item is not waiting; the inbox lists the ones that are"]],
  [
    "send a picture",
    ["choose a model with image input, or send the text alone after removing the image"],
  ],
  [
    "steer a run",
    [
      "no run in flight answers to that id: steer one while it runs, or dispatch a new one",
      "the run ended before it accepted the steer; review the retained draft and send it as a follow-up",
    ],
  ],
]);

export function diagnosticToml(error: AxError): string {
  const frame =
    error.code === "E_WIRE_MISMATCH" ? error.nearby.filter((line) => frameField.test(line)) : [];
  const body =
    error.code === "E_INVALID_ARGS" && error.subject === "Appearance.body_px"
      ? error.nearby.filter((line) => bodyField.test(line))
      : [];
  const door =
    doorCodes.has(error.code) && doorSubjects.has(error.subject)
      ? error.nearby.filter((line) => doorField.test(line))
      : [];
  const fields = [
    `code = ${JSON.stringify(error.code)}`,
    `action = ${JSON.stringify(error.action)}`,
    ...(error.request == null ? [] : [`request = ${JSON.stringify(error.request)}`]),
    ...(door.length > 0 ||
    (supportedLocation.test(error.subject) && !error.subject.startsWith("sk-"))
      ? [`subject = ${JSON.stringify(error.subject)}`]
      : []),
    `retry = ${JSON.stringify(error.retry)}`,
    ...(safeRecoveries.get(error.action)?.includes(error.recovery) === true
      ? [`recovery = ${JSON.stringify(error.recovery)}`]
      : []),
    ...(!frame.some((line) => line.startsWith("wire_v = ")) ? [`wire_v = ${String(WIRE_V)}`] : []),
    ...frame,
    ...body,
    ...door,
  ];
  return fields.join("\n");
}
