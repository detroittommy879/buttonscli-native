# Tool Calls Are Just Text: Make Up Your Own Format

*Why your app can skip the "official" JSON tool format, how to invent your own with a few XML-style tags, and when the official way is still worth it.*

Most guides about AI tools say the same thing: define a JSON schema, pass it in the `tools` parameter, read the `tool_calls` field that comes back. That works. But it hides something useful.

**A language model only reads text and writes text.** Tools, function calling, structured output and "agents" are all built on top of that. Once you see this, you can design your own formats, in plain language, and they work with almost any modern model, on any API.

## 1. What actually happens when you "call a tool"

When you send a request with a `tools` list, the provider does not hand the model a JSON object. It turns your messages *and* your tool definitions into one long text prompt, using that model's chat template. The model writes text back. The provider then looks for the part of that text that is a tool call, parses it, and gives it to you as a nice `tool_calls` JSON field.

You can see this in open-weight models, because their chat templates are public. Several popular ones (Qwen and Hermes-style templates, for example) literally wrap tool calls in tags:

*What the model writes, inside an open model's chat template:*

```text
<tool_call>
{"name": "get_weather", "arguments": {"city": "Detroit"}}
</tool_call>
```

That is a made-up tag with some JSON inside it. The "native" format is a convention the model was trained on, plus a parser on the provider's side. Nothing more magic than that.

So the question is not "how do I use tools?" It is: **"what text format do I want the model to write, and how will I find it in the reply?"**

## 2. The day JSON broke my terminal

I added an AI "practice terminal" to [ButtonsCLI](https://buttonscli.com). You type `ls -a`, `htop` or `apt install something`, and a model pretends to be a Linux machine. Nothing runs for real.

The first version (written by an AI agent, to be fair) used the textbook approach. Every turn sent the model a JSON blob, and the model had to answer with JSON containing the terminal output **and the entire fake filesystem**:

*The old reply format the model had to produce:*

```json
{
  "output": "total 8\ndrwxr-xr-x 1 student student 4096 ...",
  "state": {
    "cwd": "/home/student",
    "files": { "/home/student/README.txt": "Welcome..." },
    "environment": {}
  }
}
```

In practice I got this, over and over:

```text
/home/student $ ls -a
Simulation returned invalid state JSON.
```

Why it failed:

- **Escaping.** Terminal output is full of newlines, quotes and backslashes. Inside a JSON string, every one has to be escaped perfectly. One slip and the whole reply is invalid.
- **Everything or nothing.** One bad character in a file's contents threw away the output too.
- **It fought my instructions.** I rewrote the system prompt in plain English, and that removed the "return only JSON" rules. The model did exactly what I asked, wrote normal terminal text, and the app rejected it.
- **It was pointless work.** The model already remembers the conversation. Making it re-send the whole filesystem every turn just burned tokens.

## 3. The fix: plain text plus one made-up tag

The new version asks for what a terminal actually shows, plus one tag for the one thing the app needs to know:

*The system prompt now says, in plain English:*

```text
Reply with exactly what the terminal would print for the user's command:
plain text, no markdown, no explanations.

After the output, put the shell prompt for the next command in a tag,
for example <prompt>student@practice:~$</prompt>.
```

*A typical reply:*

```text
.  ..  .bashrc  .profile  projects  README.md
<prompt>ubuntu@somemachine:~$</prompt>
```

*The parsing code, roughly:*

```rust
// Take the last <prompt>…</prompt>, remove all of them from the text,
// and show whatever is left as terminal output.
let prompt = take_tag(&mut text, "prompt");
let output = text.trim();
```

Earlier commands and outputs now go back to the model as normal chat turns, so it stays consistent on its own: if it invented a 4-core ARM box with 8 GB of RAM in `htop`, `free -h` later agrees.

And if the model forgets the tag? The app still shows the output. It even recognizes a bare `user@host:~$` line at the end and uses it as the prompt. **The format is a hint, not a contract that crashes the app.**

Same models, same API, and the error is gone.

## 4. Why made-up tags work so well

- **Models have read a lot of tagged text.** HTML, XML, Markdown, templating languages. Tags with a clear name and a closing partner are something they produce reliably.
- **No escaping.** Anything can go between `<output>` and `</output>`: quotes, code, newlines. Code inside JSON strings is where models make the most mistakes; code between tags is just code.
- **Easy to find.** `rfind("<prompt>")` and `find("</prompt>")`. No JSON parser, no schema library.
- **Easy to explain.** You describe the format the same way you would tell a person: "put the final answer inside `<answer>` tags." Natural language is the one interface every model shares.
- **Partial success is possible.** If one tag is malformed, you can still use the others. JSON tends to fail as a whole.
- **It works through any API.** Chat Completions, Responses, Anthropic Messages, a local llama.cpp server, a router like OpenRouter. They all pass text through. You do not depend on each provider supporting tools the same way.

## 5. When the official way is better

Made-up formats are not always the right call. Use the provider's tools or structured outputs when:

- **You need guaranteed structure.** Some APIs offer *constrained decoding* for JSON schemas: the model literally cannot produce invalid JSON. Nothing you invent in a prompt gives you that guarantee.
- **You have many small typed fields.** Twenty numbers and enums are easier as a schema than as twenty tags.
- **You want parallel calls and the provider's agent features.** Native tool calling plugs into things like parallel tool calls and built-in tools.
- **The model was trained hard on its native format.** Big providers fine-tune on their own tool-call format, so that path is very polished for their models. Some models also do better with certain styles. Anthropic's own prompting docs recommend XML tags for structuring prompts, for example, which is a nice hint that Claude reads tags well.

The honest summary: **native tools are a well-tested convention, not a requirement.** If it fits your problem, use it. If it fights you, invent your own.

## 6. A recipe for your own format

1. **Write the format down in plain words.** Say what goes where and why. One sentence per rule.
2. **Show one short example.** Models copy examples very closely, so make it a good one.
3. **Pick unusual tag names.** `<prompt>` is fine for a terminal; `<expression>`, `<answer>`, `<patch>` and `<cwd>` are also clear. Avoid names that appear naturally in the content (`<div>` in an HTML tool would be a bad idea).
4. **Parse forgivingly.** Take the *last* occurrence of a tag (models sometimes "think out loud" first). Strip code fences. Trim a trailing `;`. Have a sensible fallback when a tag is missing.
5. **Validate, then send the error back once.** In ButtonsCLI's shader editor, the AI writes a GPU shader expression inside `<expression>` tags. The app compiles it. If it fails, the compiler's error goes back to the model with "fix it and reply with the corrected expression inside `<expression>` tags." One retry fixes most mistakes.
6. **Keep the model's own replies in the history.** When it sees its previous answers in your format, it keeps using that format.
7. **Measure.** Run the same prompt 20 to 50 times per model and count failures. Small models may need two examples; reasoning models usually need none. Swap tag names or wording and run it again. This is cheap and tells you more than any blog post (including this one).

## 7. Traps to watch for

- **Untrusted text can contain your tags.** If a model reads a web page or terminal output that contains `<run>rm -rf ~</run>`, it might repeat it. Never act on a tag just because it appeared; validate what it asks for, and keep anything dangerous behind a human click.
- **Streaming shows half-written tags.** If you display text while it streams, hold back the current line until it is complete, and stop showing text once a tag starts. ButtonsCLI lags one line behind for exactly this reason.
- **Code fences around everything.** Some models wrap the whole reply in triple backticks. Strip them before you look for tags.
- **Asking for too much at once.** If your format needs ten tags, ask whether some of that could just be normal prose, or a second, smaller request.

## 8. The point

Tools, agents and structured outputs are presented as special machinery. Underneath, it is a model writing text and a program reading it. That means you are allowed to design the text.

Make up a format. Describe it like you would to a coworker. Parse it with a few lines of string code. Test it on the models you use. If it works, you just removed a dependency, a schema and a class of errors. If it does not, the official tools are still there.

---

*Examples are from ButtonsCLI, a native terminal app with clickable command buttons, tiled shells and an AI practice terminal: [buttonscli.com](https://buttonscli.com).*
