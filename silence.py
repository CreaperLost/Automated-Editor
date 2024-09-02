import argparse
from video_editor.process_pipe import remove_silence_process


def main():
    parser = argparse.ArgumentParser(description="Merge videos in a directory sequentially.")
    parser.add_argument("videopath", help="Path to the directory containing videos")
    parser.add_argument("--output", default="edited/silenced_video.mp4", help="Output file name (default: merged_video.mp4)")
    args = parser.parse_args()

    # Merge videos
    final_clip = remove_silence_process({"path":args.videopath,"track":0})

    # Export the final video
    print(f"Exporting merged video to {args.output}")
    final_clip.write_videofile(args.output,audio=True,preset='veryslow')

    # Close the clips to free up system resources
    final_clip.close()

if __name__ == "__main__":
    main()