from video_editor.process_pipe import process_videos
import argparse
import json

def main():
    parser = argparse.ArgumentParser(description="Process two videos with specified paths and tracks.")
    
    parser.add_argument("--video1", type=json.loads, required=True, help="JSON string for video 1 (e.g., '{\"path\": \"path/to/video1.mp4\", \"track\": 0}')")
    parser.add_argument("--video2", type=json.loads, required=True, help="JSON string for video 2 (e.g., '{\"path\": \"path/to/video2.mp4\", \"track\": 0}')")
    
    args = parser.parse_args()
    
    process_videos(video_1=args.video1, video_2=args.video2)

if __name__ == "__main__":
    main()
    # python video_processor.py --video1 '{"path": "path/to/video1.mp4", "track": 0}' --video2 '{"path": "path/to/video2.mp4", "track": 0}'
