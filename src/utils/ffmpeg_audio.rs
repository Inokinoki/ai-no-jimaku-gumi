use ffmpeg::format::input;
use ffmpeg::media::Type;
use ffmpeg_next::{self as ffmpeg, frame};

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
    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();
    let mut decoder = context_decoder.decoder().audio().unwrap();

    let mut samples: Vec<i16> = Vec::new();
    for (stream, packet) in ictx.packets() {
        if stream.index() == 1 {
            // let mut decoded = Video::empty();
            // decoder.send_packet(&packet).unwrap();
            let mut decoded = frame::Audio::empty();
            decoder.send_packet(&packet).unwrap();
            while decoder.receive_frame(&mut decoded).is_ok() {
                println!("Decoded frame format: {:?}", decoded.format());
                println!("Decoded frame channels: {:?}", decoded.channels());
                println!(
                    "Decoded frame channel layout: {:?}",
                    decoded.channel_layout()
                );
                println!("Decoded frame pts: {:?}", decoded.pts());
                println!("Decoded frame planes: {:?}", decoded.planes());
                println!("Decoded frame is packed: {}", decoded.is_packed());
                let decoded_samples = decoded.data(0);
                println!("Decoded {} samples", decoded_samples.len());
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
                        // let converted_samples = decoded_samples.iter().map(|&x| x as f32).collect::<Vec<_>>();
                        // samples.push(converted_samples);
                        // Pack decoded_samples into f32 samples
                        // let mut converted_samples = Vec::new();
                        // for i in 0..decoded_samples.len() {
                        //     if i % 4 == 0 {
                        //         converted_samples.push((
                        //             decoded_samples[i] << 8 * 3
                        //             | decoded_samples[i + 1] << 8 * 2
                        //             | decoded_samples[i + 2] << 8
                        //             | decoded_samples[i + 3]
                        //         ) as f32);
                        //     }
                        // }
                        // // Convert the f32 samples to i16, where decoded samples are u8
                        // let converted_samples = converted_samples.iter().map(|&x| (x * 32768.0) as i16).collect::<Vec<_>>();
                        // samples.push(converted_samples);
                    }
                    ffmpeg::format::Sample::F64(planar) => {
                        // let converted_samples = decoded_samples.iter().map(|&x| x as f32).collect::<Vec<_>>();
                        // samples.push(converted_samples);
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
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
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
