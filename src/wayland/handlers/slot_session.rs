use crate::{
    state::State,
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
}

delegate_slot_session!(State);
