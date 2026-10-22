//! Preserve egui's IME controls using winit's explicit capability requests.
use winit::{
    dpi::{PhysicalPosition, PhysicalSize, Position, Size},
    window::{
        ImeCapabilities, ImeEnableRequest, ImeHint, ImePurpose, ImeRequest, ImeRequestData, Window,
    },
};

pub fn set_allowed(window: &dyn Window, allowed: bool) {
    let request = if allowed {
        let capabilities = ImeCapabilities::new()
            .with_hint_and_purpose()
            .with_cursor_area();
        let data = ImeRequestData::default()
            .with_hint_and_purpose(ImeHint::NONE, ImePurpose::Normal)
            .with_cursor_area(
                PhysicalPosition::new(0, 0).into(),
                PhysicalSize::new(0, 0).into(),
            );
        let Some(request) = ImeEnableRequest::new(capabilities, data) else {
            return;
        };
        ImeRequest::Enable(request)
    } else {
        ImeRequest::Disable
    };
    let _ = window.request_ime_update(request);
}

pub fn set_purpose(window: &dyn Window, purpose: ImePurpose) {
    if window
        .ime_capabilities()
        .is_some_and(|caps| caps.hint_and_purpose())
    {
        let _ = window.request_ime_update(ImeRequest::Update(
            ImeRequestData::default().with_hint_and_purpose(ImeHint::NONE, purpose),
        ));
    }
}

pub fn set_cursor_area(window: &dyn Window, position: Position, size: Size) {
    if window
        .ime_capabilities()
        .is_some_and(|caps| caps.cursor_area())
    {
        let _ = window.request_ime_update(ImeRequest::Update(
            ImeRequestData::default().with_cursor_area(position, size),
        ));
    }
}
