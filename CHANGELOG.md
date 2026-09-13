# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/); tags are `V`-prefixed (`V1.0.0`).

## [1.0.0] - 2026-09-13

First complete release.

### Added

- Session scanning: auto-locates the ZCode data directory (`~/.zcode`, overridable in Settings),
  lists all sessions with title, project, message count, disk usage and archive/pin/child/ghost badges.
- Session detail viewer: read-only message stream (user/assistant, reasoning, tool calls and outputs).
- Deletion: single or multi-select, two-step confirmation with a full cascade preview
  (children, index ghosts, per-table row counts, disk size, backup location).
- Safety: timestamped simple-copy backup of both databases and all session files before every
  deletion; `PRAGMA integrity_check` afterwards with automatic restore from the fresh backup on
  failure; schema compatibility check that blocks all operations on unexpected database formats.
- Limited mode: while ZCode is running, only archived sessions idle beyond a configurable
  threshold (default 1 hour; 1 minute – 365 days, minutes/hours/days units) and unreferenced by
  enabled automations can be deleted.
- About (version, release notes, compliance, author) and Help (usage, badge glossary, restore
  guide) dialogs; localized window title; light / dark / high-contrast themes; Chinese & English UI.
