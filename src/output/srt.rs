use crate::output::OutputSubtitles;
use crate::output::Subtitle;
use std::fs::File;
use std::io::Write;

pub(crate) struct SrtSubTitleExporter {
    pub file: File,
}

impl SrtSubTitleExporter {
    pub fn new(file: File) -> SrtSubTitleExporter {
        SrtSubTitleExporter { file }
    }
}

impl OutputSubtitles for SrtSubTitleExporter {
    fn output_subtitles(&mut self, subtitles: Vec<Subtitle>) {
        let mut srt = String::new();
        for (i, subtitle) in subtitles.iter().enumerate() {
            srt.push_str(&(i + 1).to_string());
            srt.push_str("\n");
            srt.push_str(&format!("{:.3} --> {:.3}", subtitle.start, subtitle.end));
            srt.push_str("\n");
            srt.push_str(subtitle.text.trim());
            srt.push_str("\n\n");
        }
        self.file.write_all(srt.as_bytes()).unwrap();
    }
}

#[test]
fn test_output_subtitles() {
    use std::io::Read;
    use tempfile::TempDir;

    let tmp_dir = TempDir::new().unwrap();
    let tmp_path = tmp_dir.path().join("test.srt");
    let file = File::create(tmp_path).unwrap();
    let mut exporter = SrtSubTitleExporter::new(file);
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
    exporter.output_subtitles(subtitles);

    let tmp_path = tmp_dir.path().join("test.srt");
    let mut file = File::open(tmp_path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    assert_eq!(
        contents,
        "1\n0.000 --> 1.000\nHello, world!\n\n2\n1.000 --> 2.000\nGoodbye, world!\n\n"
    );
}
