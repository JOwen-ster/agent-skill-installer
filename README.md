# Agent Skill Installer

* A desktop app for managing agent skills
* Paste any github link and all `SKILL.md`'s get found and added!

## [Download here!](https://github.com/JOwen-ster/agent-skill-installer/releases)

## Reasoning

* Many skills now have their own install scripts via npm, python, or shell script. I wanted to centralize this process.
* ChatGPT Desktop - Codex, has a `skill-installer` skill that it comes with but this just runs an agentic loop using whatever you prompted (usually a link to the skill). I don't want to waste my usage copying and installing files from GitHub, so this exists to have a deterministic streamlined process.

## Features

* Toggleable between `ChatGPT Codex` and `Claude Code`
  * Features are `seperated per agent/harness`
* `Search` published skills
* Paste `GitHub` repository links for smart `SKILL.md` search and install
  * Select which skills to install if many are present 
* `Drag and drop` and `file picker` support for:
  * `Folders`
  * Individual `SKILL.md` files
    * Automatic folder creation using the `SKILL.md`'s `title`
* 1 click button to access the skill installation location
* List for currently installed skills
  * Search bar included!
* List for skill search history
* List for skill installation history

## Tech Stack

* Tauri Framework
  * Rust based cross platform desktop apps
* Svelte
  * Compiled frontend JavaScript framework
* TypeScript
  * TypeSafe JavaScript
* SQLITE
  * Lightweight database

> v1.0.0