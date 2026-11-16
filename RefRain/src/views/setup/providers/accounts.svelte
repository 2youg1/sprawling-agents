<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Roster } from "./account_editor";
  import type { FlightState } from "./flight";

  export interface AccountsProps {
    readonly roster: Roster;
    readonly flight?: FlightState;
  }
</script>

<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { enrol } from "../../../core/enrol";
  import { ui } from "../../../ui";
  import { awaits, freshEditor, leave, refuse, settle } from "./account_editor";
  import type { Editor, Hands, Step } from "./account_editor";
  import { lookOf } from "./accounts";
  import type { Holds } from "./accounts";
  import Look from "./accounts.look.svelte";

  const { roster, flight }: AccountsProps = $props();
  const u = ui();
  const lang = u.lang;
  const belief = u.conn.belief;

  const editor: Editor = $state(freshEditor());
  let heading: HTMLElement | undefined;
  const held: Record<string, HTMLElement | undefined> = {};
  const attachments: Record<string, Attachment<HTMLElement> | undefined> = {};

  const hands: Hands = $derived.by((): Hands => ({
    roster: () => roster,
    lang: () => $lang,
    reach: () => ({ origin: u.origin, token: u.credential() }),
    send: (command) => u.send(command),
    enrol,
    follow: (id, step) => {
      void tick().then(() => held[`${id}:${step}`]?.querySelector("button")?.focus());
    },
    focusHeading: () => {
      void tick().then(() => heading?.focus());
    },
    ...(flight === undefined ? {} : { flight }),
  }));

  const holds: Holds = {
    heading: (node) => {
      heading = node;
      return () => {
        heading = undefined;
      };
    },
    control: (id: string, step: Step) => {
      const key = `${id}:${step}`;
      const kept = attachments[key];
      if (kept !== undefined) return kept;
      const made: Attachment<HTMLElement> = (node) => {
        held[key] = node;
        return () => {
          if (held[key] === node) held[key] = undefined;
        };
      };
      attachments[key] = made;
      return made;
    },
  };

  $effect(() => {
    settle(editor, roster);
  });
  $effect(() => {
    const refused = $belief.refusal;
    if (refused === null || !awaits(editor)) return;
    refuse(editor, refused);
    u.conn.dismissRefusal();
  });
  onDestroy(() => {
    leave(editor);
  });

  const look = $derived(lookOf(editor, hands, holds));
</script>

<Look {...look} />
