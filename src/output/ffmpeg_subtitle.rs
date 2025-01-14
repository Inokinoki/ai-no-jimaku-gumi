use crate::output::OutputSubtitles;
use crate::output::Subtitle;
use ffmpeg_next::{
    self as ffmpeg, codec, encoder,
    ffi::{AVCodecID, AVMediaType},
    format, media, Rational,
};
use tempfile::TempDir;

use super::srt::SrtSubtitleExporter;

pub(crate) struct VideoSubtitleTrackExporter {
    in_video_path: String,
    out_video_path: String,
}

impl VideoSubtitleTrackExporter {
    pub fn new(in_video_path: String, out_video_path: String) -> VideoSubtitleTrackExporter {
        VideoSubtitleTrackExporter {
            in_video_path,
            out_video_path,
        }
    }
}

impl OutputSubtitles for VideoSubtitleTrackExporter {
    fn output_subtitles(&mut self, subtitles: &[Subtitle]) {
        // Write subtitles to a temp SRT file
        let tmp_dir = TempDir::new().unwrap();
        let tmp_path = tmp_dir.path().join("output.srt");
        let tmp_path_str = tmp_path.as_os_str().to_str().unwrap();

        let file = std::fs::File::create(tmp_path_str).unwrap();
        let mut exporter = SrtSubtitleExporter::new(file);
        exporter.output_subtitles(&subtitles);

        // Export subtitles to the video
        export_subtitle_to_video(
            self.in_video_path.as_str(),
            self.out_video_path.as_str(),
            tmp_path_str,
        );
    }
}

fn export_subtitle_to_video(in_video_path: &str, out_video_path: &str, subtitle_path: &str) {
    ffmpeg::init().unwrap();

    let in_place = in_video_path == out_video_path;
    let output_file = if in_place {
        format!("{}.tmp", in_video_path)
    } else {
        out_video_path.to_string()
    };

    let mut ictx = format::input(&in_video_path).unwrap();
    let mut octx = format::output(&output_file).unwrap();

    let mut stream_mapping = vec![0; ictx.nb_streams() as _];
    let mut ist_time_bases = vec![Rational(0, 1); ictx.nb_streams() as _];
    let mut ost_index = 0;
    let _stream_count = ictx.nb_streams();
    for (ist_index, ist) in ictx.streams().enumerate() {
        let ist_medium = ist.parameters().medium();
        println!(
            "ist_index: {}, ist_type: {:?}",
            ist_index,
            ist.parameters().medium()
        );
        if ist_medium != media::Type::Audio
            && ist_medium != media::Type::Video
            && ist_medium != media::Type::Subtitle
        {
            // Skip non-audio, non-video, non-subtitle streams
            // Note: we might lose Data/Attachment streams here
            stream_mapping[ist_index] = -1;
            continue;
        }
        stream_mapping[ist_index] = ost_index;
        ist_time_bases[ist_index] = ist.time_base();
        ost_index += 1;
        let mut ost = octx.add_stream(encoder::find(codec::Id::None)).unwrap();
        ost.set_parameters(ist.parameters());
        // We need to set codec_tag to 0 lest we run into incompatible codec tag
        // issues when muxing into a different container format. Unfortunately
        // there's no high level API to do this (yet).
        unsafe {
            (*ost.parameters().as_mut_ptr()).codec_tag = 0;
        }
    }

    // Add subtitle track
    let mut subtitle_ictx = format::input(&subtitle_path).unwrap();
    let subtitle_stream = subtitle_ictx.streams().best(media::Type::Subtitle).unwrap();
    let mut subtitle_ost = octx.add_stream(encoder::find(codec::Id::MOV_TEXT)).unwrap();
    let subtitle_stream_parameters = subtitle_stream.parameters().clone();
    subtitle_ost.set_parameters(subtitle_stream_parameters);
    unsafe {
        // TODO: Support more subtitle codecs with container formats that support them
        (*subtitle_ost.parameters().as_mut_ptr()).codec_type = AVMediaType::AVMEDIA_TYPE_SUBTITLE;
        (*subtitle_ost.parameters().as_mut_ptr()).codec_id = AVCodecID::AV_CODEC_ID_MOV_TEXT;
        (*subtitle_ost.parameters().as_mut_ptr()).codec_tag = 0;
    }
    println!(
        "subtitle_ost: {:?} {:?}",
        subtitle_ost.parameters().medium(),
        subtitle_ost.parameters().id()
    );
    octx.set_metadata(ictx.metadata().to_owned());
    println!("metadata: {:?}", ictx.metadata());
    octx.write_header().unwrap();

    for (stream, mut packet) in ictx.packets() {
        let ist_index = stream.index();
        let ost_index = stream_mapping[ist_index];
        if ost_index < 0 {
            continue;
        }
        let ost = octx.stream(ost_index as _).unwrap();
        packet.rescale_ts(ist_time_bases[ist_index], ost.time_base());
        packet.set_position(-1);
        packet.set_stream(ost_index as _);
        packet.write_interleaved(&mut octx).unwrap();
    }

    for (_stream, mut packet) in subtitle_ictx.packets() {
        packet.set_stream((octx.nb_streams() - 1) as usize);
        packet.write_interleaved(&mut octx).unwrap();
    }

    octx.write_trailer().unwrap();

    if in_place {
        std::fs::rename(&output_file, out_video_path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    // TODO: Add tests
}
