use clap::Parser;

mod output;
mod utils;
mod whisper;

use output::OutputSubtitles;
use tempfile::TempDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    input_video_path: String,

    /// Which language to translate from
    /// (default: "ja")
    /// (possible values: "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh")
    /// (example: "ja")
    #[arg(short, long, default_value = "ja")]
    source_language: String,

    /// Which language to translate to
    /// (default: "en")
    /// (possible values: "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh")
    /// (example: "en")
    #[arg(short, long, default_value = "en")]
    target_language: String,
}

fn main() {
    let args = Args::parse();
    let input_video_path = args.input_video_path;
    let source_language = args.source_language;
    // TODO: use in the future
    let _target_language = args.target_language;

    println!("Hello, AI no jimaku gumi!");

    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().join("audio.wav");
    let tmp_path_str = tmp_path.as_os_str().to_str().unwrap();
    utils::ffmpeg_audio::extract_audio_from_video(&input_video_path, tmp_path_str, 16000);
    let _state = whisper::experiment::extract_from_f32_16khz_wav_audio(
        "ggml-tiny.bin",
        tmp_path_str,
        &source_language,
    );

    let subtitles = output::whisper_state::create_subtitle_from_whisper_state(&_state);
    if subtitles.is_empty() {
        println!("No subtitles found");
        return;
    }

    let tmp_path = "output.srt";
    let file = std::fs::File::create(tmp_path).unwrap();
    let mut exporter = output::srt::SrtSubTitleExporter::new(file);
    exporter.output_subtitles(subtitles);
}
