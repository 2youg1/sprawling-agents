// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The rules of a document a person reads and edits through a page:
//! which version it is, which characters its bytes spell, where its
//! blocks lie, how much of it one answer carries, and what one save
//! changes (`crates/documents/Spec.lean`).
//!
//! No I/O: the bytes arrive as slices and leave as values. Reading the
//! disk and the content store, and recording a save, belong to the
//! crates that own those (D1).
