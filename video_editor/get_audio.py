import subprocess
import json



def get_audio_tracks(video_path):
    # Use ffprobe to get information about audio tracks
    cmd = [
        'ffprobe',
        '-v', 'quiet',
        '-print_format', 'json',
        '-show_streams',
        '-select_streams', 'a',
        video_path
    ]
    result = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    info = json.loads(result.stdout)
    return info['streams']

def extract_audio(video_path, output_path, track_index=0):
    cmd = [
        'ffmpeg',
        '-i', video_path,
        '-map', f'0:a:{track_index}',
        '-acodec', 'pcm_s16le',
        '-ar', '44100',
        '-ac', '1',
        output_path
    ]
    subprocess.run(cmd, check=True)

def audio_track_inrange(audio_track_position, audio_tracks):
    if not (0 <= audio_track_position < len(audio_tracks)):
        raise IndexError