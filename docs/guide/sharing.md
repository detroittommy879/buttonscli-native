---
title: Sharing looks
description: Export themes, share shaders and bundles, and ship your favorites as built-ins.
---

# Sharing looks

## Themes are files

Every personal theme is a `.json` file in
`~/.buttonscli-native/profiles/<profile>/themes/`. Export one from the Edit
tab, send it to someone, and they import it. The file holds colors, fonts,
gradient, effects and the shader with its colors and sliders.

## Shaders and bundles

Saved shaders and theme bundles live in your profile settings. To share one,
put it in a theme: a theme carries its shader, and a theme file is easy to
send.

## Ship your favorites with the app (maintainers)

**Settings → Release picks** copies your chosen personal themes, saved
shaders and theme bundles into a source checkout, so the next build includes
them for everyone: themes become built-in themes, shaders show under
**◆ Built-in shaders**, and bundles appear as **◆** bundles.

It appears in debug builds, or in release builds when
`~/.buttonscli-native/feature-flags.json` contains `"release_picks": true` or
`"enable_all": true`. The file is only read from that folder — not from next
to the program.

1. Check the checkout path (it's found automatically when you run from one).
2. Tick themes, shaders and bundles. **Also pick the themes these bundles
   use** adds what the bundles need.
3. Click **Copy into built-ins**. Bundles are rewritten to point at the copied
   themes.
4. Rebuild, check the new built-ins, and commit `assets/`.

**Save list as JSON** only writes the list of names, if you'd rather hand it
to someone.
