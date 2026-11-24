// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createEffect, createMemo, createSignal, onSettled, Show } from "solid-js";
import { Effect, Option, Result, Schema } from "effect";
import type { EditorChange } from "../../core/document_pos";
import { readDraft, writeDraft } from "../../core/document_save";
import { EDITABLE_BYTES_MAX } from "../../core/document_windows";
import { fill, langOf, say } from "../../core/lang";
import type { Lang } from "../../core/lang";
import { kib } from "../../core/time";
import { preferences } from "../../core/prefs";
import { B3Hash } from "../../wire";
import Glyph from "../parts/glyph.solid";
import type { DocumentRewrite } from "../talk/attachments";
import { openEditor } from "./editing";
import type { ContextSelection, Editing, TextRange } from "./editing";
import type { MarkdownMode } from "./markdown_editing";
import type { PreviewResource } from "./markdown_widgets";
import { phrasesIn } from "./reading";
import { WorkspacePreview, workspaceFormat } from "./workspace_preview";
import { WorkspaceProposal } from "./workspace_proposal";
import { writeRefusal } from "./workspace_receipt";
import { selectionMenuStyle, WorkspaceDocumentTools } from "./workspace_document_tools";
import type { WriteReceipt } from "./workspace_receipt";
import {
  decodeWorkspaceText,
  encodeWorkspaceChanges,
  proposalBlock,
  validReplacement,
  workspaceExcerpt,
} from "./workspace_text";
import "./workspace_document.css";

export interface MarkdownSelection extends TextRange {
  readonly kind: "markdown";
  readonly text: string;
}

export interface WorkspaceDocumentProps {
  readonly name: string;
  readonly bytes: Uint8Array;
  readonly version: string;
  readonly save: (baseline: string, bytes: Uint8Array) => Promise<{ version: string }>;
  readonly lang?: Lang | undefined;
  readonly onSelection?: ((selection: MarkdownSelection) => void) | undefined;
  readonly reveal?: TextRange | undefined;
  readonly readResource?: PreviewResource | undefined;
  readonly identity?: string | undefined;
  readonly proposal?: DocumentRewrite | undefined;
  readonly onProposalDecision?: ((decision: "accepted" | "rejected") => void) | undefined;
}

const WRITING_KEY = "refrain.editor.typewriter";
const DRAFT_DELAY = 350;

export default function WorkspaceDocument(props: WorkspaceDocumentProps) {
  const lang = () => props.lang ?? langOf(document.documentElement.lang);
  const format = createMemo(() => workspaceFormat(props.name));
  const decoded = createMemo(() =>
    format().kind === "docx" || format().kind === "image"
      ? null
      : workspaceExcerpt(props.version, props.bytes),
  );
  const editable = () => decoded() !== null;
  const hasPreview = () => ["html", "mermaid", "math"].includes(format().kind);
  const [receipt, setReceipt] = createSignal<WriteReceipt>({ kind: "clean" });
  const [dirty, publishDirty] = createSignal(false);
  const [typewriter, setTypewriter] = createSignal(false);
  const [mode, setMode] = createSignal<MarkdownMode>("source");
  const [preview, setPreview] = createSignal<string | null>(null);
  const [canAttach, setCanAttach] = createSignal(false);
  const [context, setContext] = createSignal<ContextSelection | null>(null);
  const [attachRefused, setAttachRefused] = createSignal(false);
  const [shownVersion, setShownVersion] = createSignal<string>();
  const [draftUnkept, setDraftUnkept] = createSignal(false);
  const [staleDraft, setStaleDraft] = createSignal<string>();
  const [editRevision, setEditRevision] = createSignal(0);
  let host: HTMLDivElement | undefined;
  let menuButton: HTMLButtonElement | undefined;
  let editor: Editing | null = null;
  let original: Uint8Array = new Uint8Array();
  let baseline: ReturnType<typeof decodeWorkspaceText> = null;
  let busy = false;
  let alive = true;
  let draftKey: string | undefined;
  let draftTimer: ReturnType<typeof setTimeout> | undefined;
  let draftDue = 0;
  let previousDirty = false;
  let reviewing = false;
  const prefs = preferences();
  const validVersion = Schema.decodeUnknownOption(B3Hash);

  const reveal = (wanted: TextRange | undefined): void => {
    if (editor === null || baseline === null || wanted === undefined) return;
    if (baseline.version !== props.version) return;
    if (
      !Number.isInteger(wanted.from) ||
      !Number.isInteger(wanted.to) ||
      wanted.from < 0 ||
      wanted.to < wanted.from ||
      wanted.to > baseline.editor.length
    )
      return;
    editor.revealRange({
      from: editor.fromBaseline(wanted.from),
      to: editor.fromBaseline(wanted.to),
    });
  };

  function setDirty(next: boolean): void {
    if (next === previousDirty) return;
    previousDirty = next;
    publishDirty(next);
  }

  function keepWhenQuiet(): void {
    const remaining = draftDue - performance.now();
    if (remaining > 0) draftTimer = setTimeout(keepWhenQuiet, remaining);
    else keepDraft();
  }

  function keepSoon(): void {
    if (draftKey === undefined) return;
    draftDue = performance.now() + DRAFT_DELAY;
    draftTimer ??= setTimeout(keepWhenQuiet, DRAFT_DELAY);
  }

  function keepDraft(): void {
    clearTimeout(draftTimer);
    draftTimer = undefined;
    if (draftKey === undefined || editor === null || baseline === null) return;
    const version = Option.getOrNull(validVersion(baseline.version));
    if (version === null) return;
    const changes = editor.changes();
    if (changes.length > 0 || staleDraft() === undefined)
      prefs.setDraft(draftKey, writeDraft({ version, changes }));
  }

  function followIncoming(version: string, bytes: Uint8Array): void {
    if (
      editor === null ||
      baseline === null ||
      busy ||
      version === baseline.version ||
      editor.hasChanges()
    )
      return;
    const next = workspaceExcerpt(version, bytes);
    if (next === null) return;
    baseline = next;
    original = bytes;
    editor.follow(next.editor);
    editor.readOnly(original.length > EDITABLE_BYTES_MAX);
    setShownVersion(version);
    reveal(props.reveal);
  }

  async function save(change?: EditorChange): Promise<void> {
    const current = editor;
    const previous = baseline;
    if (
      current === null ||
      previous === null ||
      busy ||
      current.composing() ||
      previous.version !== props.version
    )
      return;
    const changes = change === undefined ? current.changes() : [change];
    if (changes.length === 0 || original.length > EDITABLE_BYTES_MAX) return;
    const bytes = encodeWorkspaceChanges(original, previous, changes);
    busy = true;
    if (change === undefined) current.sent();
    else current.readOnly(true);
    setReceipt({ kind: "saving" });
    const result = await Effect.runPromise(
      Effect.result(
        Effect.tryPromise({
          try: () => props.save(previous.version, bytes),
          catch: (error) => error,
        }),
      ),
    );
    busy = false;
    if (!alive) return;
    if (Result.isFailure(result)) {
      if (change === undefined) current.unsent();
      else current.readOnly(original.length > EDITABLE_BYTES_MAX);
      setReceipt(writeRefusal(result.failure, previous.version));
      return;
    }
    const accepted = result.success;
    original = bytes;
    baseline = decodeWorkspaceText(accepted.version, original);
    if (change === undefined) current.landed();
    else {
      if (baseline !== null) current.rebaseChange(baseline.editor, change);
      current.readOnly(original.length > EDITABLE_BYTES_MAX);
    }
    setShownVersion(accepted.version);
    setDirty(current.hasChanges());
    setReceipt({ kind: "saved" });
    setStaleDraft(undefined);
    if (draftKey !== undefined) {
      const landed = Option.getOrNull(validVersion(accepted.version));
      if (landed !== null)
        prefs.setDraft(draftKey, writeDraft({ version: landed, changes: current.changes() }));
    }
    if (change !== undefined) props.onProposalDecision?.("accepted");
  }

  function accept(proposal: DocumentRewrite, replacement: string): void {
    if (
      editor === null ||
      proposalBlock(proposal, baseline, editor.changes()) !== null ||
      !validReplacement(replacement)
    )
      return;
    const insert = replacement.replace(/\r\n?/gu, "\n");
    if (insert === proposal.quote) props.onProposalDecision?.("accepted");
    else void save({ from: proposal.from, to: proposal.to, insert });
  }

  function attach(range: TextRange | null | undefined): void {
    if (editor === null || baseline === null || range === null || range === undefined) return;
    const from = editor.toBaseline(range.from);
    const to = editor.toBaseline(range.to);
    const text = editor.slice(range);
    if (from === to || text !== baseline.editor.slice(from, to)) {
      setAttachRefused(true);
      return;
    }
    setAttachRefused(false);
    setContext(null);
    props.onSelection?.({ kind: "markdown", from, to, text });
  }

  function closeContext(): void {
    if (context() === null) return;
    setContext(null);
    editor?.focus();
  }

  function pickPreview(): void {
    if (preview() !== null) {
      setPreview(null);
      onSettled(() => editor?.measure());
    } else {
      setPreview(editor?.text() ?? decoded()?.editor ?? "");
    }
  }

  onSettled(() => {
    if (host === undefined || !editable()) return;
    baseline = decoded();
    if (baseline === null) return;
    original = props.bytes;
    setShownVersion(baseline.version);
    draftKey = props.identity === undefined ? undefined : `workspace-document:${props.identity}`;
    const stored = draftKey === undefined ? "" : prefs.draft(draftKey);
    const prior = readDraft(stored, Number.MAX_SAFE_INTEGER);
    const restored =
      prior?.version === baseline.version ? readDraft(stored, baseline.editor.length) : null;
    if (prior !== null && restored === null) setStaleDraft(stored);
    const stopDraft =
      draftKey === undefined ? undefined : prefs.draftUnkept(draftKey).subscribe(setDraftUnkept);
    globalThis.addEventListener("pagehide", keepDraft);
    const markdown = format().kind === "markdown";
    editor = openEditor({
      parent: host,
      text: baseline.editor,
      changes: restored?.changes ?? [],
      label: props.name,
      phrases: phrasesIn(lang()),
      markdown: markdown
        ? {
            diagramLabel: say(lang(), "refrain_diagram"),
            readResource: props.readResource,
          }
        : undefined,
      onSave: () => {
        void save();
      },
      onEdit: () => {
        setDirty(editor?.hasChanges() ?? false);
        if (reviewing) setEditRevision((count) => count + 1);
        keepSoon();
      },
      onMarkdownMode: setMode,
      ...(markdown
        ? {
            onSelection: (range: TextRange | null) => {
              setCanAttach(range !== null);
              setAttachRefused(false);
            },
            onContextSelection: (menu: ContextSelection) => {
              if (props.onSelection === undefined) return;
              setContext(menu);
              onSettled(() => menuButton?.focus());
            },
          }
        : {}),
    });
    setDirty((restored?.changes.length ?? 0) > 0);
    const preference = Result.try(() => localStorage.getItem(WRITING_KEY) === "on");
    const wanted = Result.isSuccess(preference) && preference.success;
    setTypewriter(wanted);
    editor.typewriter(wanted);
    editor.readOnly(original.length > EDITABLE_BYTES_MAX);
    reveal(props.reveal);
    return () => {
      globalThis.removeEventListener("pagehide", keepDraft);
      keepDraft();
      stopDraft?.();
      alive = false;
      editor?.destroy();
    };
  });

  createEffect(
    () => props.proposal !== undefined,
    (shown) => {
      reviewing = shown;
    },
  );
  createEffect(() => props.reveal, reveal);
  createEffect(
    () => ({ version: props.version, bytes: props.bytes }),
    ({ version, bytes }) => followIncoming(version, bytes),
  );

  const changed = () => shownVersion() !== undefined && shownVersion() !== props.version;
  const blockedProposal = (proposal: DocumentRewrite) => {
    editRevision();
    if (original.length > EDITABLE_BYTES_MAX) return "readonly";
    if (shownVersion() !== props.version || receipt().kind === "conflict") return "version";
    return proposalBlock(proposal, baseline, editor?.changes() ?? []);
  };
  return (
    <section
      class="workspace-document"
      aria-label={props.name}
      onKeyDown={(event) => {
        if (event.key !== "Escape" || context() === null) return;
        event.preventDefault();
        event.stopPropagation();
        closeContext();
      }}
    >
      <WorkspaceDocumentTools
        version={shownVersion() ?? props.version}
        lang={lang()}
        editable={editable()}
        markdown={format().kind === "markdown"}
        mode={mode()}
        onMode={(next) => editor?.setMarkdown(next)}
        onAttach={
          props.onSelection !== undefined && canAttach()
            ? () => attach(editor?.selection())
            : undefined
        }
        previewable={hasPreview()}
        previewing={preview() !== null}
        onPreview={pickPreview}
        typewriter={typewriter()}
        onTypewriter={(wanted) => {
          setTypewriter(wanted);
          editor?.typewriter(wanted);
          Result.try(() => localStorage.setItem(WRITING_KEY, wanted ? "on" : "off"));
        }}
        dirty={dirty()}
        changed={changed()}
        receipt={receipt()}
        onSave={() => void save()}
      />
      <Show when={changed()}>
        <p role="status">{say(lang(), "refrain_external_changed")}</p>
      </Show>
      <Show when={staleDraft() !== undefined}>
        <div class="workspace-document-error" role="status">
          <p>{say(lang(), "refrain_previous_draft")}</p>
          <button
            type="button"
            data-control="surface"
            onClick={() => {
              const stored = staleDraft();
              if (stored !== undefined) void navigator.clipboard.writeText(stored);
            }}
          >
            <Glyph name="copy" size="sm" />
            {say(lang(), "refrain_copy_draft")}
          </button>
        </div>
      </Show>
      <Show when={draftUnkept() && dirty()}>
        <p role="status">{say(lang(), "refrain_registered_draft_unkept")}</p>
      </Show>
      <Show when={receipt().detail !== undefined}>
        <details class="workspace-document-error">
          <summary>{receipt().detail}</summary>
          <Show when={receipt().diagnostic !== undefined}>
            <pre>{receipt().diagnostic}</pre>
            <button
              type="button"
              data-control="surface"
              onClick={() => {
                const diagnostic = receipt().diagnostic;
                if (diagnostic !== undefined) void navigator.clipboard.writeText(diagnostic);
              }}
            >
              <Glyph name="copy" size="sm" />
              {say(lang(), "refrain_copy_diagnostic")}
            </button>
          </Show>
          <button
            type="button"
            data-control="surface"
            onClick={() => {
              const text = editor?.text();
              if (text !== undefined) void navigator.clipboard.writeText(text);
            }}
          >
            <Glyph name="copy" size="sm" />
            {say(lang(), "refrain_copy_draft")}
          </button>
        </details>
      </Show>
      <Show when={attachRefused()}>
        <p role="status">{say(lang(), "refrain_attach_unsaved")}</p>
      </Show>
      <Show when={props.proposal} keyed>
        {(proposal) => (
          <WorkspaceProposal
            proposal={proposal}
            lang={lang()}
            reason={blockedProposal(proposal)}
            busy={receipt().kind === "saving" || receipt().kind === "pending"}
            onReveal={() => {
              reveal(proposal);
              editor?.focus();
            }}
            onAccept={(replacement) => accept(proposal, replacement)}
            onReject={() => props.onProposalDecision?.("rejected")}
          />
        )}
      </Show>
      <Show when={editable()}>
        <Show when={props.bytes.length > EDITABLE_BYTES_MAX}>
          <p role="status">
            {fill(say(lang(), "refrain_too_large"), {
              total: kib(props.bytes.length),
              through: kib(Math.min(props.bytes.length, EDITABLE_BYTES_MAX)),
            })}
          </p>
        </Show>
        <div
          class="workspace-editor-host"
          ref={(node) => {
            host = node;
          }}
          hidden={preview() !== null}
          data-refrain-reading-canvas
        />
      </Show>
      <Show when={format().kind === "docx" || format().kind === "image" || preview() !== null}>
        <WorkspacePreview
          lang={lang()}
          name={props.name}
          text={preview() ?? ""}
          bytes={props.bytes}
          format={format()}
        />
      </Show>
      <Show when={!editable() && format().kind !== "docx" && format().kind !== "image"}>
        <p role="status">{say(lang(), "refrain_format_opaque")}</p>
      </Show>
      <Show when={context() !== null}>
        <div class="workspace-selection-backdrop" onPointerDown={closeContext} />
        <div class="workspace-selection-menu" style={selectionMenuStyle(context())}>
          <button
            type="button"
            data-control="surface"
            ref={(node) => {
              menuButton = node;
            }}
            onClick={() => attach(context()?.range)}
          >
            <Glyph name="pin" size="sm" />
            {say(lang(), "refrain_attach_selection")}
          </button>
        </div>
      </Show>
    </section>
  );
}
