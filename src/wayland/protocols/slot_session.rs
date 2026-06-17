// SPDX-License-Identifier: GPL-3.0-only

pub use generated::slot_session;

#[allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]
mod generated {
    use smithay::reexports::wayland_server;

    pub mod __interfaces {
        use wayland_backend;
        wayland_scanner::generate_interfaces!("resources/protocols/slot_session.xml");
    }
    use self::__interfaces::*;

    wayland_scanner::generate_server_code!("resources/protocols/slot_session.xml");
}

use std::collections::HashMap;

use smithay::{
    output::Output,
    reexports::wayland_server::{
        Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, backend::GlobalId,
        protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle, Size},
};
use wayland_backend::{protocol::WEnum, server::ClientId};

use crate::utils::prelude::Local;

use self::slot_session::SlotSession as SlotSessionResource;

pub trait SlotSessionHandler {
    fn slot_session_state(&mut self) -> &mut SlotSessionState;
    fn slot_session_finished(&mut self);
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum PointerMode {
    #[default]
    Hidden,
    Interactive,
}

#[derive(Debug)]
struct ActiveSession {
    live: HashMap<u32, Rectangle<i32, Local>>,
    staged: HashMap<u32, Rectangle<i32, Local>>,
    pointer_mode: PointerMode,
    captured: Option<CapturedSlot>,
    pending_capture_toggle: bool,
}

#[derive(Debug, Clone)]
pub struct CapturedSlot {
    pub surface: WlSurface,
    pub rect: Rectangle<i32, Local>,
    pub output: Output,
}

#[derive(Debug, Clone, Copy)]
pub struct SlotOutputConfig {
    pub mode_size: Size<u16, Logical>,
    pub refresh: u32,
}

#[derive(Debug)]
pub struct SlotSessionState {
    _global: Option<GlobalId>,
    output_config: Option<SlotOutputConfig>,
    session: Option<ActiveSession>,
}

impl SlotSessionState {
    pub fn new<D, F>(
        dh: &DisplayHandle,
        output_config: Option<SlotOutputConfig>,
        client_filter: F,
    ) -> SlotSessionState
    where
        D: GlobalDispatch<SlotSessionResource, SlotSessionGlobalData>
            + Dispatch<SlotSessionResource, ()>
            + SlotSessionHandler
            + 'static,
        F: for<'a> Fn(&'a Client) -> bool + Send + Sync + 'static,
    {
        let global = output_config.is_some().then(|| {
            dh.create_global::<D, SlotSessionResource, _>(
                1,
                SlotSessionGlobalData {
                    filter: Box::new(client_filter),
                },
            )
        });

        SlotSessionState {
            _global: global,
            output_config,
            session: None,
        }
    }

    pub fn output_config(&self) -> Option<SlotOutputConfig> {
        self.output_config
    }

    pub fn is_slot_mode(&self) -> bool {
        self.output_config.is_some()
    }

    fn create_session(&mut self) {
        self.session = Some(ActiveSession {
            live: HashMap::new(),
            staged: HashMap::new(),
            pointer_mode: PointerMode::Hidden,
            captured: None,
            pending_capture_toggle: false,
        });
    }

    fn set_slot(&mut self, slot: u32, x: i32, y: i32, width: u32, height: u32) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        session.staged.insert(
            slot,
            Rectangle::new(
                Point::from((x, y)),
                Size::from((width as i32, height as i32)),
            ),
        );
    }

    fn set_option(&mut self, option: WEnum<slot_session::SessionOption>, value: u32) {
        let Some(session) = self.session.as_mut() else {
            return;
        };

        if let WEnum::Value(slot_session::SessionOption::PointerMode) = option {
            session.pointer_mode = if value == 0 {
                PointerMode::Hidden
            } else {
                PointerMode::Interactive
            };
        }
    }

    fn commit_session(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };

        session.live = std::mem::take(&mut session.staged);
    }

    pub fn pointer_mode(&self) -> PointerMode {
        self.session
            .as_ref()
            .map(|session| session.pointer_mode)
            .unwrap_or_default()
    }

    pub fn slot_rect_for_instance_id(
        &self,
        instance_id: &str,
    ) -> Option<Rectangle<i32, Local>> {
        let session = self.session.as_ref()?;
        let slot = instance_id
            .strip_prefix("partydeck-slot-")
            .and_then(|n| n.parse::<u32>().ok())?;
        session.live.get(&slot).copied()
    }

    pub fn captured(&self) -> Option<CapturedSlot> {
        self.session
            .as_ref()
            .and_then(|session| session.captured.clone())
    }

    pub fn is_captured(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.captured.is_some())
    }

    pub fn set_captured(&mut self, captured: CapturedSlot) {
        if let Some(session) = self.session.as_mut() {
            session.captured = Some(captured);
        }
    }

    pub fn clear_captured(&mut self) {
        if let Some(session) = self.session.as_mut() {
            session.captured = None;
        }
    }

    pub fn pending_capture_toggle(&self) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| session.pending_capture_toggle)
    }

    pub fn set_pending_capture_toggle(&mut self, pending: bool) {
        if let Some(session) = self.session.as_mut() {
            session.pending_capture_toggle = pending;
        }
    }

    fn finish_session(&mut self) -> bool {
        self.session.take().is_some()
    }
}

pub struct SlotSessionGlobalData {
    filter: Box<dyn for<'a> Fn(&'a Client) -> bool + Send + Sync>,
}

impl<D> GlobalDispatch<SlotSessionResource, SlotSessionGlobalData, D> for SlotSessionState
where
    D: GlobalDispatch<SlotSessionResource, SlotSessionGlobalData>
        + Dispatch<SlotSessionResource, ()>
        + SlotSessionHandler
        + 'static,
{
    fn bind(
        state: &mut D,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<SlotSessionResource>,
        _global_data: &SlotSessionGlobalData,
        data_init: &mut DataInit<'_, D>,
    ) {
        data_init.init(resource, ());
        state.slot_session_state().create_session();
    }

    fn can_view(client: Client, global_data: &SlotSessionGlobalData) -> bool {
        (global_data.filter)(&client)
    }
}

impl<D> Dispatch<SlotSessionResource, (), D> for SlotSessionState
where
    D: Dispatch<SlotSessionResource, ()> + SlotSessionHandler + 'static,
{
    fn request(
        state: &mut D,
        _client: &Client,
        _resource: &SlotSessionResource,
        request: slot_session::Request,
        _data: &(),
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            slot_session::Request::SetSlot {
                slot,
                x,
                y,
                width,
                height,
            } => {
                state.slot_session_state().set_slot(slot, x, y, width, height);
            }
            slot_session::Request::SetOption { option, value } => {
                state.slot_session_state().set_option(option, value);
            }
            slot_session::Request::Commit => {
                state.slot_session_state().commit_session();
            }
            slot_session::Request::End => {}
        }
    }

    fn destroyed(
        state: &mut D,
        _client: ClientId,
        _resource: &SlotSessionResource,
        _data: &(),
    ) {
        let removed = state.slot_session_state().finish_session();
        if removed {
            state.slot_session_finished();
        }
    }
}

macro_rules! delegate_slot_session {
    ($(@<$( $lt:tt $( : $clt:tt $(+ $dlt:tt )* )? ),+>)? $ty: ty) => {
        smithay::reexports::wayland_server::delegate_global_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland::protocols::slot_session::slot_session::SlotSession: $crate::wayland::protocols::slot_session::SlotSessionGlobalData
        ] => $crate::wayland::protocols::slot_session::SlotSessionState);
        smithay::reexports::wayland_server::delegate_dispatch!($(@< $( $lt $( : $clt $(+ $dlt )* )? ),+ >)? $ty: [
            $crate::wayland::protocols::slot_session::slot_session::SlotSession: ()
        ] => $crate::wayland::protocols::slot_session::SlotSessionState);
    };
}
pub(crate) use delegate_slot_session;
