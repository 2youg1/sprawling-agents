// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

const MARK = "data-control-keydown";
const ENABLED = '[data-control]:not(:disabled, [aria-disabled="true"], [aria-busy="true"])';

export function watchControlFeedback(document: Document): () => void {
  const window = document.defaultView;
  const pressed = new Map<string, HTMLElement>();

  function clear(key: string): void {
    const element = pressed.get(key);
    if (element === undefined) return;
    pressed.delete(key);
    if (![...pressed.values()].includes(element)) element.removeAttribute(MARK);
  }

  function clearAll(): void {
    for (const key of pressed.keys()) clear(key);
  }

  function down(event: KeyboardEvent): void {
    if (
      event.repeat ||
      event.isComposing ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      (event.key !== "Enter" && event.key !== " ")
    )
      return;
    const element = event.target;
    if (
      !(element instanceof HTMLElement) ||
      !(element instanceof HTMLButtonElement) ||
      !element.matches(ENABLED)
    )
      return;
    clear(event.key);
    pressed.set(event.key, element);
    element.setAttribute(MARK, "");
  }

  function up(event: KeyboardEvent): void {
    clear(event.key);
  }

  function left(event: FocusEvent): void {
    for (const [key, element] of pressed) {
      if (element === event.target) clear(key);
    }
  }

  document.addEventListener("keydown", down, true);
  document.addEventListener("keyup", up, true);
  document.addEventListener("focusout", left);
  document.addEventListener("visibilitychange", clearAll);
  window?.addEventListener("blur", clearAll);
  return () => {
    clearAll();
    document.removeEventListener("keydown", down, true);
    document.removeEventListener("keyup", up, true);
    document.removeEventListener("focusout", left);
    document.removeEventListener("visibilitychange", clearAll);
    window?.removeEventListener("blur", clearAll);
  };
}
