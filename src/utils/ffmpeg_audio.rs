use ffmpeg::format::input;
use ffmpeg::media::Type;
use ffmpeg_next::{
    self as ffmpeg,
    codec::Parameters,
    ffi::{AVChannelLayout, AVSampleFormat},
    format, frame,
};

// Get sample rate from input parameters
fn get_sample_rate(params: &Parameters) -> u32 {
    unsafe {
        // Extract sample rate from input parameters
        (*params.as_ptr()).sample_rate as u32
    }
}

// Convert sample format from i32 in Parameters to AVSampleFormat
fn get_av_sample_format(params: &Parameters) -> AVSampleFormat {
    unsafe {
        // Extract format from input parameters
        match (*params.as_ptr()).format {
            0 => AVSampleFormat::AV_SAMPLE_FMT_NONE,
            1 => AVSampleFormat::AV_SAMPLE_FMT_U8,
            2 => AVSampleFormat::AV_SAMPLE_FMT_S16,
            3 => AVSampleFormat::AV_SAMPLE_FMT_S32,
            4 => AVSampleFormat::AV_SAMPLE_FMT_FLT,
            5 => AVSampleFormat::AV_SAMPLE_FMT_DBL,
            6 => AVSampleFormat::AV_SAMPLE_FMT_U8P,
            7 => AVSampleFormat::AV_SAMPLE_FMT_S16P,
            8 => AVSampleFormat::AV_SAMPLE_FMT_S32P,
            9 => AVSampleFormat::AV_SAMPLE_FMT_FLTP,
            10 => AVSampleFormat::AV_SAMPLE_FMT_DBLP,
            11 => AVSampleFormat::AV_SAMPLE_FMT_S64,
            12 => AVSampleFormat::AV_SAMPLE_FMT_S64P,
            13 => AVSampleFormat::AV_SAMPLE_FMT_NB,
            _ => AVSampleFormat::AV_SAMPLE_FMT_NONE,
        }
    }
}

fn get_av_sample_bitrate(params: &Parameters) -> i64 {
    unsafe {
        // Extract bitrate from input parameters
        (*params.as_ptr()).bit_rate as i64
    }
}

fn get_av_sample_channel_layout(params: &Parameters) -> AVChannelLayout {
    unsafe {
        // Extract channel layout from input parameters
        (*params.as_ptr()).ch_layout.clone()
    }
}

// Extract audio from video using ffmpeg-next
fn extract_audio_from_video(video_path: &str, audio_path: &str) {
    ffmpeg::init().unwrap();

    let mut ictx = input(video_path).unwrap();
    println!(
        "Number of streams: {}, number of chapters: {}",
        ictx.nb_streams(),
        ictx.nb_chapters()
    );
    println!("Duration: {}", ictx.duration());
    println!("Bit rate: {}", ictx.bit_rate());
    println!("Metadata: {:?}", ictx.metadata());
    let format = ictx.format();
    println!("Format: {} {}", format.name(), format.description());
    format
        .extensions()
        .iter()
        .for_each(|ext| println!("Format extension: {}", ext));
    format
        .mime_types()
        .iter()
        .for_each(|mime| println!("Format MIME type: {}", mime));
    let input = ictx
        .streams()
        .best(Type::Audio)
        .ok_or(ffmpeg::Error::StreamNotFound)
        .unwrap();
    println!("Input: {:?}", input.index());
    println!("Input codec: {}", input.parameters().id().name());
    let params = input.parameters();
    let sample_rate: u32 = get_sample_rate(&params);
    let format: AVSampleFormat = get_av_sample_format(&params);
    let bitrate: i64 = get_av_sample_bitrate(&params);
    let channel_layout = get_av_sample_channel_layout(&params);
    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
    let mut decoder = context_decoder.decoder().audio().unwrap();

    let mut samples: Vec<f32> = Vec::new();
    for (stream, packet) in ictx.packets() {
        if stream.index() == 1 {
            // let mut decoded = Video::empty();
            // decoder.send_packet(&packet).unwrap();
            let mut decoded = frame::Audio::empty();
            decoder.send_packet(&packet).unwrap();
            while decoder.receive_frame(&mut decoded).is_ok() {
                let format = decoded.format();
                unsafe {
                    println!(
                        "Decoded frame line size: {:?}",
                        (*decoded.as_ptr()).linesize[0]
                    );
                }

                println!("Decoded frame format: {:?}", decoded.format());
                println!("Decoded frame channels: {:?}", decoded.channels());
                println!("Decoded frame duration: {}", decoded.packet().duration);
                println!("Decoded frame position: {}", decoded.packet().position);
                println!("Decoded frame size: {}", decoded.packet().size);
                println!(
                    "Decoded frame channel layout: {:?}",
                    decoded.channel_layout()
                );
                let pts = decoded.pts().unwrap();
                println!("Decoded frame pts: {:?}", decoded.pts());
                println!("Decoded frame planes: {:?}", decoded.planes());
                println!("Decoded frame is packed: {}", decoded.is_packed());
                for i in 0..decoded.channels() as usize {
                    println!("\tDecoded frame data size: {:?}", decoded.data(i).len());
                }
                let decoded_samples = decoded.data(0);
                println!(
                    "Decoded {} samples {}",
                    decoded_samples.len(),
                    decoded.samples()
                );
                let decoded_samples = decoded.data(0);
                match decoded.format() {
                    ffmpeg::format::Sample::U8(_) => {
                        // let converted_samples = decoded_samples.iter().map(|&x| x as f32 / 128.0 - 1.0).collect::<Vec<_>>();
                        // samples.push(converted_samples);
                    }
                    ffmpeg::format::Sample::I16(_) => {
                        // let converted_samples = decoded_samples.iter().map(|&x| x as f32 / 32768.0).collect::<Vec<_>>();
                        // samples.push(converted_samples);
                        // samples.push(decoded_samples as Vec<i16>);
                    }
                    ffmpeg::format::Sample::I32(planar) => {
                        // let converted_samples = decoded_samples.iter().map(|&x| x as f32 / 2147483648.0).collect::<Vec<_>>();
                        // samples.push(converted_samples);
                    }
                    ffmpeg_next::format::Sample::I64(_) => {
                        // let converted_samples = decoded_samples.iter().map(|&x| x as f32 / 9223372036854775808.0).collect::<Vec<_>>();
                        // samples.push(converted_samples);
                    }
                    ffmpeg::format::Sample::F32(planar) => {
                        let mut converted_samples = Vec::with_capacity(decoded.samples());
                        let mut count = 0;
                        for chunk in decoded_samples.chunks(4) {
                            if count < decoded.samples() {
                                // We just pick the first channel for now
                                count += 1;
                            } else {
                                break;
                            }
                            let sample =
                                f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                            converted_samples.push(sample);
                        }
                        samples.extend(converted_samples.iter());
                    }
                    ffmpeg::format::Sample::F64(planar) => {
                        let mut converted_samples = Vec::with_capacity(decoded.samples());
                        let mut count = 0;
                        let mut remaining_channels = decoded.channels();
                        for chunk in decoded_samples.chunks(8) {
                            match planar {
                                format::sample::Type::Packed => {
                                    if count < decoded.samples() {
                                        if remaining_channels > 0 {
                                            remaining_channels -= 1;
                                            continue;
                                        }
                                        count += 1;
                                    } else {
                                        break;
                                    }
                                    let sample = f64::from_le_bytes([
                                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5],
                                        chunk[6], chunk[7],
                                    ]);
                                    converted_samples.push(sample as f32);
                                    // Reset remaining channels
                                    remaining_channels = decoded.channels() - 1;
                                }
                                format::sample::Type::Planar => {
                                    if count < decoded.samples() {
                                        count += 1;
                                    } else {
                                        break;
                                    }
                                    let sample = f64::from_le_bytes([
                                        chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5],
                                        chunk[6], chunk[7],
                                    ]);
                                    converted_samples.push(sample as f32);
                                }
                            }
                        }
                    }
                    ffmpeg_next::format::Sample::None => {
                        panic!("No sample format found");
                    }
                }
            }
        }
    }

    let mut writer = hound::WavWriter::create(
        audio_path,
        hound::WavSpec {
            channels: if channel_layout.nb_channels < 0 { 0 } else { 1 } as u16,
            sample_rate: sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();
    for sample in samples {
        writer.write_sample(sample).unwrap();
    }
}

#[test]
fn test_extract_audio_from_video() {
    extract_audio_from_video("movie.mp4", "audio.wav");
    assert!(std::path::Path::new("audio.wav").exists());
}
