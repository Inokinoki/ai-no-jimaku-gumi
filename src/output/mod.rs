use crate::utils::Subtitle;

pub mod ffmpeg_subtitle;
pub mod srt;

pub trait OutputSubtitles {
    fn output_subtitles(&mut self, subtitles: &[Subtitle]);
}
