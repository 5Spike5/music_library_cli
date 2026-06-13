use std::fs::File;
use std::path::Path;

use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};

pub struct RodioPlayer {
    _sink_handle: MixerDeviceSink,
    player: Player,
}

impl RodioPlayer {
    pub fn new() -> Result<Self, String> {
        let mut sink_handle = DeviceSinkBuilder::open_default_sink()
            .map_err(|err| format!("failed to open audio output device: {err}"))?;
        sink_handle.log_on_drop(false);

        let player = Player::connect_new(sink_handle.mixer());

        Ok(Self {
            _sink_handle: sink_handle,
            player,
        })
    }

    pub fn play_file(&self, path: &Path) -> Result<(), String> {
        let file = File::open(path)
            .map_err(|err| format!("failed to open file `{}`: {err}", path.display()))?;
        let source = Decoder::try_from(file)
            .map_err(|err| format!("failed to decode `{}`: {err}", path.display()))?;

        self.player.stop();
        self.player.append(source);
        self.player.play();
        Ok(())
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn resume(&self) {
        self.player.play();
    }

    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn is_paused(&self) -> bool {
        self.player.is_paused()
    }

    pub fn is_empty(&self) -> bool {
        self.player.empty()
    }
}
