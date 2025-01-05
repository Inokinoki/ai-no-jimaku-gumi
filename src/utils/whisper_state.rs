use whisper_rs::WhisperState;

use super::Subtitle;

pub fn create_subtitle_from_whisper_state(state: &WhisperState) -> Vec<Subtitle> {
    let num_segments = state
        .full_n_segments()
        .expect("failed to get number of segments");

    let mut subtitles = Vec::new();
    for i in 0..num_segments {
        let segment = state
            .full_get_segment_text(i)
            .expect("failed to get segment");
        let start_timestamp = state
            .full_get_segment_t0(i)
            .expect("failed to get segment start timestamp");
        let end_timestamp = state
            .full_get_segment_t1(i)
            .expect("failed to get segment end timestamp");

        subtitles.push(Subtitle::new(
            start_timestamp as f32 / 100.,
            end_timestamp as f32 / 100.,
            segment,
        ));
    }

    subtitles
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn setup() -> (String, String) {
        let data_dir = Path::new("data").join("utils");
        let audio_path = data_dir
            .join("audio_whisper.wav")
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();
        let model_path = data_dir
            .join("ggml-tiny.bin")
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

        // Use reqwest to download a sample audio and model
        if !Path::new(model_path.as_str()).exists() {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let response = reqwest::get(
                    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
                );
                let bytes = response.await.unwrap().bytes().await.unwrap();
                std::fs::write(model_path.as_str(), bytes).unwrap();
            });
        }
        if !Path::new(audio_path.as_str()).exists() {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let response = reqwest::get(
                    "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav",
                )
                .await
                .unwrap();
                let bytes = response.bytes().await.unwrap();
                std::fs::write(audio_path.as_str(), bytes).unwrap();
            });
        }

        (audio_path, model_path)
    }

    #[test]
    fn test_create_subtitle_from_whipser_state() {
        use whisper_rs::WhisperContext;
        use whisper_rs::{FullParams, SamplingStrategy, WhisperContextParameters};

        let (audio_path, model_path) = setup();

        let samples: Vec<i16> = hound::WavReader::open(audio_path)
            .unwrap()
            .into_samples::<i16>()
            .map(|x| x.unwrap())
            .collect();

        // load a context and model
        let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
            .expect("failed to load model");

        let mut state = ctx.create_state().expect("failed to create state");

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // and set the language to translate to to english
        params.set_language(Some("en"));

        // we also explicitly disable anything that prints to stdout
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // we must convert to 16KHz mono f32 samples for the model
        // some utilities exist for this
        // note that you don't need to use these, you can do it yourself or any other way you want
        // these are just provided for convenience
        // SIMD variants of these functions are also available, but only on nightly Rust: see the docs
        let mut inter_samples = vec![Default::default(); samples.len()];

        whisper_rs::convert_integer_to_float_audio(&samples, &mut inter_samples)
            .expect("failed to convert audio data");
        let samples = whisper_rs::convert_stereo_to_mono_audio(&inter_samples)
            .expect("failed to convert audio data");

        state
            .full(params, &samples[..])
            .expect("failed to run model");

        let subtitles = create_subtitle_from_whisper_state(&state);
        assert_ne!(subtitles.len(), 0);
    }
}
