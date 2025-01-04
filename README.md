# AI no jimaku gumi (AIの字幕組)

AI no jimaku gumi is a cli utility to facilitate the translation of video.

## Installation

To get started with AI no jimaku gumi, follow these steps:

1. Clone the repository:
    ```bash
    git clone https://github.com/inokinoki/ainojimakugumi.git
    ```
2. Navigate to the project directory:
    ```bash
    cd ainojimakugumi
    ```
3. Build with cargo:
    ```bash
    cargo build
    ```
4. Download whisper model:
    ```bash
    wget https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
    ```
5. Run it with your video path after `--input-video-path` and target language after `-t`.

## Usage

To use AI no jimaku gumi, refer this help:

```
aI NO jimaKu gumI, a sub-title maker using AI.

Usage: ainojimakugumi [OPTIONS] --input-video-path <INPUT_VIDEO_PATH>

Options:
  -i, --input-video-path <INPUT_VIDEO_PATH>
          Name of the person to greet
  -s, --source-language <SOURCE_LANGUAGE>
          Which language to translate from (default: "ja") (possible values: "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh") (example: "ja") [default: ja]
  -t, --target-language <TARGET_LANGUAGE>
          Which language to translate to (default: "en") (possible values: "en", "es", "fr", "de", "it", "ja", "ko", "pt", "ru", "zh") (example: "en") [default: en]
      --start-time <START_TIME>
          Video start time [default: 0] (Not usable yet)
      --end-time <END_TIME>
          Video end time [default: 0] (Not usable yet)
      --subtitle-backend <SUBTITLE_BACKEND>
          Subtitle backend (default: "srt") (possible values: "srt", "container", "embedded") (example: "srt") (long_about: "Subtitle backend to use") [default: srt]
      --subtitle-output-path <SUBTITLE_OUTPUT_PATH>
          Subtitle output path (if srt) (default: "output.srt") (example: "output.srt") (long_about: "Subtitle output path (if srt)") [default: output.srt]
      --translator-backend <TRANSLATOR_BACKEND>
          Translator backend (default: "deepl") (possible values: "deepl", "google", "openai") (example: "google") (long_about: "Translator backend to use") [default: deepl]
      --subtitle-source <SUBTITLE_SOURCE>
          Subtitle source (default: "audio") (possible values: "audio", "container", "ocr") (example: "audio") (long_about: "Subtitle source to use") [default: audio]
  -m, --model-name <MODEL_NAME>
          [default: gpt-4o]
  -a, --api-base <API_BASE>
          [default: ]
  -p, --prompt <PROMPT>
          [default: ]
  -h, --help
          Print help
  -V, --version
          Print version
```

We are currently supporting only `deepl, llm` translation and `srt` export.

Please provide your own DeepL API key in `DEEPL_API_KEY` env, and `DEEPL_API_URL=https://api.deepl.com` if you are using the paid API version.

if you are using llm translate please refer the repo [rust-genai](https://github.com/jeremychone/rust-genai) for more detail

e.g.
```cli
export CUSTOM_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxx
./target/debug/ainojimakugumi --input-video-path one.webm --api-base https://sssss.com/v1/ --translator-backend llm --prompt 'translate this to English' --model-name 'gpt-4o-mini'
```
