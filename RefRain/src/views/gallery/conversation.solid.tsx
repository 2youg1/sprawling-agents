// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createSignal, For, Show } from "solid-js";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import Popover from "../parts/popover_view";
import { AddressLineLookView as AddressLine } from "../talk/address_line_view";
import { addressLineOf, boxWords } from "../talk/address_line";
import Asked from "../talk/asked_view";
import Coin from "../talk/coin_view";
import { ComposerFrame } from "../talk/composer_frame";
import Composer from "../talk/composer_view";
import { foldWire } from "../talk/fold";
import Gauge from "../talk/gauge_view";
import Person from "../talk/person_view";
import Rule from "../talk/rule_view";
import Saying from "../talk/saying_view";
import SettingsRow from "../talk/settings_row_view";
import TypedLine from "../talk/typed_line_view";
import WaitingCards from "../talk/waiting_view";
import { GalleryCase } from "./case";
import { AT_DEEPSEEK, MANY_PROVIDERS, pickerFacts } from "./picked";

import type { Doing } from "../../core/doing";
import { sendingInto } from "../../core/doing";
import { fill, say } from "../../core/lang";
import { toFragment } from "../../core/route";
import { offered } from "../../core/slash";
import type { Turn } from "../../wire";
import { RunId, Seq, TimeMs } from "../../wire";
import { motherName } from "../talk/forking";
import type { Utterance } from "./served";
import { CALLS, EARLIER_SEGMENT, ONE_QUESTION, ROUND, TURNS, WAITING } from "./served";
import { SAYING } from "./saying";

export { ONE_QUESTION } from "./served";

const POSTURES: readonly Doing[] = [
  { kind: "thinking" },
  { kind: "calling", tool: "exec", subject: "just check" },
  { kind: "waiting" },
  { kind: "frozen", completion: "done" },
  { kind: "unknown" },
];

const MOTHER = RunId.make("0199c0de-1a2b-4c3d-8e4f-5a6b7c8d9e0f");
const MOTHER_TASK = EARLIER_SEGMENT.find((line) => line.speaker === "person")?.text ?? "";
const LANDED: Turn = {
  calls: [],
  notes: [],
  number: 1,
  opened: Seq.make(1),
  t: TimeMs.make(1),
  timing: "measured",
};
const picker = pickerFacts(MANY_PROVIDERS, AT_DEEPSEEK, null);
const ignore = (): void => undefined;

export default function Conversation() {
  const lang = storeValue(ui().lang);
  const [open, setOpen] = createSignal(false);
  function utterance(line: Utterance) {
    if (line.speaker === "person")
      return (
        <div class="my-base flex flex-col items-end">
          <div class="max-w-[83%] rounded-panel bg-speech px-pane py-base text-body leading-read whitespace-pre-wrap">
            {line.text}
          </div>
          <div class="mt-tight text-note text-text-faint">{say(lang(), "talk_you")}</div>
        </div>
      );
    if (line.speaker === "resident")
      return (
        <div class="my-base text-body">
          <div class="mb-tight text-note text-text-faint">{say(lang(), "talk_resident")}</div>
          <div class="whitespace-pre-wrap leading-read">{line.text}</div>
        </div>
      );
    return null;
  }
  function branchable(line: Utterance) {
    return line.speaker === "person" ? (
      <Person
        text={line.text}
        label={say(lang(), "talk_you")}
        at={undefined}
        entry={{ kind: "message", turn: LANDED, text: line.text }}
        run={MOTHER}
        onFork={ignore}
        onHover={ignore}
      />
    ) : (
      utterance(line)
    );
  }
  function InBox(props: { readonly session?: Seq }) {
    return (
      <AddressLine
        look={addressLineOf(lang(), "hall/mayor", props.session ?? null, ignore)}
        seat={{ at: "box", ...boxWords(lang()) }}
      />
    );
  }
  return (
    <>
      <For each={POSTURES}>
        {(doing) => (
          <GalleryCase label={`${doing.kind} · ${sendingInto(doing)}`}>
            <Composer
              placeholder={say(lang(), "talk_placeholder_mayor")}
              sending={sendingInto(doing)}
              hearing={doing.kind === "thinking"}
              onSend={() => false}
              onStop={() => false}
            />
          </GalleryCase>
        )}
      </For>

      <For each={SAYING}>
        {(text) => (
          <GalleryCase label={`saying · ${String(text.length)}`}>
            <Saying text={text} who={say(lang(), "talk_resident")} />
          </GalleryCase>
        )}
      </For>

      <GalleryCase label="waiting">
        <WaitingCards items={WAITING} />
      </GalleryCase>

      <GalleryCase label="waiting · what the question is about, on the card">
        <Asked
          locator={ONE_QUESTION.artifact}
          content={{
            binary: false,
            bytes: 96,
            locator: ONE_QUESTION.artifact,
            text: "rm -rf target\n# frees 4.2 GiB; the next build starts cold",
            truncated: false,
          }}
        />
      </GalleryCase>

      <GalleryCase label="composer · an empty room lifts the box, the address line in its placeholder">
        <div class="flex flex-col items-center gap-base pt-[18vh] text-center">
          <div class="w-full">
            <Composer
              placeholder={say(lang(), "talk_placeholder_mayor")}
              sending="dispatch"
              door={() => <InBox />}
              onSend={() => false}
              onStop={() => false}
            />
          </div>
        </div>
      </GalleryCase>

      <GalleryCase label="composer · permissions open over the address line">
        <div class="flex min-h-[14rem] flex-col justify-end">
          <ComposerFrame
            door
            look={{
              form: { "aria-label": say(lang(), "region_composer") },
              over: false,
              box: { rows: 1, "aria-label": say(lang(), "talk_placeholder_mayor") },
            }}
            slots={{
              get door() {
                return <InBox />;
              },
              get keys() {
                return (
                  <Gauge room={null}>
                    <Coin face="idle" sending="dispatch" delivery="idle" onStop={ignore} />
                  </Gauge>
                );
              },
              get line() {
                return <TypedLine text="" box={undefined} lit={false} />;
              },
              get under() {
                return (
                  <SettingsRow
                    picker={picker}
                    room={null}
                    draws="everything"
                    kept={false}
                    menu="open"
                  />
                );
              },
            }}
          />
        </div>
      </GalleryCase>

      <GalleryCase label="composer · docked once the room has a thread, the address and session in its placeholder">
        <div class="px-pane pb-pane">
          <Composer
            started
            placeholder={say(lang(), "talk_placeholder_mayor")}
            sending="dispatch"
            door={() => <InBox session={Seq.make(23)} />}
            hearing
            onSend={() => false}
            onStop={() => false}
          />
        </div>
      </GalleryCase>

      <GalleryCase label="menu · a line that begins with a slash">
        <div class="pt-palette">
          <div class="pt-output">
            <div class="relative">
              <Popover
                label="talk_commands"
                columns={[
                  {
                    id: "commands",
                    label: "talk_commands",
                    rows: offered("/").map((each) => ({
                      id: each.spelling,
                      label: each.spelling,
                      secondary:
                        each.grammar === ""
                          ? say(lang(), each.about)
                          : `${each.grammar} · ${say(lang(), each.about)}`,
                    })),
                  },
                ]}
                onApply={() => undefined}
                onClose={() => undefined}
              />
            </div>
          </div>
        </div>
      </GalleryCase>

      <GalleryCase label="talk · one round, and the box docked">
        <div class="flex min-h-output flex-col">
          <div class="flex-1">
            <For each={ROUND}>{(line) => branchable(line)}</For>
          </div>
          <div class="px-pane pb-pane">
            <Composer
              started
              placeholder={say(lang(), "talk_placeholder_mayor")}
              sending="dispatch"
              onSend={() => false}
              onStop={() => false}
            />
          </div>
        </div>
      </GalleryCase>

      <GalleryCase label="session divider · the previous segment folded away">
        <Show when={open()}>
          <div class="fade">
            <For each={EARLIER_SEGMENT}>{(line) => utterance(line)}</For>
          </div>
        </Show>
        <div class="my-snug flex flex-col gap-snug">
          <Rule
            parts={[
              {
                kind: "fold",
                fold: {
                  label: `${fill(say(lang(), "session_previous"), { n: "5" })} · ${say(lang(), open() ? "session_collapse" : "session_expand")}`,
                  wire: foldWire(open(), () => setOpen(!open())),
                },
              },
            ]}
            mark="none"
            wire={{}}
          />
          <Rule
            parts={[
              { kind: "text", text: fill(say(lang(), "session_new_divider"), { at: "12:04" }) },
            ]}
            mark="none"
            wire={{}}
          />
        </div>
        <For each={ROUND}>{(line) => utterance(line)}</For>
      </GalleryCase>

      <GalleryCase label="talk · forked from turn 3, with the mother run linked">
        <Rule
          parts={[
            {
              kind: "text",
              text: fill(say(lang(), "session_forked_divider"), {
                turn: "3",
                at: motherName(MOTHER_TASK) ?? say(lang(), "fork_mother"),
              }),
            },
            { kind: "text", text: "11:47" },
            {
              kind: "link",
              text: say(lang(), "fork_back"),
              href: toFragment({ kind: "run", run: MOTHER }),
            },
          ]}
          mark="branch"
          wire={{}}
        />
        <For each={ROUND}>{(line) => branchable(line)}</For>
        <p class="mb-tight text-note text-text-faint" role="status">
          {fill(say(lang(), "fork_pending"), { turn: "3" })}
        </p>
      </GalleryCase>

      <GalleryCase label="message · fork from here, revealed under the message">
        <div class="[&_[data-fork]]:opacity-100">
          <For each={ROUND}>{(line) => branchable(line)}</For>
        </div>
      </GalleryCase>

      <GalleryCase label="fork picker · open, turns and calls in two columns">
        <div class="pt-palette">
          <div class="pt-output">
            <div class="relative">
              <Popover
                label="fork_pick_title"
                columns={[
                  {
                    id: "turns",
                    label: "run_turns",
                    rows: TURNS.map((each) => ({
                      id: each.id,
                      label: fill(say(lang(), "fork_from_turn"), { turn: each.turn }),
                      secondary: each.at,
                    })),
                  },
                  {
                    id: "calls",
                    label: "talk_calls",
                    rows: CALLS.map((each) => ({
                      id: each.id,
                      label: each.what,
                      secondary: each.at,
                    })),
                  },
                ]}
                onApply={() => undefined}
                onClose={() => undefined}
              />
            </div>
          </div>
        </div>
      </GalleryCase>
    </>
  );
}
