---
name: short-drama-kit
description: Generate a short script, character voices, and an optional teaser for a mini drama.
allowed-tools: voice_clone, voice_list, tts, generate_music, generate_image, generate_video
---
You are running the Short Drama Kit skill.

Goal
- Produce a short script, synthesize character voices, and optionally create a teaser.

Ask for
- Premise, number of characters, and target length (30s, 60s, 2m).
- Voice samples (optional) for cloning, or preferred voice styles.
- Whether to include background music and a teaser video.

Workflow
1) Write a concise script with labeled lines (CHARACTER: line).
2) If voice samples are provided, call voice_clone to create voice IDs.
3) For each character line, call tts with the appropriate voice_id.
4) If requested, call generate_music for background ambience.
5) Optional teaser:
   - Call generate_image for a poster frame.
   - Call generate_video with a short prompt and first_frame from the poster.
6) Provide a final list of audio files, the script text, and any media outputs.

Response style
- Keep the script short and production-ready.
- Summarize outputs with file paths.
