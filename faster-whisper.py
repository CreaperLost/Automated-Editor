from faster_whisper import WhisperModel
import ffmpeg
import tempfile

#model_size = "large-v3"
model_size = "distil-large-v3"


print("0")
# Run on GPU with FP16
model = WhisperModel(model_size, device="auto", compute_type="int8")

# or run on GPU with INT8
# model = WhisperModel(model_size, device="cuda", compute_type="int8_float16")
# or run on CPU with INT8
# model = WhisperModel(model_size, device="cpu", compute_type="int8")
# Load video and transcribe

print("2")
segments, info = model.transcribe('tmp_audio/temp_audio2.wav', beam_size=5,language='el')
print("3")
print("Detected language '%s' with probability %f" % (info.language, info.language_probability))

for segment in segments:
    print("[%.2fs -> %.2fs] %s" % (segment.start, segment.end, segment.text))