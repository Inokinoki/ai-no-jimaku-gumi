use anyhow::Context;
use ffmpeg_next::codec;
use ffmpeg_next::codec::subtitle::Rect;
use ffmpeg_next::format::input;
use ffmpeg_next::media::Type;

use super::Subtitle;

// Extract the subtitles from the first subtitle track of the video container.
// Only text based tracks are supported, bitmap (e.g. PGS) rectangles are skipped.
pub fn extract_subtitles_from_video(video_path: &str) -> anyhow::Result<Vec<Subtitle>> {
    ffmpeg_next::init().context("failed to initialize ffmpeg")?;

    let mut ictx =
        input(video_path).with_context(|| format!("failed to open video {}", video_path))?;
    let stream = ictx
        .streams()
        .best(Type::Subtitle)
        .context("no subtitle track found in the video")?;
    let time_base = stream.time_base();
    let stream_index = stream.index();
    println!(
        "Input subtitle track: {}, codec: {}",
        stream_index,
        stream.parameters().id().name()
    );

    let context_decoder =
        codec::Context::from_parameters(stream.parameters()).with_context(|| {
            format!(
                "failed to create decoder for subtitle track {}",
                stream_index
            )
        })?;
    let mut decoder = context_decoder
        .decoder()
        .subtitle()
        .context("failed to open subtitle decoder")?;

    let mut subtitles = Vec::new();
    for (stream, packet) in ictx.packets() {
        if stream.index() != stream_index {
            continue;
        }

        let mut decoded = ffmpeg_next::Subtitle::new();
        if !decoder
            .decode(&packet, &mut decoded)
            .context("failed to decode subtitle packet")?
        {
            continue;
        }

        // The display times are in milliseconds, relative to the packet timestamp.
        // Some decoders (e.g. SubRip) leave the end display time at 0, fall back
        // to the packet duration in that case.
        let packet_seconds = packet
            .pts()
            .map(|pts| pts as f64 * time_base.numerator() as f64 / time_base.denominator() as f64)
            .unwrap_or_default();
        let packet_duration_seconds = if packet.duration() > 0 {
            packet.duration() as f64 * time_base.numerator() as f64 / time_base.denominator() as f64
        } else {
            0.0
        };
        let start = packet_seconds + decoded.start() as f64 / 1000.0;
        let end = start + (decoded.end() - decoded.start()) as f64 / 1000.0;
        let end = if end > start {
            end
        } else {
            start + packet_duration_seconds
        };

        for rect in decoded.rects() {
            let text = match rect {
                Rect::Text(ref text) => text.get().to_string(),
                Rect::Ass(ref ass) => ass_field_text(ass.get()).to_string(),
                // Bitmap rectangles (e.g. PGS) are not supported yet
                _ => continue,
            };
            let text = clean_ass_text(text.as_str());
            if text.is_empty() {
                continue;
            }
            subtitles.push(Subtitle::new(start as f32, end as f32, text));
        }
    }

    Ok(subtitles)
}

// Rectangles produced by text based decoders (SubRip, MOV text, ...) contain an
// internal ASS line without timestamps, "0,0,Default,,0,0,0,,text" (9 fields),
// while lines passed through from an ASS track keep the file format,
// "Dialogue: 0,0:00:01.18,0:00:02.00,Default,,0,0,0,,text" (10 fields, where the
// second field holds a timestamp). In both formats the text is the last field.
fn ass_field_text(ass: &str) -> &str {
    let rest = match ass.strip_prefix("Dialogue:") {
        Some(rest) => rest.trim_start(),
        None => ass,
    };
    let field_count = if rest
        .split(',')
        .nth(1)
        .is_some_and(|field| field.contains(':'))
    {
        10
    } else {
        9
    };
    rest.splitn(field_count, ',')
        .nth(field_count - 1)
        .map(str::trim_start)
        .unwrap_or(rest)
}

// Strip the override tags kept by text based decoders, like {\i1} or {\pos(1,2)},
// and turn ASS line breaks (\N) into plain newlines.
fn clean_ass_text(text: &str) -> String {
    let mut cleaned = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' => {
                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                }
            }
            '\\' => match chars.peek() {
                Some('N') | Some('n') => {
                    chars.next();
                    cleaned.push('\n');
                }
                _ => cleaned.push(c),
            },
            _ => cleaned.push(c),
        }
    }
    cleaned.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    fn setup_input_video() -> String {
        let data_dir = Path::new("data").join("utils");
        let input_video_path = data_dir
            .join("audio.mp4")
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

        // Download a sample audio to use as video (serialized across tests)
        super::super::test_support::ensure_sample_file(
            input_video_path.as_str(),
            "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav",
        );

        input_video_path
    }

    // Mux an audio stream and a SubRip subtitle stream into a Matroska file
    fn mux_video_with_subtitles(video_path: &str, subtitle_path: &str, output_path: &str) {
        ffmpeg_next::init().unwrap();

        let mut ictx = input(video_path).unwrap();
        let mut octx = ffmpeg_next::format::output(&output_path).unwrap();

        let audio_stream_index = ictx.streams().best(Type::Audio).unwrap().index();
        let mut audio_ost = octx
            .add_stream(ffmpeg_next::encoder::find(ffmpeg_next::codec::Id::None))
            .unwrap();
        audio_ost.set_parameters(ictx.stream(audio_stream_index).unwrap().parameters());
        unsafe {
            (*audio_ost.parameters().as_mut_ptr()).codec_tag = 0;
        }

        let mut subtitle_ictx = input(subtitle_path).unwrap();
        let subtitle_stream = subtitle_ictx.streams().best(Type::Subtitle).unwrap();
        let mut subtitle_ost = octx
            .add_stream(ffmpeg_next::encoder::find(ffmpeg_next::codec::Id::None))
            .unwrap();
        subtitle_ost.set_parameters(subtitle_stream.parameters());
        unsafe {
            (*subtitle_ost.parameters().as_mut_ptr()).codec_tag = 0;
        }

        octx.write_header().unwrap();

        for (stream, mut packet) in ictx.packets() {
            if stream.index() == audio_stream_index {
                let ost = octx.stream(0).unwrap();
                packet.rescale_ts(stream.time_base(), ost.time_base());
                packet.set_position(-1);
                packet.set_stream(0);
                packet.write_interleaved(&mut octx).unwrap();
            }
        }
        for (stream, mut packet) in subtitle_ictx.packets() {
            let ost = octx.stream(1).unwrap();
            packet.rescale_ts(stream.time_base(), ost.time_base());
            packet.set_position(-1);
            packet.set_stream(1);
            packet.write_interleaved(&mut octx).unwrap();
        }

        octx.write_trailer().unwrap();
    }

    #[test]
    fn test_extract_subtitles_from_video() {
        let input_video_path = setup_input_video();
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let subtitle_path = tmp_dir.path().join("subs.srt");
        std::fs::write(
            &subtitle_path,
            "1\n00:00:00,500 --> 00:00:02,000\nHello subtitle\n\n2\n00:00:03,000 --> 00:00:04,500\nSecond line\n",
        )
        .unwrap();
        let video_path = tmp_dir.path().join("video_with_subs.mkv");
        mux_video_with_subtitles(
            input_video_path.as_str(),
            subtitle_path.to_str().unwrap(),
            video_path.to_str().unwrap(),
        );

        let subtitles = extract_subtitles_from_video(video_path.to_str().unwrap()).unwrap();
        assert_eq!(subtitles.len(), 2);
        assert_eq!(subtitles[0].text, "Hello subtitle");
        assert!((subtitles[0].start - 0.5).abs() < 0.05);
        assert!((subtitles[0].end - 2.0).abs() < 0.05);
        assert_eq!(subtitles[1].text, "Second line");
        assert!((subtitles[1].start - 3.0).abs() < 0.05);
        assert!((subtitles[1].end - 4.5).abs() < 0.05);
    }

    #[test]
    fn test_extract_subtitles_without_track() {
        let input_video_path = setup_input_video();
        // A raw audio file has no subtitle track
        let result = extract_subtitles_from_video(input_video_path.as_str());
        assert!(result.is_err());
    }

    #[test]
    fn test_ass_field_text() {
        // Internal ASS line as produced by text based decoders (9 fields, no prefix)
        let internal = "0,0,Default,,0,0,0,,Hello subtitle";
        assert_eq!(ass_field_text(internal), "Hello subtitle");
        // Line passed through from an ASS track (10 fields with timestamps)
        let file = "Dialogue: 0,0:00:01.18,0:00:03.00,Default,,0,0,0,,Hello, with comma";
        assert_eq!(ass_field_text(file), "Hello, with comma");
        assert_eq!(ass_field_text("Plain text"), "Plain text");
    }

    #[test]
    fn test_clean_ass_text() {
        assert_eq!(clean_ass_text("{\\i1}Hello{\\i0}"), "Hello");
        assert_eq!(clean_ass_text("First\\NSecond"), "First\nSecond");
        assert_eq!(clean_ass_text("No tags"), "No tags");
        // A single backslash which is not a line break is kept as is
        assert_eq!(clean_ass_text("Back\\\\slash"), "Back\\\\slash");
    }
}
