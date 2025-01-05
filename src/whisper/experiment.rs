/*
wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
*/

use hound;
use whisper_rs::{
    FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState,
};

pub fn extract_from_f32_16khz_wav_audio(
    model_path: &str,
    wav_path: &str,
    language: &str,
) -> WhisperState {
    let samples: Vec<f32> = hound::WavReader::open(wav_path)
        .unwrap()
        .into_samples::<f32>()
        .map(|x| x.unwrap())
        .collect();

    // load a context and model
    let ctx = WhisperContext::new_with_params(&model_path, WhisperContextParameters::default())
        .expect("failed to load model");

    let mut state = ctx.create_state().expect("failed to create state");

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // and set the language to translate to to english
    params.set_language(Some(&language));

    // we also explicitly disable anything that prints to stdout
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // we must convert to 16KHz mono f32 samples for the model
    // if needed (smile by Inoki)
    // some utilities exist for this
    // note that you don't need to use these, you can do it yourself or any other way you want
    // these are just provided for convenience
    // SIMD variants of these functions are also available, but only on nightly Rust: see the docs
    // let mut inter_samples = vec![Default::default(); samples.len()];

    // whisper_rs::convert_integer_to_float_audio(&samples, &mut inter_samples)
    //     .expect("failed to convert audio data");
    // let samples = whisper_rs::convert_stereo_to_mono_audio(&inter_samples)
    //     .expect("failed to convert audio data");

    // now we can run the model
    // note the key we use here is the one we created above
    state
        .full(params, &samples[..])
        .expect("failed to run model");

    state
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    fn setup() -> (String, String) {
        let data_dir = Path::new("data").join("whisper");
        let audio_path = data_dir
            .join("audio.wav")
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();
        let processed_audio_path = data_dir
            .join("processed_audio.wav")
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
        if !Path::new(processed_audio_path.as_str()).exists() {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let response = reqwest::get(
                    "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav",
                )
                .await
                .unwrap();
                let bytes = response.bytes().await.unwrap();
                std::fs::write(audio_path.as_str(), bytes).unwrap();

                let samples: Vec<i16> = hound::WavReader::open(audio_path)
                    .unwrap()
                    .into_samples::<i16>()
                    .map(|x| x.unwrap())
                    .collect();

                let mut inter_samples = vec![Default::default(); samples.len()];

                whisper_rs::convert_integer_to_float_audio(&samples, &mut inter_samples)
                    .expect("failed to convert audio data");
                let samples = whisper_rs::convert_stereo_to_mono_audio(&inter_samples)
                    .expect("failed to convert audio data");

                let spec = hound::WavSpec {
                    channels: 1,
                    sample_rate: 16000,
                    bits_per_sample: 32,
                    sample_format: hound::SampleFormat::Float,
                };
                let mut writer =
                    hound::WavWriter::create(processed_audio_path.as_str(), spec).unwrap();
                for sample in samples {
                    writer.write_sample(sample).unwrap();
                }
                writer.finalize().unwrap();
            });
        }

        (processed_audio_path, model_path)
    }

    #[test]
    fn test_extract_from_f32_16khz_wav_audio() {
        let (audio_path, model_path) = setup();

        extract_from_f32_16khz_wav_audio(&model_path, &audio_path, "en");
    }
}
