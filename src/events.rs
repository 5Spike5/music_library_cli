#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaybackEvent {
    Started(usize),
    Paused,
    Resumed,
    Stopped,
    Finished(usize),
    SwitchedTo(usize),
    Error(String),
}
