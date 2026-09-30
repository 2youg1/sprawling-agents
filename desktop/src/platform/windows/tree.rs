// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The UI Automation tree of one window: role, name, ref and bounds,
//! read through `uiautomation`'s safe calls (desktop-SPEC.md section
//! 12.10).
//!
//! **Every element this module reads dies inside the call that read
//! it.** A `ref` handed back to a caller is a rectangle and a
//! generation, never a live element (desktop-SPEC.md §8.6, second pair),
//! so a connection that snapshots a thousand windows holds a thousand
//! rectangles rather than a thousand cross-process interface pointers.
//! What does outlive a call is the [`Reader`]: the COM apartment, the
//! automation object and the walker one desk reads every tree with,
//! entered once and left once.
//!
//! The walk is bounded twice — by the depth the caller asked for and by
//! [`views::MOST_REFS_PER_SNAPSHOT`] — because a UI Automation tree has
//! no promise of being finite in any practical sense: one list control
//! can present tens of thousands of items, and a snapshot a model
//! cannot read to the end is a snapshot it has not read.
//!
//! A node with neither a name nor a usable rectangle is skipped and its
//! children are not: a layout container a caller cannot act on should
//! not cost a ref, and should not hide what is inside it either.
//!
//! [`views::MOST_REFS_PER_SNAPSHOT`]: super::views::MOST_REFS_PER_SNAPSHOT

use uiautomation::types::Handle;
use uiautomation::{UIAutomation, UIElement, UITreeWalker};
use winsafe::co;
use winsafe::guard::CoUninitializeGuard;

use super::fault;
use super::geometry::Bounds;
use super::views::{MOST_REFS_PER_SNAPSHOT, Node};
use crate::refusal::Refusal;

/// What one desk reads trees with.
///
/// The fields drop in the order they are declared, and that order is
/// the rule: the walker and the automation object are COM objects, so
/// they go before the apartment they live in is left.
pub(crate) struct Reader {
    walker: UITreeWalker,
    automation: UIAutomation,
    _apartment: CoUninitializeGuard,
}

impl Reader {
    /// Enters this thread's apartment and starts UI Automation.
    ///
    /// The multithreaded apartment, because this thread opens no window
    /// that COM would have to pump messages for, which is the case
    /// Microsoft recommends it for (desktop-SPEC.md section 12.10).
    ///
    /// # Errors
    /// Refuses when this thread cannot enter an apartment, and when UI
    /// Automation will not start or will not open a walk.
    pub(crate) fn start() -> Result<Reader, Refusal> {
        let apartment = winsafe::CoInitializeEx(co::COINIT::MULTITHREADED).map_err(|err| {
            fault::com(
                "join this thread to the desktop's automation apartment",
                "restart this server; a thread that cannot enter an apartment cannot read any \
                 tree",
                err,
            )
        })?;
        let automation = UIAutomation::new_direct().map_err(|err| {
            fault::automation(
                "start UI Automation on this machine",
                "take a `desktop.screenshot` of the window instead; this machine's UI \
                 Automation service is not answering",
                &err,
            )
        })?;
        let walker = automation.get_control_view_walker().map_err(|err| {
            fault::automation(
                "open a walk over the window's tree",
                "take a `desktop.screenshot` of this window instead",
                &err,
            )
        })?;
        Ok(Reader {
            walker,
            automation,
            _apartment: apartment,
        })
    }

    /// Reads one window's tree.
    ///
    /// # Errors
    /// Refuses when the window has no automation element. A single node
    /// that will not answer is skipped rather than refused, because one
    /// unreadable control is not a reason to report no window.
    pub(crate) fn read(&self, window: &winsafe::HWND, depth: u32) -> Result<Vec<Node>, Refusal> {
        let handle = Handle::from(windows::Win32::Foundation::HWND(window.ptr()));
        let root = self.automation.element_from_handle(handle).map_err(|err| {
            fault::automation(
                "read the window's accessibility tree",
                "call `desktop.windows` again; a window that has closed has no tree, and a \
                 window drawn without accessibility information cannot be read this way — use \
                 `desktop.screenshot` to see it instead",
                &err,
            )
        })?;
        Ok(self.walk(&root, depth))
    }

    /// The bounded walk, and the only place a ref is minted.
    fn walk(&self, root: &UIElement, depth: u32) -> Vec<Node> {
        let mut found: Vec<Node> = Vec::new();
        let mut frontier: Vec<(UIElement, u32)> = vec![(root.clone(), 0)];
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
            for child in self.children(&element) {
                frontier.push((child, level.saturating_add(1)));
            }
        }
        found
    }

    /// One element's children, in the order the walker reports them.
    fn children(&self, parent: &UIElement) -> Vec<UIElement> {
        let mut found: Vec<UIElement> = Vec::new();
        let mut next = self.walker.get_first_child(parent).ok();
        while let Some(child) = next {
            if found.len() >= MOST_REFS_PER_SNAPSHOT {
                break;
            }
            next = self.walker.get_next_sibling(&child).ok();
            found.push(child);
        }
        found
    }
}

/// One element as a caller reads it, or `None` when it is not something
/// a caller could act on.
fn described(element: &UIElement, level: u32, minted: usize) -> Option<Node> {
    let role = element.get_localized_control_type().ok()?;
    let name = element.get_name().ok()?;
    let rect = element.get_bounding_rectangle().ok()?;
    let bounds = Bounds::from_corners(
        rect.get_left(),
        rect.get_top(),
        rect.get_right(),
        rect.get_bottom(),
    )
    .ok()?;
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
    use super::super::enumerate;
    use super::super::fixture::Opened;
    use super::*;
    use winsafe::prelude::Handle as _;

    /// The contract desktop-SPEC.md section 8-11 holds the tree to,
    /// whichever interface reads it: a window's tree starts at the window,
    /// named by its title, and names the control inside it one level
    /// down. Roles are not compared, because they are a word this server
    /// chooses rather than a fact the window reports.
    #[test]
    fn a_windows_tree_names_the_control_inside_it() {
        let title = format!("sprawling contract tree {}", std::process::id());
        let _opened = Opened::at(&title, -20_000, -20_000, Some("Press"));
        let window = enumerate::desktop()
            .unwrap()
            .into_iter()
            .find(|window| window.named.title == title)
            .unwrap();
        let nodes = Reader::start().unwrap().read(&window.handle, 2).unwrap();
        let named: Vec<(&str, u32)> = nodes
            .iter()
            .map(|node| (node.name.as_str(), node.depth))
            .collect();
        assert_eq!(named, vec![(title.as_str(), 0), ("Press", 1)]);
    }

    /// A window that is not there has no tree, and that is a refusal
    /// naming a next step rather than a crash. This is the one thing
    /// about this module that holds on a machine with no desktop
    /// (desktop-SPEC.md §16.2).
    #[test]
    fn a_handle_that_names_no_window_is_refused_with_a_next_step() {
        let refusal = Reader::start()
            .unwrap()
            .read(&winsafe::HWND::NULL, 2)
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

    /// Two readers on one thread are two entries into one apartment,
    /// each left when its reader goes: a connection that is served
    /// twice on one thread does not find the apartment already left.
    #[test]
    fn two_readers_on_one_thread_both_start() {
        let first = Reader::start();
        let second = Reader::start();
        assert!(first.is_ok() && second.is_ok());
    }
}
