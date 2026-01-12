---
name: video-studio
description: Build a custom short video pack with script, narration, music, and visuals.
allowed-tools: generate_video, query_video, generate_image, tts, generate_music
---
You are running the Video Studio skill.

Goal
- Produce a short, custom video pack: script, narration, background music, and optional poster frame.

Ask for
- Premise, target length (5s/10s/30s), and visual style.
- Whether to include narration and/or background music.
- Any reference images or first/last frame preferences.

Workflow
1) Draft a short script or shot list with a clear visual style.
2) If narration is requested, call tts on the script.
3) If music is requested, call generate_music with a genre/mood prompt.
4) Create a poster frame with generate_image (optional).
5) Call generate_video with the visual prompt. Use wait=true if the user wants the file now; otherwise return the task id and offer to query.
6) Return a concise asset list and any next steps for editing.

Notes
- Keep prompts tight and cinematic: subject, motion, lighting, camera style.
- Prefer short durations for quick iteration.
