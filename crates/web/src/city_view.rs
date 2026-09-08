// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The isometric city, drawn as shapes in the document.
//!
//! **It was a canvas until F2.02, and the reason it was one did not reach
//! this picture.** The recorded argument was that a thousand Residents must
//! not become a thousand elements. This view has never drawn a Resident: it
//! draws Buildings, of which a city holds tens, and the canvas was charging
//! four certain costs for that hypothetical saving - a fixed bitmap resampled
//! by CSS on every display that is not exactly its size, no way to read a
//! custom property (which is why the selection outline settled for a grey
//! where the code plainly wanted `--ACCENT`), no hover, focus, or keyboard
//! reach without reimplementing all three, and a drawing path that existed
//! only on wasm and so was reachable by no host test or gate.
//!
//! **Hit testing is no longer a second derivation.** The browser tests hits
//! against the very polygons it painted, so "what is drawn is what can be
//! picked" stopped being an assertion and became the construction. The
//! inverse projection, the point-in-quadrilateral test and the pointer
//! coordinate clamp went with it.
//!
//! **The picture fills what it is given.** The viewBox is the bounding box
//! of what was drawn, so a city of three buildings is a picture of three
//! buildings rather than three specks in a fixed 1000x560 field. The old
//! fit reserved `2n+1` tile widths for a diamond `n` tiles wide, which is
//! where most of that empty field came from.
//!
//! **The silhouette is the data.** A tower's height is the work its plan
//! took on and the lit band up its walls is the part that is done, so
//! progress is read off the skyline rather than from a number beside it.

//! Index only.

mod page;
mod text;

pub use page::CityView;
