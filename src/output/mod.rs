use crate::utils::Subtitle;

pub mod srt;

pub trait OutputSubtitles {
    fn output_subtitles(&mut self, subtitles: &Vec<Subtitle>);
}
