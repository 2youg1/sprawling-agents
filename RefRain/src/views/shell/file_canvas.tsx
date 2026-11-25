// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { lazy, Loading, Match, Show, Switch, untrack } from "solid-js";

import { say, type Lang } from "../../core/lang";
import type { Stretch } from "../../core/stretches";
import type { Seq } from "../../wire";
import { PdfLoading } from "../editor/formats/pdf_loading";
import { AttachmentTarget } from "./attachment_target";
import { scopedId } from "./chrome";
import type { WorkspaceDocuments } from "./documents";
import { cacheKey, type OpenedFile } from "./file_cache";
import { WorkspaceFailure } from "./workspace_failure";

const TextDocument = lazy(() => import("../editor/workspace_document"));

interface FileProps {
  readonly docs: WorkspaceDocuments;
  readonly file: OpenedFile;
  readonly lang: Lang;
}

function FileHeader(props: FileProps) {
  function category(event: SubmitEvent): void {
    event.preventDefault();
    if (!(event.currentTarget instanceof HTMLFormElement)) return;
    const value = new FormData(event.currentTarget).get("category");
    if (typeof value === "string") props.docs.category(props.file, value.trim() || null);
  }
  return (
    <div class="refrain-file-header">
      <h2>{props.file.record.path}</h2>
      <details>
        <summary>{say(props.lang, "shell_file_details")}</summary>
        <form class="refrain-files-tools" onSubmit={category}>
          <label>
            {say(props.lang, "shell_file_category")}
            <input name="category" value={props.file.record.category ?? ""} />
          </label>
          <button data-control="" class="refrain-nav-button" type="submit">
            {say(props.lang, "shell_save_category")}
          </button>
        </form>
        <a href={props.docs.link(props.file)}>{say(props.lang, "shell_file_link")}</a>
      </details>
      <button
        data-control=""
        class="refrain-nav-button"
        type="button"
        onClick={() => props.docs.closeFile()}
      >
        {say(props.lang, "shell_close_file")}
      </button>
    </div>
  );
}

function FormatFile(props: FileProps) {
  const registered = untrack(() => props.file);
  const textRange = () => {
    const selection = props.docs.revealed();
    return selection?.kind === "markdown" ? selection : undefined;
  };
  const pdfRange = () => {
    const selected = props.docs.revealed();
    const citation = props.docs.citation();
    return selected !== undefined && selected.kind !== "markdown" && citation !== null
      ? { ...selected, version: citation.version }
      : null;
  };
  return (
    <div class="refrain-document-holder">
      <Loading
        fallback={
          <p class="refrain-canvas-status" role="status">
            {say(props.lang, "shell_file_loading")}
          </p>
        }
      >
        <Switch>
          <Match when={props.file.record.path.toLowerCase().endsWith(".pdf")}>
            <PdfLoading
              bytes={props.file.bytes}
              name={props.file.record.path}
              version={props.file.record.version}
              lang={props.lang}
              progress={props.docs.remembered() ?? null}
              reference={pdfRange()}
              onProgress={(point) => props.docs.progress(registered, point)}
              onSelection={(range) => props.docs.attach(props.file, range, range.version)}
            />
          </Match>
          <Match when={true}>
            <TextDocument
              identity={cacheKey(props.file.workspace, props.file.record.id)}
              name={props.file.record.path}
              bytes={props.file.bytes}
              version={props.file.record.version}
              save={(baseline, bytes) => props.docs.save(props.file, baseline, bytes)}
              onSelection={(range) =>
                props.docs.attach(props.file, range, props.file.record.version)
              }
              reveal={textRange()}
              proposal={props.docs.reviewing()}
              onProposalDecision={() => props.docs.closeReview()}
              readResource={(relative) => props.docs.readResource(props.file, relative)}
              lang={props.lang}
            />
          </Match>
        </Switch>
      </Loading>
    </div>
  );
}

function quoteOf(selection: ReturnType<WorkspaceDocuments["revealed"]>): string | undefined {
  return selection !== undefined && "text" in selection ? selection.text : undefined;
}

export function FileCanvas(props: {
  readonly scope?: string | undefined;
  readonly docs: WorkspaceDocuments;
  readonly lang: Lang;
  readonly visible: boolean;
  readonly session?: Seq | undefined;
  readonly began?: Seq | undefined;
  readonly targets: readonly Stretch[];
}) {
  return (
    <>
      <section
        id={scopedId("workspace-file-canvas", props.scope)}
        class="refrain-canvas"
        data-refrain-reading-canvas=""
        aria-label={say(props.lang, "shell_file_mode")}
        inert={!props.visible}
        aria-hidden={props.visible ? "false" : "true"}
      >
        <Show when={props.docs.pending()}>
          <p class="refrain-canvas-status" role="status">
            {say(props.lang, "shell_file_loading")}
          </p>
        </Show>
        <Show when={props.docs.failed() !== null}>
          <div class="refrain-canvas-status">
            <WorkspaceFailure failure={props.docs.failed()} lang={props.lang} />
          </div>
          <Show when={props.docs.recovery() !== null}>
            <form class="refrain-files-tools" onSubmit={(event) => props.docs.relink(event)}>
              <label>
                {say(props.lang, "shell_relink_path")}
                <input
                  value={props.docs.repair()}
                  required
                  onInput={(event) => {
                    props.docs.setRepair(event.currentTarget.value);
                  }}
                />
              </label>
              <button data-control="" class="refrain-nav-button" type="submit">
                {say(props.lang, "shell_relink")}
              </button>
            </form>
          </Show>
        </Show>
        <Show when={props.docs.changed()}>
          <div class="refrain-canvas-status" role="status">
            <p>{say(props.lang, "shell_reference_changed")}</p>
            <Show when={quoteOf(props.docs.citation()?.selection)}>
              {(quote) => <blockquote>{quote()}</blockquote>}
            </Show>
          </div>
        </Show>
        <Show when={props.docs.opened()}>
          {(file) => (
            <>
              <FileHeader file={file()} docs={props.docs} lang={props.lang} />
              <Show when={props.docs.identity()} keyed>
                <FormatFile file={file()} docs={props.docs} lang={props.lang} />
              </Show>
            </>
          )}
        </Show>
        <Show when={props.docs.opened() === null && !props.docs.pending()}>
          <p class="refrain-canvas-status">{say(props.lang, "shell_no_file")}</p>
        </Show>
      </section>
      <Show when={props.docs.selection()}>
        {(attachment) => (
          <AttachmentTarget
            attachment={attachment()}
            current={props.session ?? props.began}
            choosing={props.docs.choosing()}
            lang={props.lang}
            targets={props.targets}
            onUse={props.docs.useTarget}
            onNew={props.docs.newSession}
            onCancel={props.docs.cancelTarget}
          />
        )}
      </Show>
    </>
  );
}
