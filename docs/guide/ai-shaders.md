---
title: AI shaders
description: Describe a look and get several working shader versions to pick from, with their own colors and sliders.
---

# AI shaders

Describe a background in words, and ButtonsCLI has AI write several versions at
once. Each one is compiled and checked before you see it, and you click the one
you like. You need a [Shader model set up](ai-setup.md) first.

The **Create with AI** section is on **Settings → Themes → Shaders**.

<!-- IMAGE: create-with-ai.png — Create with AI with a prompt, four finished
     versions as moving thumbnails, the flowchart progress under each. -->

## Make one

1. Pick **Start**:
   - **New idea** (default) — write from scratch. Your current code and
     sliders are *not* sent, so the result isn't stuck on the old shader.
   - **Change current shader** — send the current code and change it as you
     describe ("make it slower", "add stars", "more purple").
2. Pick **Colors**:
   - **AI picks new ones** (default) — every version chooses its own palette
     to fit the idea.
   - **Keep this shader's** / **Keep the theme's** — build on the colors shown
     under Style.
3. Describe the look. Say what moves, the mood, and the colors, for example:
   *bouncing fireballs with gravity, on a dark red night sky*.
4. Set **versions** (1–6) and click **Generate**.

The line under the buttons — "AI is told to …" — shows exactly what the AI is
told about your settings.

## What happens while it works

Each version runs through the same steps, shown as a small flowchart:

**Write → Check → Judge → Ready**

1. **Write** — your Shader model writes the code, a name, and (if allowed)
   colors and sliders.
2. **Check** — the code is compiled on your machine. Nothing broken gets
   through.
3. **Judge** — a second model (your Shader judge model) reviews it: does it
   match the request, is it visible, does Strength work? It keeps it, improves
   it, or fixes a compile error.
4. **Ready** — you see a live moving thumbnail. If even the fix fails, the
   version shows as failed and the others carry on.

With several versions, each one gets a different direction (Balanced, Subtle,
Bold, Playful, Geometric, Organic) so you get real variety. **Stop** cancels;
finished versions stay.

## Use a result

Click a thumbnail. The shader, its colors (as **This shader's**) and its
sliders move into Style, and the pane updates. You can then change colors,
drag sliders, lower Strength, or **Save to shader list**.

## Random ideas

Out of ideas? Click **🎲 N random ideas** (N follows the versions slider). The
Shader model suggests that many different ideas in one go. It's told the
ideas must be drawable by a shader, to name the technique that makes each one
work, and to mix simple, elegant ideas with ambitious ones.

- The first idea goes into the prompt box.
- Click any idea in the list to use it instead.
- **Write one version per idea** writes every idea at once, one version each,
  from scratch.
- **Clear ideas** hides the list.

<!-- IMAGE: random-ideas.png — the ideas list with four numbered ideas and the
     "Write one version per idea (4)" button. -->

## Fix with AI

If code you edited by hand doesn't compile, the Code section shows the error
and a **Fix with AI** button. The fix goes straight into the code;
**Undo AI change** puts your version back.

## Tips for good prompts

- **Name the motion**: drifting, pulsing, orbiting, falling, rippling.
- **Name the mood and colors**: *calm teal and gold*, *angry neon*.
- **Keep text in mind**: busy, bright patterns fight with text. Ask for
  "dark edges" or "calm center" if you read a lot in that pane.
- **Iterate**: generate with **New idea**, pick the closest one, then switch to
  **Change current shader** for "slower", "fewer stars", "bigger shapes".
- Things shaders **can't** do: show photos, text or 3D models, or remember
  earlier frames. They draw everything with math, every frame.

## What is sent to the AI

Your description, the settings notes ("AI is told to …"), the colors, and —
only with **Change current shader** — the current code and sliders. Terminal
contents are never sent.
