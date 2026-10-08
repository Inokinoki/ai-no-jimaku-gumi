use crate::utils::Subtitle;

pub mod ffmpeg_embedded;
pub mod ffmpeg_subtitle;
pub mod srt;

pub trait OutputSubtitles {
    fn output_subtitles(&mut self, subtitles: &[Subtitle]);
}

// Build a sibling temporary file name for an in-place export, keeping the
// container extension so that ffmpeg can still guess the format from it
pub(crate) fn temp_sibling(path: &str) -> String {
    let file_path = std::path::Path::new(path);
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output");
    let temp_name = match file_name.rsplit_once('.') {
        Some((stem, extension)) => format!("{}.tmp.{}", stem, extension),
        None => format!("{}.tmp", file_name),
    };
    file_path
        .with_file_name(temp_name)
        .to_str()
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}.tmp", path))
}
