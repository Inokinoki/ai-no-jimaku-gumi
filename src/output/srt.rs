use crate::output::OutputSubtitles;
use crate::output::Subtitle;
use std::fs::File;
use std::io::Write;

pub(crate) struct SrtSubtitleExporter {
    pub file: File,
}

impl SrtSubtitleExporter {
    pub fn new(file: File) -> SrtSubtitleExporter {
        SrtSubtitleExporter { file }
    }
}

fn format_time(time: f32) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        (time as u32 / 3600),
        (time as u32 % 3600 / 60),
        ((time % 60.0).floor() as u32),
        ((time.fract() * 1000.0) as u32)
    )
}

impl OutputSubtitles for SrtSubtitleExporter {
    fn output_subtitles(&mut self, subtitles: &Vec<Subtitle>) {
        let mut srt = String::new();
        for (i, subtitle) in subtitles.iter().enumerate() {
            srt.push_str(&(i + 1).to_string());
            srt.push_str("\n");
            srt.push_str(&format!(
                "{} --> {}",
                format_time(subtitle.start),
                format_time(subtitle.end)
            ));
            srt.push_str("\n");
            srt.push_str(subtitle.text.trim());
            srt.push_str("\n\n");
        }
        self.file.write_all(srt.as_bytes()).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_subtitles() {
        use std::io::Read;
        use tempfile::TempDir;

        let tmp_dir = TempDir::new().unwrap();
        let tmp_path = tmp_dir.path().join("test.srt");
        let file = File::create(tmp_path).unwrap();
        let mut exporter = SrtSubtitleExporter::new(file);
        let subtitles = vec![
            Subtitle {
                start: 0.0,
                end: 1.0,
                text: "Hello, world!".to_string(),
            },
            Subtitle {
                start: 1.0,
                end: 2.0,
                text: "Goodbye, world!".to_string(),
            },
        ];
        exporter.output_subtitles(&subtitles);

        let tmp_path = tmp_dir.path().join("test.srt");
        let mut file = File::open(tmp_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        assert_eq!(
            contents,
            format!(
                "1\n{} --> {}\nHello, world!\n\n2\n{} --> {}\nGoodbye, world!\n\n",
                format_time(0.0),
                format_time(1.0),
                format_time(1.0),
                format_time(2.0)
            )
        );
    }
}
