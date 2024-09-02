import whisper
import ffmpeg
import tempfile
import os

# Load the Whisper model
model = whisper.load_model("medium")  # You can use "base", "small", "medium", or "large" depending on your needs

# Function to extract audio from video
def extract_audio_from_video(video_path):
    with tempfile.NamedTemporaryFile(suffix=".wav", delete=False) as audio_file:
        audio_path = audio_file.name

    (
        ffmpeg
        .input(video_path)
        .output(audio_path, format='wav', acodec='pcm_s16le', ac=1, ar='16k')
        .run(quiet=True)
    )
    return audio_path

# Function to transcribe the audio with time codes
def transcribe_with_whisper(model, audio_path):
    result = model.transcribe(audio_path, language="el", verbose=True)
    return result['segments']  # This returns the segments with time codes

# Load video and transcribe
video_path = 'edited/addedsound.mp4'  # Replace with the path to your video file
audio_path = extract_audio_from_video(video_path)

print("Transcribing audio...")
segments = transcribe_with_whisper(model, audio_path)

# Display the transcription with time codes
for segment in segments:
    print(f"[{segment['start']:0.2f} - {segment['end']:0.2f}] {segment['text']}")

# Cleanup
os.remove(audio_path)  # Delete the temporary audio file
