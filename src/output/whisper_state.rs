use whisper_rs::WhisperState;

use crate::output::Subtitle;

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

#[test]
fn test_create_subtitle_from_whipser_state() {
    use whisper_rs::WhisperContext;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContextParameters};

    let samples: Vec<f32> = hound::WavReader::open("audio.wav")
        .unwrap()
        .into_samples::<f32>()
        .map(|x| x.unwrap())
        .collect();

    // load a context and model
    let ctx = WhisperContext::new_with_params("ggml-tiny.bin", WhisperContextParameters::default())
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

    state
        .full(params, &samples[..])
        .expect("failed to run model");

    let subtitles = create_subtitle_from_whisper_state(&state);
    assert_ne!(subtitles.len(), 0);
}
