pub mod ffmpeg_audio;
pub mod whisper_state;

pub struct Subtitle {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

impl Subtitle {
    pub fn new(start: f32, end: f32, text: String) -> Subtitle {
        Subtitle { start, end, text }
    }
}
