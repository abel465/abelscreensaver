use libmpv::events::Event as MPVEvent;
use libmpv2 as libmpv;
use std::path::Path;

pub struct MpvClient {
    mpv: libmpv::Mpv,
}

impl MpvClient {
    pub fn new(mpv: libmpv::Mpv) -> Self {
        MpvClient { mpv }
    }

    pub fn playlist_prev(&self) {
        self.mpv.command("playlist-prev", &[]).ok();
    }

    pub fn playlist_next(&self) {
        self.mpv.command("playlist-next", &[]).ok();
    }

    pub fn playlist_append_play(&self, path: &Path) {
        self.mpv
            .command("loadfile", &[&path.to_str().unwrap(), "append-play"])
            .unwrap();
    }

    pub fn playlist_append(&self, path: &Path) {
        self.mpv
            .command("loadfile", &[&path.to_str().unwrap(), "append"])
            .unwrap();
    }

    pub fn next_event(&mut self) -> Option<libmpv::Result<MPVEvent>> {
        self.mpv.event_context_mut().wait_event(0.0)
    }

    pub fn set_mute(&self, mute: bool) {
        self.mpv.set_property("mute", mute).unwrap();
    }

    pub fn set_pause(&self, pause: bool) {
        self.mpv.set_property("pause", pause).unwrap();
    }

    pub fn get_path(&self) -> String {
        self.mpv.get_property("path").unwrap()
    }
}
