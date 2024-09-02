import os
import argparse
from moviepy.editor import VideoFileClip, concatenate_videoclips

def merge_videos(directory):
    # Get all video files in the directory
    video_files = [f for f in os.listdir(directory) if f.endswith(('.mp4', '.avi', '.mov', '.mkv'))]
    
    # Sort the video files by name
    video_files.sort()
    
    # Create VideoFileClip objects for each video
    clips = [VideoFileClip(os.path.join(directory, video)) for video in video_files]
    
    # Concatenate all clips
    final_clip = concatenate_videoclips(clips)
    
    return final_clip

def main():
    parser = argparse.ArgumentParser(description="Merge videos in a directory sequentially.")
    parser.add_argument("directory", help="Path to the directory containing videos")
    parser.add_argument("--output", default="merged_video.mp4", help="Output file name (default: merged_video.mp4)")
    args = parser.parse_args()

    # Merge videos
    final_clip = merge_videos(args.directory)

    # Export the final video
    print(f"Exporting merged video to {args.output}")
    final_clip.write_videofile(args.output,audio=True,preset='veryslow')

    # Close the clips to free up system resources
    final_clip.close()

if __name__ == "__main__":
    main()