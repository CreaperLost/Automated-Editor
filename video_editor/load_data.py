from video_editor.get_audio import get_audio_tracks, extract_audio,audio_track_inrange


def load_video_and_check(video_details, i):
    video_path = video_details['path']
    if video_path != None:
        audio_pos = video_details['track']

        # Get audio track information
        audio = get_audio_tracks(video_path)

        # Check that the tracks specified are in-range.
        try: 
            audio_track_inrange(audio_pos, audio)
            extract_audio(video_path, f'tmp_audio/temp_audio{i}.wav', audio_pos)
            return True
        except IndexError:
            print(f"Audio {i} is not in bounds.")        
    return False
        