use anyhow::Context;
use ffmpeg_next::codec;
use ffmpeg_next::encoder;
use ffmpeg_next::filter;
use ffmpeg_next::format;
use ffmpeg_next::frame;
use ffmpeg_next::media;
use ffmpeg_next::Rational;
use tempfile::TempDir;

use crate::output::OutputSubtitles;
use crate::output::Subtitle;

use super::srt::SrtSubtitleExporter;

pub(crate) struct HardSubtitleVideoExporter {
    in_video_path: String,
    out_video_path: String,
}

impl HardSubtitleVideoExporter {
    pub fn new(in_video_path: String, out_video_path: String) -> HardSubtitleVideoExporter {
        HardSubtitleVideoExporter {
            in_video_path,
            out_video_path,
        }
    }
}

impl OutputSubtitles for HardSubtitleVideoExporter {
    fn output_subtitles(&mut self, subtitles: &[Subtitle]) {
        // Write subtitles to a temp SRT file
        let tmp_dir = TempDir::new().unwrap();
        let tmp_path = tmp_dir.path().join("output.srt");
        let tmp_path_str = tmp_path.as_os_str().to_str().unwrap();

        let file = std::fs::File::create(tmp_path_str).unwrap();
        let mut exporter = SrtSubtitleExporter::new(file);
        exporter.output_subtitles(&subtitles);

        // Export subtitles to the video
        burn_subtitles_into_video(
            self.in_video_path.as_str(),
            tmp_path_str,
            self.out_video_path.as_str(),
        )
        .unwrap();
    }
}

// Escape a filename for use inside a filter graph argument: the argument
// parser splits on ':' and terminates on '\'', so Windows drive letters need
// escaping (and backslashes become forward slashes, which work everywhere)
fn escape_filter_arg(path: &str) -> String {
    path.replace('\\', "/")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

fn rescale_pts(pts: i64, from: Rational, to: Rational) -> i64 {
    unsafe {
        ffmpeg_next::ffi::av_rescale_q(
            pts,
            ffmpeg_next::ffi::AVRational {
                num: from.numerator(),
                den: from.denominator(),
            },
            ffmpeg_next::ffi::AVRational {
                num: to.numerator(),
                den: to.denominator(),
            },
        )
    }
}

// Pull the filtered frames out of the graph sink and write the encoded
// packets to the output
#[allow(clippy::too_many_arguments)]
fn drain_filtered_frames(
    sink: &mut filter::context::Sink,
    encoder: &mut encoder::Encoder,
    octx: &mut format::context::Output,
    output_stream_index: usize,
    filter_time_base: Rational,
    encoder_time_base: Rational,
    stream_time_base: Rational,
    packet: &mut ffmpeg_next::packet::Packet,
) -> anyhow::Result<()> {
    let mut filtered = frame::Video::empty();
    while sink.frame(&mut filtered).is_ok() {
        if let Some(pts) = filtered.pts() {
            filtered.set_pts(Some(rescale_pts(pts, filter_time_base, encoder_time_base)));
        }
        encoder.send_frame(&filtered)?;

        while encoder.receive_packet(packet).is_ok() {
            packet.set_position(-1);
            packet.set_stream(output_stream_index);
            packet.rescale_ts(encoder_time_base, stream_time_base);
            packet.write_interleaved(octx)?;
        }
    }
    Ok(())
}

// Burn the subtitles of the SRT file into the video of in_video_path by
// decoding it, rendering the subtitles with the ffmpeg subtitles filter and
// re-encoding the video (audio tracks are passed through untouched)
fn burn_subtitles_into_video(
    in_video_path: &str,
    subtitle_path: &str,
    out_video_path: &str,
) -> anyhow::Result<()> {
    ffmpeg_next::init().context("failed to initialize ffmpeg")?;

    let in_place = in_video_path == out_video_path;
    let output_file = if in_place {
        super::temp_sibling(in_video_path)
    } else {
        out_video_path.to_string()
    };

    let mut ictx = format::input(in_video_path)
        .with_context(|| format!("failed to open video {}", in_video_path))?;
    let input_video_stream = ictx
        .streams()
        .best(media::Type::Video)
        .context("no video stream found in the video")?;
    let input_video_index = input_video_stream.index();
    let input_video_time_base = input_video_stream.time_base();
    // The mp4/mkv muxers accept the stream frame rate, fall back to 25 fps
    // when the container does not declare one
    let frame_rate = {
        let rate = input_video_stream.rate();
        if rate.numerator() > 0 {
            rate
        } else {
            Rational(25, 1)
        }
    };

    let mut video_decoder = codec::Context::from_parameters(input_video_stream.parameters())
        .context("failed to create the video decoder context")?
        .decoder()
        .video()
        .context("failed to open the video decoder")?;

    let mut octx = format::output(&output_file)
        .with_context(|| format!("failed to create output video {}", output_file))?;

    // Video output stream: re-encode as H.264
    let h264 = encoder::find(codec::Id::H264).context("no H.264 encoder found")?;
    let (output_video_stream_index, output_video_time_base) = {
        let mut output_video_stream = octx.add_stream(h264)?;
        output_video_stream.set_time_base(Rational(1, frame_rate.numerator().max(1)));
        (output_video_stream.index(), output_video_stream.time_base())
    };
    let mut video_encoder_builder = codec::Context::new_with_codec(h264)
        .encoder()
        .video()
        .context("failed to open the video encoder")?;
    video_encoder_builder.set_width(video_decoder.width());
    video_encoder_builder.set_height(video_decoder.height());
    video_encoder_builder.set_format(ffmpeg_next::format::Pixel::YUV420P);
    video_encoder_builder.set_time_base(output_video_time_base);
    video_encoder_builder.set_frame_rate(Some(frame_rate));
    video_encoder_builder.set_aspect_ratio(video_decoder.aspect_ratio());
    video_encoder_builder.set_gop(250);
    // Matroska/mp4 want the stream headers in the container, not in-band
    video_encoder_builder.set_flags(ffmpeg_next::codec::flag::Flags::GLOBAL_HEADER);
    let mut video_encoder = video_encoder_builder
        .open_with(ffmpeg_next::Dictionary::from_iter([
            ("preset", "medium"),
            ("crf", "23"),
        ]))
        .context("failed to open the H.264 encoder")?;
    {
        let mut output_video_stream = octx
            .stream_mut(output_video_stream_index)
            .context("the output video stream disappeared")?;
        output_video_stream.set_parameters(&video_encoder);
        unsafe {
            let parameters = output_video_stream.parameters().as_mut_ptr();
            // Do not force the codec tag of the input video on the new stream
            (*parameters).codec_tag = 0;
            (*parameters).codec_id = ffmpeg_next::ffi::AVCodecID::AV_CODEC_ID_H264;
        }
    }

    // All audio streams are passed through without re-encoding
    let mut audio_stream_mapping = std::collections::HashMap::new();
    let mut audio_time_bases = std::collections::HashMap::new();
    for stream in ictx.streams() {
        if stream.parameters().medium() != media::Type::Audio {
            continue;
        }
        let mut ost = octx
            .add_stream(encoder::find(codec::Id::None).unwrap())
            .unwrap();
        ost.set_parameters(stream.parameters());
        unsafe {
            (*ost.parameters().as_mut_ptr()).codec_tag = 0;
        }
        audio_stream_mapping.insert(stream.index(), ost.index());
        audio_time_bases.insert(stream.index(), stream.time_base());
    }

    // Video filter graph: buffersrc -> subtitles -> format -> buffersink
    let mut graph = filter::Graph::new();
    let input_pixel_format: ffmpeg_next::ffi::AVPixelFormat = video_decoder.format().into();
    let buffer_args = format!(
        "video_size={}x{}:pix_fmt={}:time_base={}/{}:pixel_aspect={}/{}",
        video_decoder.width(),
        video_decoder.height(),
        input_pixel_format as i32,
        input_video_time_base.numerator(),
        input_video_time_base.denominator(),
        video_decoder.aspect_ratio().numerator(),
        video_decoder.aspect_ratio().denominator(),
    );
    graph
        .add(
            &filter::find("buffer").context("the buffer filter is not available")?,
            "in",
            &buffer_args,
        )
        .context("failed to create the buffer source")?;
    graph
        .add(
            &filter::find("buffersink").context("the buffersink filter is not available")?,
            "out",
            "",
        )
        .context("failed to create the buffer sink")?;
    let filter_spec = format!(
        "subtitles=filename='{}',format=yuv420p",
        escape_filter_arg(subtitle_path)
    );
    graph
        .output("in", 0)
        .context("failed to mark the filter graph source")?
        .input("out", 0)
        .context("failed to mark the filter graph sink")?
        .parse(&filter_spec)
        .with_context(|| format!("failed to build the subtitle filter graph: {}", filter_spec))?;
    graph
        .validate()
        .context("failed to validate the filter graph")?;
    let mut source_context = graph.get("in").context("the buffer source disappeared")?;
    let mut sink_context = graph.get("out").context("the buffer sink disappeared")?;
    let mut source = source_context.source();
    let mut sink = sink_context.sink();
    let filter_time_base = sink.time_base();

    octx.write_header().context("failed to write the header")?;

    // The muxer may adjust stream time bases in write_header, use the final
    // ones when scaling the packet timestamps below
    let output_stream_time_base = octx
        .stream(output_video_stream_index)
        .context("the output video stream disappeared")?
        .time_base();

    let mut packet = ffmpeg_next::packet::Packet::empty();
    let mut decoded = frame::Video::empty();
    for (stream, mut packet) in ictx.packets() {
        if stream.index() == input_video_index {
            video_decoder
                .send_packet(&packet)
                .context("failed to decode a video packet")?;
            while video_decoder.receive_frame(&mut decoded).is_ok() {
                source
                    .add(&decoded)
                    .context("failed to push a frame into the filter graph")?;
                drain_filtered_frames(
                    &mut sink,
                    &mut video_encoder,
                    &mut octx,
                    output_video_stream_index,
                    filter_time_base,
                    output_video_time_base,
                    output_stream_time_base,
                    &mut packet,
                )?;
            }
        } else if let Some(output_index) = audio_stream_mapping.get(&stream.index()).copied() {
            let ost = octx.stream(output_index).unwrap();
            packet.rescale_ts(stream.time_base(), ost.time_base());
            packet.set_position(-1);
            packet.set_stream(output_index);
            packet.write_interleaved(&mut octx)?;
        }
    }

    // Flush the decoder, the filter graph and the encoder
    video_decoder.send_eof()?;
    while video_decoder.receive_frame(&mut decoded).is_ok() {
        source.add(&decoded)?;
        drain_filtered_frames(
            &mut sink,
            &mut video_encoder,
            &mut octx,
            output_video_stream_index,
            filter_time_base,
            output_video_time_base,
            output_stream_time_base,
            &mut packet,
        )?;
    }
    source.flush()?;
    drain_filtered_frames(
        &mut sink,
        &mut video_encoder,
        &mut octx,
        output_video_stream_index,
        filter_time_base,
        output_video_time_base,
        output_stream_time_base,
        &mut packet,
    )?;
    video_encoder.send_eof()?;
    while video_encoder.receive_packet(&mut packet).is_ok() {
        packet.set_position(-1);
        packet.set_stream(output_video_stream_index);
        packet.rescale_ts(output_video_time_base, output_stream_time_base);
        packet.write_interleaved(&mut octx)?;
    }

    octx.write_trailer()
        .context("failed to write the trailer")?;

    // Close both files before replacing the input, Windows refuses to
    // rename over a file that is still open
    drop(octx);
    drop(ictx);
    drop(video_decoder);
    drop(graph);
    if in_place {
        std::fs::rename(&output_file, out_video_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Create a small black H.264 video so that the test does not need any
    // sample file, only the ffmpeg codecs
    fn write_test_video(video_path: &str, frames: i32) {
        ffmpeg_next::init().unwrap();

        let mut octx = format::output(&video_path).unwrap();
        let h264 = encoder::find(codec::Id::H264).unwrap();
        let mut builder = {
            let mut ost = octx.add_stream(h264).unwrap();
            ost.set_time_base(Rational(1, 25));
            codec::Context::new_with_codec(h264)
                .encoder()
                .video()
                .unwrap()
        };
        builder.set_width(320);
        builder.set_height(240);
        builder.set_format(ffmpeg_next::format::Pixel::YUV420P);
        builder.set_time_base(Rational(1, 25));
        builder.set_frame_rate(Some(Rational(25, 1)));
        builder.set_gop(25);
        builder.set_flags(ffmpeg_next::codec::flag::Flags::GLOBAL_HEADER);
        let mut encoder = builder.open().unwrap();
        octx.stream_mut(0).unwrap().set_parameters(&encoder);

        octx.write_header().unwrap();
        // The muxer may have adjusted the stream time base
        let stream_time_base = octx.stream(0).unwrap().time_base();

        let mut packet = ffmpeg_next::packet::Packet::empty();
        for i in 0..frames {
            let mut frame = frame::Video::new(ffmpeg_next::format::Pixel::YUV420P, 320, 240);
            // Fill with limited range black (Y=16, U=V=128)
            for plane in 0..3 {
                let value = if plane == 0 { 16 } else { 128 };
                let stride = frame.stride(plane);
                let lines = if plane == 0 { 240 } else { 120 };
                let bytes = if plane == 0 { 320 } else { 160 };
                for line in 0..lines {
                    for byte in &mut frame.data_mut(plane)[line * stride..line * stride + bytes] {
                        *byte = value;
                    }
                }
            }
            frame.set_pts(Some(i as i64));
            encoder.send_frame(&frame).unwrap();
            while encoder.receive_packet(&mut packet).is_ok() {
                packet.set_position(-1);
                packet.set_stream(0);
                packet.rescale_ts(Rational(1, 25), stream_time_base);
                packet.write_interleaved(&mut octx).unwrap();
            }
        }
        encoder.send_eof().unwrap();
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_position(-1);
            packet.set_stream(0);
            packet.rescale_ts(Rational(1, 25), stream_time_base);
            packet.write_interleaved(&mut octx).unwrap();
        }
        octx.write_trailer().unwrap();
    }

    #[test]
    fn test_burn_subtitles_into_video() {
        let tmp_dir = tempfile::TempDir::new().unwrap();
        let video_path = tmp_dir.path().join("video.mkv");
        write_test_video(video_path.to_str().unwrap(), 75);

        let subtitles = vec![
            Subtitle::new(0.5, 2.5, "Hello subtitle".to_string()),
            Subtitle::new(3.0, 4.0, "Second line".to_string()),
        ];
        let mut exporter = HardSubtitleVideoExporter::new(
            video_path.to_str().unwrap().to_string(),
            video_path.to_str().unwrap().to_string(),
        );
        exporter.output_subtitles(&subtitles);

        // Decode the output and check that some frames contain bright pixels,
        // which can only come from the burned white subtitle text
        ffmpeg_next::init().unwrap();
        let mut ictx = format::input(video_path.to_str().unwrap()).unwrap();
        assert!(ictx.streams().best(media::Type::Video).is_some());

        let (video_stream_index, video_stream_parameters) = {
            let video_stream = ictx.streams().best(media::Type::Video).unwrap();
            (video_stream.index(), video_stream.parameters())
        };
        let mut decoder = codec::Context::from_parameters(video_stream_parameters)
            .unwrap()
            .decoder()
            .video()
            .unwrap();
        let mut decoded = frame::Video::empty();
        let mut bright_pixels = 0;
        let mut frames = 0;
        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet).unwrap();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    frames += 1;
                    let stride = decoded.stride(0);
                    for line in 0..decoded.height() as usize {
                        for byte in &decoded.data(0)
                            [line * stride..line * stride + decoded.width() as usize]
                        {
                            if *byte > 200 {
                                bright_pixels += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(frames > 50, "unexpected frame count {}", frames);
        assert!(
            bright_pixels > 100,
            "no bright pixels found, subtitles were probably not burned in"
        );
    }
}
