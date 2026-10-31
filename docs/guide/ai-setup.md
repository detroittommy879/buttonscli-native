---
title: Set up AI
description: Connect ChatGPT or an API key, and choose which model does which job.
---

# Set up AI

AI features are optional. To use them, add a **connection** (where requests
go) and choose **models** for each job.

## 1. Add a connection

Open **Settings → AI providers** and click **Add connection**.

### ChatGPT plan (no API key)

Choose **ChatGPT plan (sign in)** and click **Continue with ChatGPT**. Your
browser opens; sign in and allow ButtonsCLI to use your Plus or Pro plan.
Back in the app it says *Connected as …*. Requests then count against your
ChatGPT plan instead of an API bill.

<!-- IMAGE: chatgpt-signin.png — Providers with a connected ChatGPT plan. -->

### API key

Choose an OpenAI-compatible or Anthropic connection, enter the endpoint and
model, and paste your key. Keys are stored in your operating system's
credential store (Windows Credential Manager, macOS Keychain, Secret Service
on Linux), never in a settings file.

## 2. Choose models per job

**Settings → AI models** has one row per job:

| Job | Used by |
| --- | --- |
| **AI Help model** | The AI Help chat |
| **Theme generation model** | [AI themes](ai-themes.md) |
| **Shader generation model** | Writing [AI shaders](ai-shaders.md) and random ideas |
| **Shader judge model** | Checking and fixing shaders — pick a *different* model from the writer for a real second opinion |
| **Simulated terminal model** | The simulated terminal demo |

**Add fallback** adds a second choice that is used if the first one fails or
is busy.

## Privacy

- You choose every provider; nothing is sent anywhere until you click a
  Generate, Help or similar button.
- Terminal contents are only sent by AI Help, and only when you include them.
- Shader and theme requests send your description and the settings noted in
  each page, never your terminal text.
