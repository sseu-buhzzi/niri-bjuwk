use niri_ipc::{Action, Event, Reply, Request, Response, Window, Workspace, socket::Socket};

use crate::error::{BjuwkError, BjuwkResult, IoContextExt};

pub struct NiriIpcWrapper(Socket);

impl NiriIpcWrapper {
    pub fn new() -> BjuwkResult<Self> {
        Socket::connect()
            .map(Self)
            .map_err(|e| BjuwkError::Other(format!("Cannot connect to niri IPC: {e}")))
    }

    pub fn get_focused_window(&mut self) -> BjuwkResult<Option<Window>> {
        let reply = self.request(Request::FocusedWindow)?;
        match reply {
            Ok(Response::FocusedWindow(w)) => Ok(w),
            reply => Err(reply.into()),
        }
    }

    pub fn receive_workspaces_and_windows(mut self) -> BjuwkResult<(Vec<Workspace>, Vec<Window>)> {
        let reply = self.request(Request::EventStream)?;
        match reply {
            Ok(Response::Handled) => {}
            reply => return Err(reply.into()),
        }
        let mut read_events = self.0.read_events();
        let mut workspaces_ret = None;
        let mut windows_ret = None;
        let mut pending_count = 2u8;
        while let Ok(event) = read_events() {
            match event {
                Event::WorkspacesChanged { workspaces } => {
                    workspaces_ret = Some(workspaces);
                    pending_count -= 1;
                }
                Event::WindowsChanged { windows } => {
                    windows_ret = Some(windows);
                    pending_count -= 1;
                }
                _ => {}
            }
            if pending_count == 0 {
                break;
            }
        }
        Ok((
            workspaces_ret.unwrap_or_default(),
            windows_ret.unwrap_or_default(),
        ))
    }

    pub fn send_action(&mut self, action: Action) -> BjuwkResult<()> {
        println!("#### send_action {action:?}");
        std::thread::sleep(std::time::Duration::from_millis(1024));
        let reply = self.request(Request::Action(action))?;
        match reply {
            Ok(Response::Handled) => Ok(()),
            reply => Err(reply.into()),
        }
    }

    fn request(&mut self, request: Request) -> BjuwkResult<Reply> {
        let context = format!("{request:?}");
        let reply = self.0.send(request).context(context)?;
        Ok(reply)
    }
}

pub type WorkspaceId = u64;
pub type WindowId = u64;
