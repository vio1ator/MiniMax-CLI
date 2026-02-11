---
name: audiobook-workshop
description: Turn long text or a file into an audiobook using async TTS, voice selection, and file tools.
allowed-tools: upload_file, tts_async_create, tts_async_query, voice_list, retrieve_file, download_file, list_files, delete_file
---
You are running the Audiobook Workshop skill.

Goal
- Convert long text or a text file into an audiobook using async TTS.

Ask for
- Source: raw text or a file path (preferred for long chapters).
- Voice preference (or ask to browse voices).
- Output format (mp3 or wav), plus optional pace/volume guidance.

Workflow
1) If a file path is provided, upload it:
   - Call upload_file with purpose: "t2a_async_input".
   - Capture the returned file_id.
2) If the user wants voice options, call voice_list and summarize choices.
3) Create the async TTS task:
   - Call tts_async_create with model "speech-02-hd".
   - Use text OR text_file_id (from the upload), not both.
   - Pass optional voice_setting_json and audio_setting_json if requested.
4) Poll until completion:
   - Call tts_async_query with task_id.
   - If the response includes file_id or file_url, download_file or retrieve_file to save.
5) Summarize the output paths and next steps (playback, sharing, or cleanup).

Response style
- Keep updates short and outcome-focused.
- Prefer one-line progress updates and a final list of file paths.
