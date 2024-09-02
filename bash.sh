# python sync-silence.py --video1 '{"path": "unedited/clip1.mp4", "track": 0}' --video2 '{"path": "unedited/clip1.mp4", "track": 0}'
# python merge_videos.py unedited/ --output edited/merged_output.mp4
# python add_sound.py edited/merged_output.mp4 tmp_audio/temp_audio1.wav edited/addedsound.mp4 --volume 0.8
python download_youtube.py https://youtube.com/shorts/-_Lb7w5FQsc -r "highest"
# python silence.py unedited/clip3.mp4