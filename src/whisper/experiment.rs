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

#[test]
fn test_extract_from_f32_16khz_wav_audio() {
    extract_from_f32_16khz_wav_audio("ggml-tiny.bin", "audio.wav", "en");
}
