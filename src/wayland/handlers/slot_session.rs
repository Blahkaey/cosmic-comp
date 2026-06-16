use smithay::utils::Rectangle;

use crate::{
    shell::WeakCosmicSurface,
    state::State,
    utils::prelude::Local,
    wayland::protocols::slot_session::{
        SlotSessionHandler, SlotSessionState, delegate_slot_session,
    },
};
impl SlotSessionHandler for State {
    fn slot_session_state(&mut self) -> &mut SlotSessionState {
        &mut self.common.slot_session_state
    }

    fn slot_session_finished(&mut self) {
        self.common.event_loop_signal.stop();
        self.common.event_loop_signal.wakeup();
    }

    fn slot_session_committed(
        &mut self,
        reconfigure: Vec<(WeakCosmicSurface, Rectangle<i32, Local>)>,
    ) {
        if reconfigure.is_empty() {
            return;
        }
        let mut shell = self.common.shell.write();
        let output = shell
            .outputs()
            .next()
            .cloned()
            .expect("nested output present");
        for (window, rect) in reconfigure {
            if let Some(surface) = window.upgrade() {
                shell.reconfigure_session_slot(&surface, &output, rect);
                self.common
                    .slot_session_state
                    .update_captured_rect(&surface, rect);
            }
        }
    }
}

delegate_slot_session!(State);
