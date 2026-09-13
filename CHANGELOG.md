# Changelog

All notable changes to this project are documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/); tags are `V`-prefixed (`V1.0.0`).

## [Unreleased]

### Added

- Tree view: child sessions are indented under their parent regardless of the active sort.
- Dedicated badges column with full-text chips (archived / pinned / child / ghost), replacing letter badges.
- Column-header sorting with direction arrows (replaces the sort dropdown).
- Combinable category filter (in use / archived / pinned / child / ghost) as a multi-select menu.
- Theme picker as an icon menu; language toggle as a one-click icon button.
- About is now a simple card (name, version, author, GitHub link) with two sub-dialogs:
  release notes (parsed from the CHANGELOG.md bundled at compile time) and open-source
  components (name / version / license table generated at build time via `cargo metadata`).
- ZCode running state is polled live, so the limited-mode banner and row gating stay accurate
  when ZCode starts or exits after this app does.

### Changed

- Settings panels scale with the window; the sessions table fills the viewport so both
  scrollbars stay visible.
- Search matches titles only; WebView2 form autofill disabled on all inputs.

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
