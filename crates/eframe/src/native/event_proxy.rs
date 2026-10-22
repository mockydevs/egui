//! Keep typed eframe events beside winit's coalescing wake-up notification.
use super::winit_integration::UserEvent;
use egui::mutex::Mutex;
use std::sync::Arc;
#[derive(Clone)]
pub(crate) struct UserEventProxy {
    proxy: winit::event_loop::EventLoopProxy,
    events: Arc<Mutex<Vec<UserEvent>>>,
}
impl UserEventProxy {
    pub fn new(proxy: winit::event_loop::EventLoopProxy) -> Self {
        Self {
            proxy,
            events: Default::default(),
        }
    }
    pub fn send_event(&self, event: UserEvent) {
        self.events.lock().push(event);
        self.proxy.wake_up();
    }
    pub fn take(&self) -> Vec<UserEvent> {
        core::mem::take(&mut *self.events.lock())
    }
}
