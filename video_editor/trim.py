from pydub import AudioSegment
from pydub.silence import detect_nonsilent
from moviepy.editor import concatenate_videoclips


# Remove silences from both videos based on their audio tracks
def remove_silence(video, min_silence_len=1000, silence_thresh=-70):
    """
    Removes sections of silence from a video based on its audio track.

    Args:
        video (VideoFileClip): The input video.
        min_silence_len (int, optional): The minimum duration of a silent segment in milliseconds. Defaults to 1000.
        silence_thresh (int, optional): The silence threshold in decibels. Defaults to -50.

    Returns:
        VideoFileClip: The video with silent sections removed.
    """
    # Extract audio from video, 44100 for top quality.
    audio = video.audio.to_soundarray(fps=44100)
    audio_segment = AudioSegment(audio.tobytes(), frame_rate=44100, sample_width=2, channels=1)
    # Detect non-silent chunks
    non_silent_ranges = detect_nonsilent(audio_segment, min_silence_len=min_silence_len, silence_thresh=silence_thresh)
    print("Non silent segments", non_silent_ranges)
    # Create subclips of non-silent parts and concatenate
    for start, end in non_silent_ranges:
        print(start,end)
    clips = [video.subclip(start/10000, end/10000) for start, end in non_silent_ranges]
    return concatenate_videoclips(clips)
