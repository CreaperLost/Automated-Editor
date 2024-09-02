# Export the processed videos separately, each with its specified audio track

def export_video(video, i):
    print(f"Processed video index {i}")
    video.write_videofile(f"edited/processed_video{i}.mp4",audio=True)
    video.close()