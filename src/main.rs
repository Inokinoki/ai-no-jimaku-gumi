use clap::Parser;

use tokio;

mod output;
mod translate;
mod utils;
mod whisper;

use genai::adapter::AdapterKind;
use genai::resolver::{AuthData, Endpoint, ServiceTargetResolver};
use genai::{Client, ModelIden, ServiceTarget};
use output::OutputSubtitles;
use tempfile::TempDir;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the input video
    #[arg(short, long)]
    input_video_path: String,

    /// Which language to translate from
    /// (default: "ja")
    /// (possible values: "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh")
    /// (example: "ja")
    #[arg(long, default_value = "ja")]
    source_language: String,

    /// Which language to translate to
    /// (default: "en")
    /// (possible values: "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh")
    /// (example: "en")
    #[arg(long, default_value = "en")]
    target_language: String,

    /// Video start time (not used yet)
    #[arg(long, default_value = "0")]
    start_time: usize,

    /// Video end time (not used yet)
    #[arg(long, default_value = "0")]
    end_time: usize,

    /// Subtitle source
    /// (default: "audio")
    /// (possible values: "audio", "container", "ocr")
    /// (example: "audio")
    /// (long_about: "Subtitle source to use")
    #[arg(long, default_value = "audio")]
    subtitle_source: String,

    /// ggml model path
    /// (default: "ggml-tiny.bin")
    /// (example: "ggml-tiny.bin", ggml-small.bin")
    /// (long_about: "Path to the ggml model")
    #[arg(long, default_value = "ggml-tiny.bin")]
    ggml_model_path: String,

    /// Only extract the audio
    /// (default: false)
    /// (long_about: "Only extract the audio, if subtitle source is audio, but do not transcribe (Debug purpose)")
    /// (example: true)
    #[arg(long)]
    only_extract_audio: bool,

    /// Only save the transcripted subtitle
    /// (default: false)
    /// (long_about: "Only save the transcripted subtitle but do not translate (Debug purpose)")
    /// (example: true)
    #[arg(long)]
    only_transcript: bool,

    /// Original subtitle SRT file path
    /// (default: "")
    /// (example: "origin.srt")
    /// (long_about: "Original subtitle path to save the transcripted subtitle as SRT")
    #[arg(long, default_value = "")]
    original_subtitle_path: String,

    /// Only translate the subtitle
    /// (default: false)
    /// (long_about: "Only translate the subtitle but do not export (Debug purpose)")
    #[arg(long)]
    only_translate: bool,

    /// Subtitle backend
    /// (default: "srt")
    /// (possible values: "srt", "container", "embedded")
    /// (example: "srt")
    /// (long_about: "Subtitle backend to use")
    #[arg(short, long, default_value = "srt")]
    subtitle_backend: String,

    /// Subtitle output path (if srt)
    /// (default: "output.srt")
    /// (example: "output.srt")
    /// (long_about: "Subtitle output path (if srt)")
    #[arg(long, default_value = "output.srt")]
    subtitle_output_path: String,

    /// Translator backend
    /// (default: "deepl")
    /// (possible values: "deepl", "google", "llm", "whisper")
    /// (example: "google")
    /// (long_about: "Translator backend to use")
    #[arg(short, long, default_value = "deepl")]
    translator_backend: String,

    /// Model name (if llm)
    /// (default: "gpt-4o")
    /// (example: "gpt-4o")
    /// (long_about: "Model name (if using llm for translation)")
    #[arg(long, default_value = "gpt-4o")]
    llm_model_name: String,

    /// API base (if llm)
    /// (default: "https://api.openai.com")
    /// (example: "https://api.openai.com")
    /// (long_about: "API base used in `genai` crate (if using llm for translation)")
    #[arg(long, default_value = "https://api.openai.com")]
    llm_api_base: String,

    /// Prompt (if llm)
    /// (default: "")
    /// (example: "Translate the following text to English")
    /// (long_about: "Prompt (if using llm for translation)")
    #[arg(long, default_value = "")]
    llm_prompt: String,
}

fn main() {
    let args = Args::parse();
    let input_video_path = args.input_video_path.as_str();
    let source_language = args.source_language;
    let target_language = args.target_language;

    println!("Hello, AI no jimaku gumi!");

    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().join("audio.wav");
    let tmp_path_str = tmp_path.as_os_str().to_str().unwrap();

    if args.only_extract_audio {
        utils::ffmpeg_audio::extract_audio_from_video(&input_video_path, tmp_path_str, 16000);

        // Generate a random name for the audio file based on timestamp
        let tmp_path = {
            let timestamp = chrono::Utc::now().timestamp();
            format!("{}.audio.{}.wav", args.input_video_path, timestamp)
        };
        // Copy the audio to the output path
        std::fs::copy(tmp_path_str, tmp_path.as_str()).unwrap();
        println!("Done, audio extracted to {}", tmp_path);
        return;
    }

    // Get the original subtitles
    let mut subtitles = match args.subtitle_source.as_str() {
        "audio" => {
            utils::ffmpeg_audio::extract_audio_from_video(&input_video_path, tmp_path_str, 16000);
            let state: whisper_rs::WhisperState = if args.translator_backend == "whisper" {
                if target_language != "en" {
                    println!("Whisper only supports english translation");
                    return;
                }

                // Transribe and translate the audio into subtitle directly (english only)
                whisper::experiment::extract_and_translate_from_f32_16khz_wav_audio(
                    &args.ggml_model_path,
                    tmp_path_str,
                    &source_language,
                    true,
                )
            } else {
                // Transcribe the audio into subtitle, the translation will be done later
                whisper::experiment::extract_from_f32_16khz_wav_audio(
                    &args.ggml_model_path,
                    tmp_path_str,
                    &source_language,
                )
            };
            utils::whisper_state::create_subtitle_from_whisper_state(&state)
        }
        source => {
            println!("Unsupported subtitle source now, {}", source);
            return;
        }
    };
    if subtitles.is_empty() {
        println!("No subtitles found");
        return;
    }

    if args.only_transcript {
        // Save original subtitles
        let tmp_path = if args.original_subtitle_path.is_empty() {
            input_video_path.to_string() + ".srt"
        } else {
            args.original_subtitle_path
        };
        let file = std::fs::File::create(tmp_path.as_str()).unwrap();
        let mut exporter = output::srt::SrtSubtitleExporter::new(file);
        exporter.output_subtitles(&subtitles);

        // Save transcripted subtitles and return
        println!("Done, transcripted subtitles saved to {}", tmp_path);
        return;
    }

    // Translate the subtitles
    match args.translator_backend.as_str() {
        "whisper" => {
            // Already translated if audio source is used
            match args.subtitle_source.as_str() {
                "audio" => {
                    println!("Skipping - subtitles are already translated using whisper");
                }
                source => {
                    println!("Unsupported translator backend for the given input source now, {}", source);
                    return;
                }
            };
        }
        "deepl" => {
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
        }
        "llm" => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let model_name = args.llm_model_name.clone();
            let api_base = args.llm_api_base;
            let model_name_clone = model_name.clone();

            let client = if !api_base.is_empty() {
                // -- Build an auth_resolver and the AdapterConfig
                // link https://github.com/jeremychone/rust-genai/blob/main/examples/c06-target-resolver.rs
                let target_resolver = ServiceTargetResolver::from_resolver_fn(
                    move | _service_target: ServiceTarget| -> Result<ServiceTarget, genai::resolver::Error> {
                        let endpoint = Endpoint::from_owned(api_base.clone());
                        let auth = AuthData::from_env("CUSTOM_API_KEY");
                        let model = ModelIden::new(AdapterKind::OpenAI, model_name_clone);
                        Ok(ServiceTarget { endpoint, auth, model })
                    },
                );
                Client::builder()
                    .with_service_target_resolver(target_resolver)
                    .build()
            } else {
                Client::default()
            };
            let system_prompt = if !args.llm_prompt.is_empty() {
                args.llm_prompt.clone()
            } else {
                format!(
                    "Translate the following text. to language {}",
                    &target_language
                )
            };
            subtitles.iter_mut().for_each(|s| {
                s.text = rt
                    .block_on(translate::llm::translate_text(
                        &client,
                        &model_name,
                        &system_prompt,
                        vec![s.text.as_str()],
                    ))
                    .unwrap();
            });
        }
        // more translators can be added here
        translator => {
            println!("Unsupported translator backend now {}", translator);
            return;
        }
    }

    // Save the translated subtitles
    if args.subtitle_backend == "srt" || args.only_translate {
        let tmp_path = if args.original_subtitle_path.is_empty() {
            input_video_path.to_string() + ".srt"
        } else {
            args.subtitle_output_path
        };
        let file = std::fs::File::create(tmp_path.as_str()).unwrap();
        let mut exporter = output::srt::SrtSubtitleExporter::new(file);
        exporter.output_subtitles(&subtitles);

        if args.only_translate {
            // This might be confusing, but we return here to avoid any other post-processing
            println!("Done, translated subtitles saved to {}", tmp_path);
            return;
        }
    } else {
        println!("Unsupported subtitle backend now");
        return;
    }
}
