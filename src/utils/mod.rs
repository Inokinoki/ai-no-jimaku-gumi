pub mod ffmpeg_audio;
pub mod ffmpeg_subtitle_track;
pub mod whisper_state;

pub struct Subtitle {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

impl Subtitle {
    pub fn new(start: f32, end: f32, text: String) -> Subtitle {
        Subtitle { start, end, text }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;
    use std::sync::Mutex;

    static DOWNLOAD_LOCK: Mutex<()> = Mutex::new(());

    // The same sample files are shared by tests in several modules which run
    // in parallel, serialize their creation so that no test ever reads a
    // partially written file
    pub(crate) fn ensure_sample_file(path: &str, url: &str) {
        let _guard = DOWNLOAD_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if Path::new(path).exists() {
            return;
        }

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let response = reqwest::get(url).await.unwrap();
            let bytes = response.bytes().await.unwrap();
            std::fs::write(path, bytes).unwrap();
        });
    }
}
