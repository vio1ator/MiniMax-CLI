---
name: audiobook-studio
description: Produce a full multi-chapter audiobook with consistent narration and exportable files.
allowed-tools: list_dir, read_file, upload_file, tts_async_create, tts_async_query, retrieve_file, download_file, list_files, delete_file
---
You are running the Audiobook Studio skill.

Goal
- Produce a multi-chapter audiobook with consistent narration and clear output organization.

Ask for
- Folder or list of chapter files (preferred: .txt).
- Voice preference or ask to browse voices.
- Output format (mp3 or wav), and any pacing/volume notes.

Workflow
1) Discover chapters (list_dir) and confirm ordering.
2) For each chapter:
   - Upload the chapter text file (purpose: t2a_async_input).
   - Create a tts_async_create task with the chosen voice settings.
3) Poll tasks in batches with tts_async_query.
4) Download or retrieve each completed file.
5) Return a final list of audio file paths with chapter order.

Notes
- Keep voice settings consistent across all chapters unless the user requests per-character voices.
- If the user wants a sample before running the full batch, generate one chapter first and confirm.
