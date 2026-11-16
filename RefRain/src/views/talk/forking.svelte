<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  import { fill, say } from "../../core/lang";
  import type { Key } from "../../core/lang";
  import type { Call, RoundsAnswer } from "../../wire";
  import { ui } from "../../ui";
  import { callWord } from "./calls";
  import { Popover, type PopoverColumn, type PopoverRow } from "../parts/popover";
  import { planFork } from "./forking";
  import type { ForkEntry, ForkFilter, ForkPlan } from "./forking";

  interface Props {
    readonly rounds: RoundsAnswer;
    readonly onFork: (plan: ForkPlan) => void;
    readonly onClose: () => void;
  }

  const { rounds, onFork, onClose }: Props = $props();
  const { lang } = ui();

  const FILTER_KEY: Readonly<Record<ForkFilter, Key>> = {
    all: "fork_filter_all",
    quiet: "fork_filter_no_tools",
    user: "fork_filter_user",
  };
  const FILTERS: readonly ForkFilter[] = ["all", "quiet", "user"];

  interface Row {
    readonly id: string;
    readonly entry: ForkEntry;
    readonly label: string;
    readonly secondary?: string;
    readonly tools: boolean;
  }

  let filter = $state<ForkFilter>("all");
  let cursor = $state("");

  const turns = $derived(rounds.turns);

  const rows = $derived.by((): Row[] => {
    const out: Row[] = [];
    const first = turns.at(0);
    const task = rounds.opening?.task ?? "";
    if (first !== undefined && task !== "") {
      out.push({
        id: "m:task",
        entry: { kind: "message", turn: first, text: task },
        label: task,
        secondary: say($lang, "talk_you"),
        tools: first.calls.length > 0,
      });
    }
    for (const turn of turns) {
      const tools = turn.calls.length > 0;
      out.push({
        id: `t:${String(turn.number)}`,
        entry: { kind: "turn", turn },
        label: fill(say($lang, "fork_from_turn"), { turn: String(turn.number) }),
        secondary: turn.said ?? "",
        tools,
      });
      for (const note of turn.notes) {
        if (!("arrived" in note)) continue;
        out.push({
          id: `m:${String(note.arrived.at)}`,
          entry: { kind: "message", turn, text: note.arrived.said ?? "" },
          label: note.arrived.said ?? "",
          secondary: note.arrived.from ?? "",
          tools,
        });
      }
    }
    return out;
  });

  const shown = $derived.by((): Row[] => {
    switch (filter) {
      case "all":
        return rows;
      case "quiet":
        return rows.filter((row) => !row.tools);
      case "user":
        return rows.filter((row) => row.entry.kind === "message");
    }
  });

  const held = $derived(rows.find((row) => row.id === cursor) ?? shown.at(0));
  const calls = $derived(held?.entry.turn.calls ?? []);
  const heldPlan = $derived(held === undefined ? null : planFork(rounds.run, held.entry));

  const callRow = (call: Call): PopoverRow => ({
    id: `c:${String(call.at)}`,
    label: callWord(call.tool, call.subject),
  });
  const columns = $derived.by((): readonly PopoverColumn[] => [
    {
      id: "turns",
      label: "fork_turns",
      rows: shown.map((row) => ({
        id: row.id,
        label: row.label,
        secondary: row.secondary === "" ? undefined : row.secondary,
      })),
    },
    { id: "calls", label: "talk_calls", rows: calls.map(callRow) },
  ]);

  const footer = $derived.by((): string => {
    const told = `${say($lang, FILTER_KEY[filter])} · ${say($lang, "fork_keeps")}`;
    return heldPlan?.walkedBack ? `${told} · ${say($lang, "fork_safe_point")}` : told;
  });

  function apply(pane: PopoverColumn, row: PopoverRow): void {
    if (pane.id !== "turns") return;
    const found = rows.find((each) => each.id === row.id);
    if (found === undefined) return;
    onFork(planFork(rounds.run, found.entry));
  }

  function cursorAt(rowId: string | null): void {
    if (rowId !== null && !rowId.startsWith("c:")) cursor = rowId;
  }

  function keys(event: KeyboardEvent): void {
    if (!event.ctrlKey || event.metaKey || event.key.toLowerCase() !== "o") return;
    event.preventDefault();
    const at = FILTERS.indexOf(filter);
    filter = FILTERS[(at + 1) % FILTERS.length] ?? "all";
  }
</script>

<svelte:window onkeydown={keys} />

<div class="relative w-full">
  <Popover label="fork_pick_title" {columns} onApply={apply} onCursorChange={cursorAt} {onClose} />
  <div class="py-tight text-note text-text-faint" role="status">{footer}</div>
</div>
