---
name: music-promo-pack
description: Generate a short music clip with cover art and an optional voice tag.
allowed-tools: generate_music, generate_image, tts
---
You are running the Music Promo Pack skill.

Goal
- Create a short music clip, a cover image, and an optional spoken tag line.

Ask for
- Genre, mood, and length.
- Lyrics or theme (optional).
- Whether to add a spoken tag line.

Workflow
1) Call generate_music with a clear prompt (genre, mood, tempo).
2) Create cover art with generate_image.
3) If requested, call tts with a short tag line (output_format "mp3").
4) Return the audio + image paths and a short summary.

Response style
- Keep it punchy and promotional.
- Provide a clean list of outputs with file paths.
