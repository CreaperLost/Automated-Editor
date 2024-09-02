from pytube.innertube import _default_clients
from pytube import cipher
import os
from pytube import YouTube as pytubeYT
import argparse
import re
from moviepy.editor import VideoFileClip, AudioFileClip, concatenate_videoclips
from pytubefix import YouTube as fixtubeYT
from pytubefix.cli import on_progress


# Fixing PyTube.
def sanitize_filename(filename):
    # Remove any non-ASCII characters
    filename = filename.encode('ascii', 'ignore').decode('ascii')
    
    # Replace spaces with underscores
    filename = filename.replace(' ', '_')
    
    # Remove any characters that aren't alphanumeric, underscore, hyphen, or period
    filename = re.sub(r'[^\w\-.]', '', filename)
    
    # Remove any leading or trailing periods or spaces
    filename = filename.strip('. ')
    
    # Limit the length of the filename (optional, adjust as needed)
    max_length = 255  # Maximum filename length for most file systems
    if len(filename) > max_length:
        filename = filename[:max_length]
    
    # If the filename is empty after sanitization, provide a default
    if not filename:
        filename = "untitled"
    
    return filename

_default_clients[ "ANDROID"][ "context"]["client"]["clientVersion"] = "19.08.35"
_default_clients["IOS"]["context"]["client"]["clientVersion"] = "19.08.35"
_default_clients[ "ANDROID_EMBED"][ "context"][ "client"]["clientVersion"] = "19.08.35"
_default_clients[ "IOS_EMBED"][ "context"]["client"]["clientVersion"] = "19.08.35"
_default_clients["IOS_MUSIC"][ "context"]["client"]["clientVersion"] = "6.41"
_default_clients[ "ANDROID_MUSIC"] = _default_clients[ "ANDROID_CREATOR" ]

def get_throttling_function_name(js: str) -> str:
    """Extract the name of the function that computes the throttling parameter.

    :param str js:
        The contents of the base.js asset file.
    :rtype: str
    :returns:
        The name of the function used to compute the throttling parameter.
    """
    function_patterns = [
        r'a\.[a-zA-Z]\s*&&\s*\([a-z]\s*=\s*a\.get\("n"\)\)\s*&&\s*'
        r'\([a-z]\s*=\s*([a-zA-Z0-9$]+)(\[\d+\])?\([a-z]\)',
        r'\([a-z]\s*=\s*([a-zA-Z0-9$]+)(\[\d+\])\([a-z]\)',
    ]
    #logger.debug('Finding throttling function name')
    for pattern in function_patterns:
        regex = re.compile(pattern)
        function_match = regex.search(js)
        if function_match:
            #logger.debug("finished regex search, matched: %s", pattern)
            if len(function_match.groups()) == 1:
                return function_match.group(1)
            idx = function_match.group(2)
            if idx:
                idx = idx.strip("[]")
                array = re.search(
                    r'var {nfunc}\s*=\s*(\[.+?\]);'.format(
                        nfunc=re.escape(function_match.group(1))),
                    js
                )
                if array:
                    array = array.group(1).strip("[]").split(",")
                    array = [x.strip() for x in array]
                    return array[int(idx)]

    raise re.RegexMatchError(
        caller="get_throttling_function_name", pattern="multiple"
    )

cipher.get_throttling_function_name = get_throttling_function_name


def main():
    # Create the parser
    parser = argparse.ArgumentParser(description="Download YouTube videos.")
    
    # Add arguments
    parser.add_argument("url", help="The YouTube video URL to download")
    parser.add_argument("-o", "--output", default=".", help="The output directory for downloaded videos (default: current directory)")
    parser.add_argument("-r", "--resolution", choices=['highest', '720p', '480p', '360p','other'], default='highest', 
                        help="Desired resolution of the video (default: highest)")
    
    # Parse arguments
    args = parser.parse_args()
    
    # Create output directory if it doesn't exist
    if not os.path.exists(args.output):
        os.makedirs(args.output)
    
    # Download the video
    try:
        yt = pytubeYT(args.url)
        
        base_filename = sanitize_filename(yt.title)

        if args.resolution == '360p':
            video = yt.streams.get_highest_resolution()
        elif args.resolution == 'other':
            url = args.url
            yt = fixtubeYT(url, on_progress_callback = on_progress)            
            ys = yt.streams.get_highest_resolution()
            ys.download()
            return
        else:
            video = yt.streams.filter(adaptive=True, file_extension='mp4').first()
            audio = yt.streams.filter(only_audio=True).first()
        
        if video is None:
            print(f"No stream found with resolution {args.resolution}. Downloading highest available resolution.")
            video = yt.streams.get_highest_resolution()
        
        print(f"Downloading: {yt.title}")
        video.download(args.output,filename=f"{base_filename}.mp4")
        print("Downloading Audio")
        audio.download(args.output, filename=f"{base_filename}.mp3")
        print(f"Download completed: {yt.title}")
        video = VideoFileClip(f"{base_filename}.mp4")
        audio = AudioFileClip(f"{base_filename}.mp3")
        final_clip = video.set_audio(audio)
        output_file = os.path.join(args.output, f"{yt.title}_final.mp4")
        final_clip.write_videofile(output_file,audio=True,preset='veryslow',threads=4)
        
        # Close the clips
        video.close()
        audio.close()
        final_clip.close()
        
        # Remove temporary files
        os.remove(f"{base_filename}.mp4")
        os.remove(f"{base_filename}.mp3")
    except Exception as e:
        print(f"An error occurred: {str(e)}")

if __name__ == "__main__":
    main()