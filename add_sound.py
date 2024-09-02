import argparse
from moviepy.editor import VideoFileClip, AudioFileClip, CompositeAudioClip

def add_audio_to_video(video_path, audio_path, output_path, volume):
    # Load the video file
    video = VideoFileClip(video_path)
    
    # Load the audio file
    audio = AudioFileClip(audio_path)
    
    # Set the audio volume
    audio = audio.volumex(volume)
    
    # If the audio is longer than the video, cut it to match the video duration
    if audio.duration > video.duration:
        audio = audio.subclip(0, video.duration)

    
    # Combine the original audio (if any) with the new audio
    final_audio = CompositeAudioClip([video.audio, audio]) if video.audio else audio
    
    # Set the final audio to the video
    final_video = video.set_audio(final_audio)
    
    # Write the result to a file
    final_video.write_videofile(output_path)
    
    # Close the clips to free up system resources
    video.close()
    audio.close()
    final_video.close()

def main():
    parser = argparse.ArgumentParser(description="Add MP3 audio to a video file with volume control.")
    parser.add_argument("video", help="Path to the input video file")
    parser.add_argument("audio", help="Path to the MP3 file to add as audio")
    parser.add_argument("output", help="Path for the output video file")
    parser.add_argument("--volume", type=float, default=1.0, help="Volume level for the added audio (default: 1.0)")
    
    args = parser.parse_args()

    print(f"Adding audio '{args.audio}' to video '{args.video}' at volume {args.volume}")
    add_audio_to_video(args.video, args.audio, args.output, args.volume)
    print(f"Video with added audio saved as '{args.output}'")

if __name__ == "__main__":
    main()