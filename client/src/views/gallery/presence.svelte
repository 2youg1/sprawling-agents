<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The mailbox key at the foot of the edge column, in each of the
  // three ways it is drawn: nothing to do, a count of things to do, and
  // the mark that pulses while the link is still on its way up - the one
  // reason that resolves without the person doing anything. It reads
  // the city instead of its props, so each fixture hands it one through
  // `Stand`; what differs between the three cases is the city behind it.

  import type { AxError } from "../../wire";

  // One refusal nobody has opened. The words are the city's own - an
  // action, a subject and the way back - which is what a refusal
  // carries on the wire and what the panel under the dot reads back.
  const UNREAD: AxError = {
    action: "attach an endpoint",
    code: "E_CREDENTIAL_MISSING",
    gate: null,
    nearby: [],
    recovery: "file a key for this provider, then attach it again",
    retry: "no",
    subject: "zenmux",
  };
</script>

<script lang="ts">
  import Presence from "../notices.svelte";
  import Case from "./case.svelte";
  import { ONE_QUESTION } from "./conversation.svelte";
  import Stand from "./stand.svelte";
</script>

<Case label="mailbox key · nothing to do">
  <Stand link={{ kind: "live", city: "sprawling" }} unread={[]} waiting={[]}>
    <Presence asked={0} hint={(words: string) => words} />
  </Stand>
</Case>

<Case label="mailbox key · a question waiting and a refusal unread">
  <Stand
    link={{ kind: "live", city: "sprawling" }}
    unread={[UNREAD]}
    waiting={[ONE_QUESTION]}
  >
    <Presence asked={0} hint={(words: string) => words} />
  </Stand>
</Case>

<!-- Still on its way up, which is the state that resolves by itself:
the dot pulses rather than asking for anything. -->
<Case label="mailbox key · the link is still connecting">
  <Stand link={{ kind: "handshaking" }} unread={[]} waiting={[]}>
    <Presence asked={0} hint={(words: string) => words} />
  </Stand>
</Case>
