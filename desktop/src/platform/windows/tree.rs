// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The UI Automation tree of one window: role, name, ref and bounds.
//!
//! **Every COM object this module makes dies inside the call that made
//! it.** A `ref` handed back to a caller is a rectangle and a
//! generation, never a live element (desktop-SPEC.md §8.6, second pair),
//! so a connection that snapshots a thousand windows holds a thousand
//! rectangles rather than a thousand cross-process interface pointers.
//! That is what confines the whole of COM to this file.
//!
//! The walk is breadth-first and bounded twice — by the depth the caller
//! asked for and by [`views::MOST_REFS_PER_SNAPSHOT`] — because a UI
//! Automation tree has no promise of being finite in any practical
//! sense: one list control can present tens of thousands of items, and a
//! snapshot a model cannot read to the end is a snapshot it has not read.
//!
//! A node with neither a name nor a usable rectangle is skipped and its
//! children are not: a layout container a caller cannot act on should
//! not cost a ref, and should not hide what is inside it either.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker,
};

use super::fault;
use super::geometry::Bounds;
use super::views::{MOST_REFS_PER_SNAPSHOT, Node};
use crate::refusal::Refusal;

/// Reads one window's tree.
///
/// # Errors
/// Refuses when UI Automation will not start, when the window has no
/// automation element, and when the tree cannot be walked. A single node
/// that will not answer is skipped rather than refused, because one
/// unreadable control is not a reason to report no window.
#[expect(
    unsafe_code,
    reason = "UI Automation is a COM interface; every method on it is an FFI call"
)]
pub(crate) fn read(handle: HWND, depth: u32) -> Result<Vec<Node>, Refusal> {
    let automation = start()?;
    // SAFETY: `automation` is a live `IUIAutomation` this function owns,
    // and `handle` is an HWND this connection resolved from the desktop
    // in this same call. A window that has closed since makes this fail
    // rather than misbehave, which is the `map_err` below.
    let root = unsafe { automation.ElementFromHandle(handle) }.map_err(|err| {
        fault::win32(
            "read the window's accessibility tree",
            "call `desktop.windows` again; a window that has closed has no tree, and a window \
             drawn without accessibility information cannot be read this way — use \
             `desktop.screenshot` to see it instead",
            &err,
        )
    })?;
    // SAFETY: `automation` is the live object created above. The walker
    // is a property read that allocates a new interface pointer and
    // borrows nothing from us.
    let walker = unsafe { automation.ControlViewWalker() }.map_err(|err| {
        fault::win32(
            "open a walk over the window's tree",
            "take a `desktop.screenshot` of this window instead",
            &err,
        )
    })?;
    Ok(walk(&walker, &root, depth))
}

/// This thread's automation apartment, entered once per call.
///
/// `CoInitializeEx` on a thread that is already in a compatible
/// apartment answers `S_FALSE`, which is a success: the server's read
/// loop is one thread, so the second and every later snapshot take that
/// path. It is deliberately not paired with `CoUninitialize` — leaving
/// the apartment would invalidate nothing this module still holds, and
/// re-entering it per call buys nothing.
#[expect(
    unsafe_code,
    reason = "entering a COM apartment and creating the automation object are both FFI entry points"
)]
fn start() -> Result<IUIAutomation, Refusal> {
    // SAFETY: this passes no reserved pointer, which is what the API
    // requires, and asks for the apartment model UI Automation is
    // documented to be used from. It is called on the server's own read
    // loop thread and on no other, so it cannot race a second
    // initialisation.
    let entered = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if entered.is_err() {
        return Err(fault::win32(
            "join this thread to the desktop's automation apartment",
            "restart this server; a thread that cannot enter an apartment cannot read any tree",
            &windows::core::Error::from_hresult(entered),
        ));
    }
    // SAFETY: `CUIAutomation` is the CLSID of the in-process UI
    // Automation object, `None` asks for no aggregation, and `T` is
    // inferred as `IUIAutomation`, which is an interface that class is
    // documented to implement — so the pointer this hands back really
    // is of the type the binding claims.
    unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) }.map_err(|err| {
        fault::win32(
            "start UI Automation on this machine",
            "take a `desktop.screenshot` of the window instead; this machine's UI Automation \
             service is not answering",
            &err,
        )
    })
}

/// The bounded breadth-first walk, and the only place a ref is minted.
fn walk(walker: &IUIAutomationTreeWalker, root: &IUIAutomationElement, depth: u32) -> Vec<Node> {
    let mut found: Vec<Node> = Vec::new();
    let mut frontier: Vec<(IUIAutomationElement, u32)> = vec![(root.clone(), 0)];
    while let Some((element, level)) = frontier.pop() {
        if found.len() >= MOST_REFS_PER_SNAPSHOT {
            break;
        }
        if let Some(node) = described(&element, level, found.len()) {
            found.push(node);
        }
        if level >= depth {
            continue;
        }
        for child in children(walker, &element) {
            frontier.push((child, level.saturating_add(1)));
        }
    }
    found
}

/// One element's children, in the order the walker reports them.
#[expect(
    unsafe_code,
    reason = "walking a UI Automation tree is a pair of COM calls, each an FFI call"
)]
fn children(
    walker: &IUIAutomationTreeWalker,
    parent: &IUIAutomationElement,
) -> Vec<IUIAutomationElement> {
    let mut found: Vec<IUIAutomationElement> = Vec::new();
    // SAFETY: `walker` and `parent` are live interface pointers this
    // call owns. An element with no children answers with a failure
    // rather than with a null this code would dereference, which is why
    // the `Result` is turned into `None` here.
    let mut next = unsafe { walker.GetFirstChildElement(parent) }.ok();
    while let Some(child) = next {
        if found.len() >= MOST_REFS_PER_SNAPSHOT {
            break;
        }
        // SAFETY: `child` is the live element from the line above, still
        // owned by this loop; the walker is unchanged. The end of the
        // sibling list arrives as a failure, which ends the loop.
        next = unsafe { walker.GetNextSiblingElement(&child) }.ok();
        found.push(child);
    }
    found
}

/// One element as a caller reads it, or `None` when it is not something
/// a caller could act on.
#[expect(
    unsafe_code,
    reason = "each property of an automation element is read through a COM vtable, which is an FFI call"
)]
fn described(element: &IUIAutomationElement, level: u32, minted: usize) -> Option<Node> {
    // SAFETY: `element` is a live interface pointer this walk owns, and
    // the three reads below share this one precondition. Each is a
    // property read: it allocates its own result and borrows nothing
    // from us, and it answers with a failure for a control that has gone
    // away rather than with a dangling value.
    let role = unsafe { element.CurrentLocalizedControlType() }.ok()?;
    let name = unsafe { element.CurrentName() }.ok()?;
    let rect = unsafe { element.CurrentBoundingRectangle() }.ok()?;
    let bounds = Bounds::from_corners(rect.left, rect.top, rect.right, rect.bottom).ok()?;
    let name = name.to_string();
    let role = role.to_string();
    if name.is_empty() && role.is_empty() {
        return None;
    }
    Some(Node {
        reference: format!("e{}", minted.saturating_add(1)),
        role,
        name,
        bounds,
        depth: level,
    })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// A window that is not there has no tree, and that is a refusal
    /// naming a next step rather than a crash. This is the one thing
    /// about this module that holds on a machine with no desktop
    /// (desktop-SPEC.md §16.2).
    #[test]
    fn a_handle_that_names_no_window_is_refused_with_a_next_step() {
        let refusal = read(HWND(std::ptr::null_mut()), 2)
            .expect_err("no window, no tree, and no pretending otherwise");
        let error = refusal.as_error();
        assert_eq!(error["data"]["code"], "E_TOOL_UNAVAILABLE");
        assert!(
            error["data"]["recovery"]
                .as_str()
                .unwrap()
                .contains("desktop.screenshot")
        );
    }

    /// Entering the apartment twice on one thread is what this server's
    /// second snapshot does, and it is a success rather than a failure.
    #[test]
    fn joining_the_apartment_twice_on_one_thread_is_not_a_failure() {
        assert!(start().is_ok());
        assert!(start().is_ok());
    }
}
