use clap::Parser;

use tokio;

mod output;
mod translate;
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

    /// Video start time
    #[arg(long, default_value = "0")]
    start_time: usize,

    /// Video end time
    #[arg(long, default_value = "0")]
    end_time: usize,

    /// Subtitle backend
    /// (default: "srt")
    /// (possible values: "srt", "container", "embedded")
    /// (example: "srt")
    /// (long_about: "Subtitle backend to use")
    #[arg(long, default_value = "srt")]
    subtitle_backend: String,

    /// Subtitle output path (if srt)
    /// (default: "output.srt")
    /// (example: "output.srt")
    /// (long_about: "Subtitle output path (if srt)")
    #[arg(long, default_value = "output.srt")]
    subtitle_output_path: String,

    /// Translator backend
    /// (default: "deepl")
    /// (possible values: "deepl", "google", "openai")
    /// (example: "google")
    /// (long_about: "Translator backend to use")
    #[arg(long, default_value = "deepl")]
    translator_backend: String,

    /// Subtitle source
    /// (default: "audio")
    /// (possible values: "audio", "container", "ocr")
    /// (example: "audio")
    /// (long_about: "Subtitle source to use")
    #[arg(long, default_value = "audio")]
    subtitle_source: String,

    /// Original subtitle SRT file path
    /// (default: "")
    /// (example: "origin.srt")
    /// (long_about: "Original subtitle path to save the transcripted subtitle as SRT")
    #[arg(long, default_value = "")]
    original_subtitle_path: String,
}

fn main() {
    let args = Args::parse();
    let input_video_path = args.input_video_path;
    let source_language = args.source_language;
    let target_language = args.target_language;

    println!("Hello, AI no jimaku gumi!");

    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().join("audio.wav");
    let tmp_path_str = tmp_path.as_os_str().to_str().unwrap();
    utils::ffmpeg_audio::extract_audio_from_video(&input_video_path, tmp_path_str, 16000);
    let state = whisper::experiment::extract_from_f32_16khz_wav_audio(
        "ggml-tiny.bin",
        tmp_path_str,
        &source_language,
    );

    let mut subtitles = utils::whisper_state::create_subtitle_from_whisper_state(&state);
    if subtitles.is_empty() {
        println!("No subtitles found");
        return;
    }

    if !args.original_subtitle_path.is_empty() {
        // Save original subtitles
        let tmp_path = args.original_subtitle_path;
        let file = std::fs::File::create(tmp_path).unwrap();
        let mut exporter = output::srt::SrtSubtitleExporter::new(file);
        exporter.output_subtitles(&subtitles);
    }

    if args.translator_backend == "deepl" {
        let deepl_api_key = std::env::var("DEEPL_API_KEY").unwrap();
        if deepl_api_key.is_empty() {
            println!("DEEPL_API_KEY is not set");
            return;
        }

        let rt = tokio::runtime::Runtime::new().unwrap();
        subtitles.iter_mut().for_each(|s| {
            s.text = rt
                .block_on(translate::deepl::translate_text(
                    deepl_api_key.as_str(),
                    vec![s.text.as_str()],
                    target_language.as_str(),
                    Some(source_language.as_str()),
                ))
                .unwrap();
        });
    } else {
        println!("Unsupported translator backend now");
        return;
    }

    if args.subtitle_backend == "srt" {
        let tmp_path = args.subtitle_output_path;
        let file = std::fs::File::create(tmp_path).unwrap();
        let mut exporter = output::srt::SrtSubtitleExporter::new(file);
        exporter.output_subtitles(&subtitles);
    } else {
        println!("Unsupported subtitle backend now");
        return;
    }
}
