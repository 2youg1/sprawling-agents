// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { fill, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import type {
  RegistryNewest,
  ReleaseAnswer,
  ReleaseLine,
  ReleaseVerdict,
  UpdateHint,
} from "../../wire";
import { confirmedCommand, registryLineOf, updateOf } from "./reading";

export type Ink = "text" | "quiet" | "accent" | "alert";

export interface Statement {
  readonly ink: Ink;
  readonly text: string;
}

export interface RegistryLook {
  readonly name: string;
  readonly newest: ReleaseLine | undefined;
  readonly reason: { readonly text: string; readonly said: string | undefined } | undefined;
}

export interface ChannelWire {
  readonly "aria-label": string;
  readonly value: string;
  readonly onchange: (event: { readonly currentTarget: { readonly value: string } }) => void;
}

export interface UpdateLook {
  readonly channel: string;
  readonly choice:
    | {
        readonly unchosen: string;
        readonly commands: readonly string[];
        readonly wire: ChannelWire;
      }
    | undefined;
  readonly command: string | undefined;
}

export interface AnswerLook {
  readonly refused: { readonly text: string; readonly recovery: string } | undefined;
  readonly statements: readonly Statement[];
  readonly registries: readonly RegistryLook[];
  readonly update: UpdateLook | undefined;
}

export interface Hands {
  readonly choose: (command: string | null) => void;
}

export function lookOf(
  answer: ReleaseAnswer,
  lang: Lang,
  chosen: string | null,
  hands: Hands,
): AnswerLook {
  const updateLook = (hint: UpdateHint): UpdateLook => {
    const how = updateOf(hint);
    const label = say(lang, "release_channel_confirm");
    return {
      channel: say(lang, how.channel),
      choice:
        hint.alternatives.length === 0
          ? undefined
          : {
              unchosen: label,
              commands: hint.alternatives,
              wire: {
                "aria-label": label,
                value: chosen ?? "",
                onchange: (event) => {
                  hands.choose(confirmedCommand(hint, event.currentTarget.value));
                },
              },
            },
      command: how.command ?? confirmedCommand(hint, chosen) ?? undefined,
    };
  };
  const registries = (lines: readonly RegistryNewest[]): RegistryLook[] =>
    lines.map((each) => {
      const line = registryLineOf(each);
      return {
        name: say(lang, line.registry),
        newest: line.newest ?? undefined,
        reason:
          line.reason === null
            ? undefined
            : { text: say(lang, line.reason.key), said: line.reason.said ?? undefined },
      };
    });
  if ("refused" in answer) {
    return {
      refused: {
        text: say(lang, "release_refused_registries"),
        recovery: answer.refused.refusal.recovery,
      },
      statements: [],
      registries: [],
      update: undefined,
    };
  }
  if ("unconfirmed" in answer) {
    const { mine } = answer.unconfirmed;
    return {
      refused: undefined,
      statements: [
        {
          ink: "text",
          text: fill(say(lang, "release_mine"), { version: mine.version, released: mine.released }),
        },
        { ink: "quiet", text: say(lang, "release_origin_unconfirmed") },
      ],
      registries: registries(answer.unconfirmed.registries),
      update: updateLook(answer.unconfirmed.update),
    };
  }
  if ("unreleased" in answer) {
    return {
      refused: undefined,
      statements: [{ ink: "quiet", text: say(lang, "release_source") }],
      registries: registries(answer.unreleased.registries),
      update: updateLook(answer.unreleased.update),
    };
  }
  const { mine, newest, verdict } = answer.stands;
  const mineText = fill(
    say(lang, mine.released.length === 0 ? "release_mine_undated" : "release_mine"),
    {
      version: mine.version,
      released: mine.released,
    },
  );
  return {
    refused: undefined,
    statements: [{ ink: "text", text: mineText }, verdictOf(verdict, newest, lang)],
    registries: registries(answer.stands.registries),
    update: updateLook(answer.stands.update),
  };
}

function verdictOf(verdict: ReleaseVerdict, newest: ReleaseLine, lang: Lang): Statement {
  switch (verdict) {
    case "current":
      return { ink: "accent", text: say(lang, "release_current") };
    case "ahead":
      return { ink: "quiet", text: fill(say(lang, "release_ahead"), { version: newest.version }) };
    case "behind":
      return {
        ink: "alert",
        text: fill(
          say(lang, newest.released.length === 0 ? "release_behind_undated" : "release_behind"),
          {
            version: newest.version,
            released: newest.released,
          },
        ),
      };
  }
}
