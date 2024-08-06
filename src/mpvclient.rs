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
        let playlist_pos = self.mpv.get_property::<i64>("playlist-pos").unwrap();
        if playlist_pos != 0 {
            self.mpv.command("playlist-prev", &[]).ok();
        }
    }

    pub fn playlist_next(&self) {
        self.mpv.command("playlist-next", &[]).ok();
    }

    pub fn playlist_append_play(&self, path: &Path) {
        let quoted = format!("'{}'", path.to_str().unwrap());
        self.mpv
            .command("loadfile", &[&quoted, "append-play"])
            .unwrap();
    }

    pub fn playlist_append(&self, path: &Path) {
        let quoted = format!("'{}'", path.to_str().unwrap());
        self.mpv.command("loadfile", &[&quoted, "append"]).unwrap();
    }

    pub fn playlist_replace(&self, path: &Path) {
        let quoted = format!("'{}'", path.to_str().unwrap());
        self.mpv.command("loadfile", &[&quoted]).unwrap();
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

    pub fn set_image_duration(&self, duration_secs: f64) {
        self.mpv
            .set_property("image-display-duration", duration_secs)
            .unwrap();
    }

    /// Clear the playlist, except the currently played file.
    pub fn playlist_clear(&self) {
        self.mpv.command("playlist-clear", &[]).unwrap();
    }
}
