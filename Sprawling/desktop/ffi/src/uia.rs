// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationTreeWalker, UIA_CONTROLTYPE_ID,
};
use windows::core::Error;

#[derive(Clone)]
pub struct Automation(IUIAutomation);

pub struct Walker(IUIAutomationTreeWalker);

#[derive(Clone)]
pub struct Element(IUIAutomationElement);

impl Automation {
    pub fn direct() -> Result<Automation, Error> {
        // SAFETY: CUIAutomation is a static CLSID valid for the whole call; windows_core owns the returned interface.
        #[expect(unsafe_code, reason = "CoCreateInstance admits no safe interface")]
        let inner: IUIAutomation = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_ALL) }?;
        Ok(Automation(inner))
    }

    pub fn control_view_walker(&self) -> Result<Walker, Error> {
        // SAFETY: self.0 is a live automation the Reader owns; UIA transfers ownership of the returned walker.
        #[expect(unsafe_code, reason = "the UIA call admits no safe interface")]
        let inner = unsafe { self.0.ControlViewWalker() }?;
        Ok(Walker(inner))
    }

    pub fn element_from_handle(&self, window: HWND) -> Result<Element, Error> {
        // SAFETY: self.0 is a live automation the Reader owns; the HWND value is borrowed for this one call.
        #[expect(unsafe_code, reason = "the UIA call admits no safe interface")]
        let inner = unsafe { self.0.ElementFromHandle(window) }?;
        Ok(Element(inner))
    }
}

impl Walker {
    pub fn first_child(&self, parent: &Element) -> Result<Option<Element>, Error> {
        // SAFETY: self.0 and parent.0 are live for this one call; an S_OK-null answer reads as exhaustion below.
        #[expect(unsafe_code, reason = "the walker call admits no safe interface")]
        let step = unsafe { self.0.GetFirstChildElement(&parent.0) };
        match step {
            Ok(element) => Ok(Some(Element(element))),
            Err(err) if exhausted(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn next_sibling(&self, element: &Element) -> Result<Option<Element>, Error> {
        // SAFETY: as first_child, for the next sibling in place of the first child.
        #[expect(unsafe_code, reason = "the walker call admits no safe interface")]
        let step = unsafe { self.0.GetNextSiblingElement(&element.0) };
        match step {
            Ok(element) => Ok(Some(Element(element))),
            Err(err) if exhausted(&err) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl Element {
    pub fn name(&self) -> Result<String, Error> {
        // SAFETY: self.0 is a live element; the returned BSTR is an out-value converted and dropped here.
        #[expect(unsafe_code, reason = "the UIA call admits no safe interface")]
        let raw = unsafe { self.0.CurrentName() }?;
        Ok(raw.to_string())
    }

    pub fn control_type(&self) -> Result<UIA_CONTROLTYPE_ID, Error> {
        // SAFETY: as name, with an integer out-value in place of the BSTR.
        #[expect(unsafe_code, reason = "the UIA call admits no safe interface")]
        let raw = unsafe { self.0.CurrentControlType() }?;
        Ok(raw)
    }

    pub fn bounds(&self) -> Result<RECT, Error> {
        // SAFETY: as name, with a struct out-value in place of the BSTR.
        #[expect(unsafe_code, reason = "the UIA call admits no safe interface")]
        let raw = unsafe { self.0.CurrentBoundingRectangle() }?;
        Ok(raw)
    }
}

fn exhausted(err: &Error) -> bool {
    err.code().is_ok()
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
    use windows::Win32::Foundation::E_POINTER;

    #[test]
    fn an_s_ok_null_answer_is_exhaustion_and_a_real_hresult_is_a_fault() {
        assert!(exhausted(&Error::empty()));
        assert!(!exhausted(&Error::from_hresult(E_POINTER)));
    }
}
