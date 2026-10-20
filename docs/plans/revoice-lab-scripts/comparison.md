Columns
  voice match  cosine similarity of Chatterbox's speaker encoder (an independent model; it isn't
               one of the cloning models compared, except Chatterbox itself) between the candidate
               and 2 minutes of George's real speech from other parts of the recording.
               His own test minute against that sets the ceiling.
  clarity      mean CTC confidence of an English wav2vec2 model reading the known words; his
               accented original scores lower than native speech.
  pace         seconds of speech vs his 45.4 s of slots.
  timing       median / 90th-percentile word-onset error after the no-stretch word fit.
  hard stops   words that end abruptly before a pause (his original: 0).

| Candidate | Voice match ↑ | Clarity ↑ | Speech s (slots 45.4) | Timing median / p90 | Hard stops | Peak VRAM GB | RTF |
|---|---|---|---|---|---|---|---|
| original (your voice, accented) | 0.978 | 0.877 | 42.5 | - | 0/16 | - | - |
| voxcpm_clone | 0.967 | 0.922 | 40.6 | 40.0 / 254.0 ms | 0/32 | 6.49 | 3.63 |
| chatterbox_clone | 0.941 | 0.934 | 43.3 | 100.0 / 340.0 ms | 0/24 | 3.31 | 0.99 |
| qwen_clone_xvec | 0.967 | 0.956 | 39.5 | 70.0 / 294.0 ms | 0/33 | 4.36 | 5.22 |
| qwen_clone_icl | 0.960 | 0.923 | 39.4 | 40.0 / 240.0 ms | 1/37 | 4.64 | 4.84 |
| longcat | 0.966 | 0.938 | 44.9 | 140.0 / 774.0 ms | 1/29 | 8.05 | 0.63 |
