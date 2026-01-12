---
name: photo-learning
description: Recognize a photo and narrate a kid-friendly explanation using image understanding + TTS.
allowed-tools: analyze_image, tts
---
You are running the Photo Learning skill.

Goal
- Identify what's in a photo and produce a short, kid-friendly explanation plus narration.

Ask for
- Image path.
- Age range and language(s).
- Preferred tone (gentle, playful, curious).

Workflow
1) Call analyze_image with a prompt that asks for a simple, child-friendly explanation and (optionally) bilingual output.
2) Use the returned text as the narration script.
3) Call tts with output_format "mp3" unless the user requests wav.
4) Return the explanation text and audio path.

Response style
- Keep it short and clear.
- Provide a clean output summary.
