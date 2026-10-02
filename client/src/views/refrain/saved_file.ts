// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A file the page hands the browser to save: an exported version, an
// exported comparison (client/Spec.lean §4-61). The browser's own download
// asks where it goes; nothing is written anywhere else.

export function saveFile(name: string, type: string, text: string): void {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  // The download has its own copy once the click is handled.
  setTimeout(() => {
    URL.revokeObjectURL(url);
  }, 0);
}
