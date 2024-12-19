use ffmpeg::format::input;
use ffmpeg::media::Type;
use ffmpeg_next::{
    self as ffmpeg,
    codec::Parameters,
    ffi::{AVChannelLayout, AVSampleFormat},
    format, frame,
};

fn convert_to_f32_audio_sample(samples: Vec<u8>, format: format::Sample) -> f32 {
    match format {
        ffmpeg::format::Sample::U8(_) => {
            assert!(samples.len() >= 1);
            samples[0] as f32 / 255.0
        }
        ffmpeg::format::Sample::I16(_) => {
            assert!(samples.len() >= 2);
            i16::from_le_bytes([samples[0], samples[1]]) as f32 / 32768.0
        }
        ffmpeg::format::Sample::I32(_) => {
            assert!(samples.len() >= 4);
            i32::from_le_bytes([samples[0], samples[1], samples[2], samples[3]]) as f32
                / 2147483648.0
        }
        ffmpeg_next::format::Sample::I64(_) => {
            assert!(samples.len() >= 8);
            i64::from_le_bytes([
                samples[0], samples[1], samples[2], samples[3], samples[4], samples[5], samples[6],
                samples[7],
            ]) as f32
                / 9223372036854775808.0
        }
        ffmpeg::format::Sample::F32(_) => {
            assert!(samples.len() >= 4);
            f32::from_le_bytes([samples[0], samples[1], samples[2], samples[3]])
        }
        ffmpeg::format::Sample::F64(_) => {
            assert!(samples.len() >= 8);
            f64::from_le_bytes([
                samples[0], samples[1], samples[2], samples[3], samples[4], samples[5], samples[6],
                samples[7],
            ]) as f32
        }
        ffmpeg_next::format::Sample::None => {
            panic!("No sample format found");
        }
    }
}

fn retrieve_f32_audio_samples(decoded: &frame::Audio, plane: usize) -> Vec<f32> {
    // Get the number of samples in the decoded audio
    let num_samples = decoded.samples();
    let mut converted_samples = Vec::with_capacity(num_samples);
    let mut count = 0;
    let data_len = match decoded.format() {
        ffmpeg::format::Sample::U8(_) => 1,
        ffmpeg::format::Sample::I16(_) => 2,
        ffmpeg::format::Sample::I32(_) => 4,
        ffmpeg::format::Sample::I64(_) => 8,
        ffmpeg::format::Sample::F32(_) => 4,
        ffmpeg::format::Sample::F64(_) => 8,
        ffmpeg::format::Sample::None => 0,
    };
    for chunk in decoded.data(plane).chunks(data_len) {
        if count >= num_samples {
            // Finish if we have enough samples
            break;
        }

        // Convert the chunk to a f32 sample
        let sample = convert_to_f32_audio_sample(chunk.to_vec(), decoded.format());
        converted_samples.push(sample);
        count += 1;
    }
    converted_samples
}

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
pub fn extract_audio_from_video(video_path: &str, audio_path: &str) {
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
                let plane = 0;
                let converted_samples = retrieve_f32_audio_samples(&decoded, plane);
                samples.extend(converted_samples);
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
