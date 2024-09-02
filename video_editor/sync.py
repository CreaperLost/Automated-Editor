from moviepy.editor import VideoFileClip
from scipy.signal import correlate
from scipy.io import wavfile
import numpy as np


verbose = False


def sync_videos(video1_path, video2_path):
    sync_point1, sync_point2 = find_sync_point('tmp_audio/temp_audio1.wav', 'tmp_audio/temp_audio2.wav')

    print(f"Sync point is : {sync_point1}, {sync_point2}")
    video1 = VideoFileClip(video1_path,verbose=verbose)
    video2 = VideoFileClip(video2_path,verbose=verbose)

    #if sync_point > 0:
    video1 = video1.subclip(sync_point1)
    #else:
    video2 = video2.subclip(sync_point2)

    return video1, video2


def find_closest_to_percent_max(arr, percent=0.7, tp = 0):
    # Calculate the target value (70% of max)
    if tp == 0:
        target = np.max(arr) * percent
    else:
        target = np.min(arr) * percent
    
    # Find the value closest to the target
    closest_value = arr[np.abs(arr - target).argmin()]
    
    # Find the first occurrence of this value
    first_occurrence = np.where(arr == closest_value)[0][0]
    
    return closest_value, first_occurrence

def find_sync_point(audio1_path, audio2_path):
    # Read audio files
    rate1, data1 = wavfile.read(audio1_path)
    rate2, data2 = wavfile.read(audio2_path)
    
    # Ensure same sample rate
    assert rate1 == rate2, "Sample rates must match"

    # Compute cross-correlation
    correlation = correlate(data1, data2, mode='full', method='fft')
    cl_val, pos = find_closest_to_percent_max(correlation, tp=0)
    print(cl_val, pos)
    # Find the peak in the correlation
    sync_point1 = np.argmax(correlation) - len(data1) + 1
    print(sync_point1)
    # Reverse the logic.
    correlation2 = correlate(data1, data2, mode='full', method='fft')
    cl_val, pos = find_closest_to_percent_max(correlation2, tp=1)
    print(cl_val, pos)
    sync_point2 = np.argmin(correlation2) - len(data2) + 1
    print(sync_point2)

    return abs(sync_point1 / rate1), abs(sync_point2/rate2)   # Convert samples to seconds

