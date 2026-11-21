//! Agent control left out of the build (the `agent-control` feature off, as in the mobile
//! build): the socket never opens, so no script or `horizon-ctl` reaches this game. The
//! names the rest of the code calls stay, doing nothing; control.rs is the real one.
//! `FrameStats` keeps its place in the frame loop but records nothing: only the control
//! commands, the benchmark and splitscreen's stats readout read it, and those are desktop
//! features too.
use crate::block_entity_render::StaticCacheFrameStats;
use std::{
    io,
    path::{Path, PathBuf},
    sync::mpsc,
};

/// One command line from a client, with the channel for its JSON reply.
pub struct Request {
    pub line: String,
    pub reply: mpsc::Sender<String>,
}

/// The control endpoint that never listens.
pub struct Server {
    requests: mpsc::Receiver<Request>,
    sender: mpsc::Sender<Request>,
}

impl Server {
    /// No socket in this build: a game started for an agent (`--control`) reports it.
    pub fn start(_path: &Path) -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this build has no agent control (the `agent-control` feature)",
        ))
    }
    /// Commands from inside the process only (`--benchmark` without `--control`); nothing
    /// answers them in this build.
    pub fn local() -> Self {
        let (sender, requests) = mpsc::channel();
        Self { requests, sender }
    }
    /// How many clients are connected: none ever.
    pub fn clients(&self) -> usize {
        0
    }
    /// Sends commands as a socket client would, from another thread.
    pub fn sender(&self) -> mpsc::Sender<Request> {
        self.sender.clone()
    }
    /// The endpoint's path: empty, as a local server's is.
    pub fn path(&self) -> &Path {
        Path::new("")
    }
    pub fn try_recv(&self) -> Option<Request> {
        self.requests.try_recv().ok()
    }
}

/// `$HORIZON_CONTROL`, else the endpoint's usual name; no socket is made of it in this build.
pub fn default_path() -> PathBuf {
    if let Some(path) = std::env::var_os("HORIZON_CONTROL") {
        return path.into();
    }
    if !cfg!(unix) {
        return std::env::temp_dir().join("horizon-control.json");
    }
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("horizon-control.sock")
}

/// Nothing to check: no endpoint is made or reached in this build.
pub fn check_path(_path: &Path) -> io::Result<()> {
    Ok(())
}

/// The frame loop's statistics, kept empty: no control command reads them in this build.
#[derive(Default)]
pub struct FrameStats;

impl FrameStats {
    pub fn record(&mut self, _millis: f32) {}
    pub fn record_cpu(&mut self, _millis: f32) {}
    pub fn record_phases(&mut self, _millis: [f32; 4]) {}
    pub fn record_gpu(&mut self, _millis: &[f32]) {}
    pub fn record_block_entities(&mut self, _stats: StaticCacheFrameStats) {}
    pub fn reset(&mut self) {}
    pub fn summary(&self) -> serde_json::Value {
        serde_json::json!({ "frames": 0 })
    }
}
