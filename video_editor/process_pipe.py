# https://www.osxexperts.net/
# Use the above link to install FFMPEG and FFPROBE
from video_editor.sync import sync_videos
from video_editor.trim import remove_silence
from moviepy.editor import VideoFileClip, AudioFileClip
from video_editor.sync import verbose
from video_editor.clean_up import clean_up_workspace
from video_editor.exporter import export_video
from video_editor.load_data import load_video_and_check



def process_videos( video_1: dict = {"path": None, "track": None}, video_2: dict = {"path": None, "track": None} ):

    clean_up_workspace()

    loaded1 = load_video_and_check(video_1, 1)
    loaded2 = load_video_and_check(video_2, 2)

    if loaded1 and loaded2:
        # Sync using FFT.
        video1, video2 = sync_videos(video_1['path'], video_2['path'])
    # No syncing.
    elif loaded1:
        video1 = VideoFileClip(video_1['path'],verbose=verbose)
    elif loaded2:
        video2 = VideoFileClip(video_2['path'],verbose=verbose)
    else:
        print("Both inputs undefined.")
        return
    
    # Remove silences
    if loaded1: export_video(remove_silence(video1), 0)
    if loaded2: export_video(remove_silence(video2), 1)

def remove_silence_process(video: dict = {"path": None, "track": None}):
    clean_up_workspace()
    video = VideoFileClip(video['path'],verbose=verbose)
    export_video(remove_silence(video), 0)


    

#