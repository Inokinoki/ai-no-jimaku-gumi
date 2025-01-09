use ffmpeg::format::input;
use ffmpeg::format::sample::Type as SampleType;
use ffmpeg::media::Type;
use ffmpeg::software::resampling::context::Context as Resampler;
use ffmpeg_next::{
    self as ffmpeg, format,
    frame::{self, Audio},
    util,
};

fn convert_to_f32_audio_sample(samples: Vec<u8>, format: format::Sample) -> f32 {
    match format {
        ffmpeg::format::Sample::U8(_) => {
            assert!(!samples.is_empty());
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
    let data_len = match decoded.format() {
        ffmpeg::format::Sample::U8(_) => 1,
        ffmpeg::format::Sample::I16(_) => 2,
        ffmpeg::format::Sample::I32(_) => 4,
        ffmpeg::format::Sample::I64(_) => 8,
        ffmpeg::format::Sample::F32(_) => 4,
        ffmpeg::format::Sample::F64(_) => 8,
        ffmpeg::format::Sample::None => 0,
    };
    for (count, chunk) in decoded.data(plane).chunks(data_len).enumerate() {
        if count >= num_samples {
            // Finish if we have enough samples
            break;
        }

        // Convert the chunk to a f32 sample
        let sample = convert_to_f32_audio_sample(chunk.to_vec(), decoded.format());
        converted_samples.push(sample);
    }
    converted_samples
}

// Extract audio from video using ffmpeg-next
pub fn extract_audio_from_video(video_path: &str, audio_path: &str, output_sample_rate: u32) {
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
    let input = ictx
        .streams()
        .best(Type::Audio)
        .ok_or(ffmpeg::Error::StreamNotFound)
        .unwrap();
    println!("Input: {:?}", input.index());
    println!("Input codec: {}", input.parameters().id().name());
    let context_decoder =
        ffmpeg::codec::context::Context::from_parameters(input.parameters()).unwrap();

    // Prepare decoder
    let mut decoder = context_decoder.decoder().audio().unwrap();

    // Prepare wav writer
    let mut writer = hound::WavWriter::create(
        audio_path,
        hound::WavSpec {
            channels: 1,
            sample_rate: output_sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        },
    )
    .unwrap();

    for (stream, packet) in ictx.packets() {
        if stream.index() == 1 {
            // let mut decoded = Video::empty();
            // decoder.send_packet(&packet).unwrap();
            let mut decoded = frame::Audio::empty();
            decoder.send_packet(&packet).unwrap();
            while decoder.receive_frame(&mut decoded).is_ok() {
                // Create resampler
                let mut resampler = Resampler::get(
                    decoded.format(),
                    decoded.channel_layout(),
                    decoded.rate(),
                    format::Sample::F32(SampleType::Planar),
                    util::channel_layout::ChannelLayout::MONO,
                    output_sample_rate,
                )
                .unwrap();

                // Create output frame
                let mut output_frame = Audio::new(
                    resampler.output().format,
                    decoded.samples(),
                    resampler.output().channel_layout,
                );

                // Currently only support one plane
                let plane = 0;
                // Convert to the given sample rate
                resampler.run(&decoded, &mut output_frame).unwrap();
                let resampled_samples = retrieve_f32_audio_samples(&output_frame, plane);

                for sample in resampled_samples {
                    writer.write_sample(sample).unwrap();
                }
                writer.flush().unwrap();
            }
        }
    }
    // Close the writer
    writer.finalize().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    fn setup() -> (String, String) {
        let data_dir = Path::new("data").join("utils");
        let input_video_path = data_dir
            .join("audio.mp4")
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();
        let audio_path = data_dir
            .join("audio.wav")
            .as_os_str()
            .to_str()
            .unwrap()
            .to_string();

        // Use reqwest to download a sample audio as video
        if !Path::new(input_video_path.as_str()).exists() {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let response = reqwest::get(
                    "https://github.com/ggerganov/whisper.cpp/raw/master/samples/jfk.wav",
                )
                .await
                .unwrap();
                let bytes = response.bytes().await.unwrap();
                std::fs::write(input_video_path.as_str(), bytes).unwrap();
            });
        }

        (input_video_path, audio_path)
    }

    #[test]
    fn test_extract_audio_from_video() {
        let (video_path, audio_path) = setup();

        extract_audio_from_video(video_path.as_str(), audio_path.as_str(), 16000);
        assert!(std::path::Path::new(audio_path.as_str()).exists());
    }
}
