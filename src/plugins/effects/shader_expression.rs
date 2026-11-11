//! A color expression, or a short body of `let` lines ending in `return`,
//! with `var`, `if`/`else` and fixed-count `for` loops unless the Shader
//! options switch those off. No `while`/`loop`, own functions or resources.
use crate::theme::CustomShader;
#[cfg(not(target_arch = "wasm32"))]
use eframe::wgpu::naga;

pub(crate) const MAX_SOURCE_BYTES: usize = crate::theme::MAX_CUSTOM_SHADER_BYTES;

/// The generated function's first line. `strength` is the user's Strength
/// slider (0..1) and `bg` the terminal background color.
const HEADER: &str = "fn custom_pattern(p: vec2<f32>, t: f32, knobs: vec4<f32>, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>, c4: vec3<f32>, c5: vec3<f32>, strength: f32, bg: vec3<f32>) -> vec3<f32> {\n";

/// Names the code gets for free; sliders cannot reuse them.
pub(crate) const RESERVED: [&str; 10] = [
    "p", "t", "knobs", "c1", "c2", "c3", "c4", "c5", "strength", "bg",
];

/// The helper functions custom code may call. Sliders the AI adds cannot
/// take these names.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) const HELPER_NAMES: [&str; 22] = [
    "hash",
    "hash2",
    "noise",
    "cloud",
    "stars",
    "palette",
    "hsv",
    "fbm",
    "ridged",
    "flow",
    "voronoi",
    "rotate",
    "polar",
    "sd_circle",
    "sd_box",
    "sd_segment",
    "smin",
    "fill",
    "stroke",
    "halo",
    "tonemap",
    // Not in the helpers file: added with the previous frame (see `PREV_FRAME`).
    "prev",
];

/// Rough work per pixel for one helper call, in `hash` lookups. Helpers that
/// are not listed cost 1.
const HELPER_COSTS: [(&str, u32); 9] = [
    ("hash2", 2),
    ("prev", 2),
    ("noise", 4),
    ("stars", 3),
    ("cloud", 12),
    ("ridged", 12),
    ("fbm", 20),
    ("voronoi", 28),
    ("flow", 44),
];

/// Most helper work one shader may ask for per pixel, unless Settings → Debug
/// changes it. With the 256-expression limit this bounds the cost of every
/// frame: about twenty `cloud` calls, twelve `fbm` or five `flow`.
pub(crate) const MAX_COST: u32 = crate::settings::ShaderLab::DEFAULT_COST_LIMIT;

/// Most Naga expressions in one shader. It was 256, which a 30-line shader
/// from a strong model overran, with an error that did not say so.
const MAX_EXPRESSIONS: usize = 1024;

/// Most times one `for` loop may run, and most times the innermost body of
/// nested loops may run.
const MAX_LOOP_TRIPS: u32 = 64;
const MAX_NESTED_TRIPS: u32 = 256;
/// What one pass through a loop body costs before its helper calls.
const LOOP_STEP_COST: u32 = 2;

/// What custom code may contain. The defaults are what ships; the Shader
/// options under Create with AI can change them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Rules {
    /// Accept `var`, assignment, `if`/`else` and `for` loops whose count is a
    /// fixed number (see [`loop_trips`]). `while` and `loop` stay out: nothing
    /// here can prove they end.
    pub(crate) loops: bool,
    /// Most helper work per pixel; 0 is no limit.
    pub(crate) cost_limit: u32,
    /// Accept `prev(q)`, the color this shader drew last frame (experiment,
    /// Settings → Debug).
    pub(crate) feedback: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            loops: true,
            cost_limit: MAX_COST,
            feedback: false,
        }
    }
}

static LOOPS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static COST_LIMIT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(MAX_COST);
static FEEDBACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// The rules every shader is checked against from now on.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn set_rules(rules: Rules) {
    LOOPS.store(rules.loops, std::sync::atomic::Ordering::Relaxed);
    COST_LIMIT.store(rules.cost_limit, std::sync::atomic::Ordering::Relaxed);
    FEEDBACK.store(rules.feedback, std::sync::atomic::Ordering::Relaxed);
}

pub(crate) fn rules() -> Rules {
    Rules {
        loops: LOOPS.load(std::sync::atomic::Ordering::Relaxed),
        cost_limit: COST_LIMIT.load(std::sync::atomic::Ordering::Relaxed),
        feedback: FEEDBACK.load(std::sync::atomic::Ordering::Relaxed),
    }
}

/// Whether `code` calls `prev(...)`: it reads the previous frame, so it is
/// drawn through the feedback path. A `let prev` alone does not count.
pub(crate) fn uses_feedback(code: &str) -> bool {
    let clean = without_comments(code);
    let mut rest = clean.as_str();
    while let Some(at) = rest.find("prev") {
        let before = rest[..at].chars().next_back();
        let after = rest[at + 4..].trim_start();
        let whole = !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
        if whole && after.starts_with('(') {
            return true;
        }
        rest = &rest[at + 4..];
    }
    false
}

/// Stands in for `prev` while code is checked on its own; the real one, which
/// reads a texture, is added by [`combine`].
const PREV_STUB: &str = "fn prev(q: vec2<f32>) -> vec3<f32> { return vec3<f32>(q, 0.0); }\n";

/// The previous frame for code that calls `prev` (see [`combine`]). `{Y}` is
/// `-` when p.y grows upward. Bindings 1 and 2 follow the uniforms.
const PREV_FRAME: &str = "@group(0) @binding(1) var prev_frame: texture_2d<f32>;
@group(0) @binding(2) var prev_sampler: sampler;
fn prev(q: vec2<f32>) -> vec3<f32> {
    let size = vec2<f32>(u.geometry.z / u.geometry.w, 1.0) * u.settings.z;
    let uv = vec2<f32>(q.x, {Y}q.y) / size + 0.5;
    return textureSampleLevel(prev_frame, prev_sampler, uv, 0.0).rgb;
}
";

/// How many times each `for` loop in `code` runs, in source order, or why a
/// loop is not allowed. Only this shape is accepted, so every loop provably
/// ends: `for (var i = 0; i < 8; i++) { ... }` with whole-number bounds, a
/// step of one, and a body that never changes the counter.
fn loop_trips(code: &str) -> Result<Vec<u32>, String> {
    const SHAPE: &str = "Write loops as `for (var i = 0; i < 8; i++) { ... }`: whole-number bounds, a step of one, at most 64 turns, and do not change the counter inside. `while` and `loop` are not allowed.";
    let clean = without_comments(code);
    let mut tokens: Vec<&str> = Vec::new();
    let mut start = None;
    for (index, c) in clean.char_indices() {
        let word = c.is_ascii_alphanumeric() || c == '_';
        if !word {
            if let Some(from) = start.take() {
                tokens.push(&clean[from..index]);
            }
            if !c.is_whitespace() {
                tokens.push(&clean[index..index + c.len_utf8()]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(from) = start {
        tokens.push(&clean[from..]);
    }
    let number = |token: &str| {
        token
            .trim_end_matches(['i', 'u'])
            .parse::<i64>()
            .map_err(|_| SHAPE.to_owned())
    };
    let mut trips = Vec::new();
    for (at, token) in tokens.iter().enumerate() {
        if matches!(*token, "while" | "loop") {
            return Err(SHAPE.into());
        }
        if *token != "for" {
            continue;
        }
        let mut rest = tokens[at + 1..].iter().copied();
        let mut next = || rest.next().ok_or_else(|| SHAPE.to_owned());
        let expect = |found: &str, wanted: &str| {
            if found == wanted {
                Ok(())
            } else {
                Err(SHAPE.to_owned())
            }
        };
        expect(next()?, "(")?;
        expect(next()?, "var")?;
        let counter = next()?;
        let mut token = next()?;
        if token == ":" {
            next()?;
            token = next()?;
        }
        expect(token, "=")?;
        let low = number(next()?)?;
        expect(next()?, ";")?;
        expect(next()?, counter)?;
        expect(next()?, "<")?;
        let mut token = next()?;
        let inclusive = token == "=";
        if inclusive {
            token = next()?;
        }
        let high = number(token)? + i64::from(inclusive);
        expect(next()?, ";")?;
        expect(next()?, counter)?;
        let step: Vec<&str> = [next()?, next()?].into();
        if step == ["+", "="] {
            expect(next()?, "1")?;
        } else if step[0] == "=" {
            expect(step[1], counter)?;
            expect(next()?, "+")?;
            expect(next()?, "1")?;
        } else if step != ["+", "+"] {
            return Err(SHAPE.into());
        }
        expect(next()?, ")")?;
        expect(next()?, "{")?;
        // Nothing in the body may change or take the address of the counter.
        let mut depth = 1;
        let body: Vec<&str> = rest
            .take_while(|token| {
                depth += i32::from(*token == "{") - i32::from(*token == "}");
                depth > 0
            })
            .collect();
        for (index, token) in body.iter().enumerate() {
            let after = |offset: usize| body.get(index + offset).copied().unwrap_or("");
            let before = index.checked_sub(1).map_or("", |index| body[index]);
            let changed = (after(1) == "=" && after(2) != "=")
                || (matches!(after(1), "+" | "-" | "*" | "/" | "%") && after(2) == "=")
                || (matches!(after(1), "+" | "-") && after(2) == after(1));
            if *token == counter && (changed || before == "&") {
                return Err(SHAPE.into());
            }
        }
        let count = high - low;
        if !(1..=i64::from(MAX_LOOP_TRIPS)).contains(&count) {
            return Err(SHAPE.into());
        }
        trips.push(count as u32);
    }
    Ok(trips)
}

/// Adds up what `block` costs and rejects statements the rules do not allow.
fn block_cost(
    block: &naga::Block,
    module: &naga::Module,
    rules: Rules,
    trips: &mut std::slice::Iter<'_, u32>,
    times: u32,
    cost: &mut u64,
) -> Result<(), String> {
    const STRICT: &str = "`if`, loops, `var` and changing a value are not allowed here. Use `let` lines with select(), mix() or step(), and end with one `return`.";
    for statement in block.iter() {
        match statement {
            naga::Statement::Emit(_) | naga::Statement::Return { .. } => {}
            // Calls can only reach the fixed, bounded helpers: the function
            // count check rejects any added function.
            naga::Statement::Call { function, .. } => {
                let name = module.functions[*function].name.as_deref().unwrap_or("");
                let price = HELPER_COSTS
                    .iter()
                    .find(|(helper, _)| *helper == name)
                    .map_or(1, |(_, price)| *price);
                *cost += u64::from(price) * u64::from(times);
            }
            naga::Statement::Store { .. } | naga::Statement::Break | naga::Statement::Continue
                if rules.loops => {}
            naga::Statement::Block(inner) if rules.loops => {
                block_cost(inner, module, rules, trips, times, cost)?;
            }
            naga::Statement::If { accept, reject, .. } if rules.loops => {
                block_cost(accept, module, rules, trips, times, cost)?;
                block_cost(reject, module, rules, trips, times, cost)?;
            }
            naga::Statement::Loop {
                body, continuing, ..
            } if rules.loops => {
                let count = trips
                    .next()
                    .ok_or("A loop here is not a plain `for` loop.")?;
                let times = times.saturating_mul(*count);
                if times > MAX_NESTED_TRIPS {
                    return Err(format!(
                        "Nested loops may run their inner body at most {MAX_NESTED_TRIPS} times."
                    ));
                }
                *cost += u64::from(LOOP_STEP_COST) * u64::from(times);
                block_cost(body, module, rules, trips, times, cost)?;
                block_cost(continuing, module, rules, trips, times, cost)?;
            }
            _ if rules.loops => {
                return Err(
                    "Only `let`, `var`, assignment, `if`/`else`, `for` and `return` are supported."
                        .into(),
                )
            }
            _ => return Err(STRICT.into()),
        }
    }
    Ok(())
}

/// Whether `source` uses `name` as a whole identifier, comments aside.
pub(crate) fn uses(source: &str, name: &str) -> bool {
    without_comments(source)
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|word| word == name)
}

/// `source` with `//` and (nested) `/* */` comments replaced by spaces. WGSL
/// has no string literals, so what is left is only code.
fn without_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut code = Vec::with_capacity(bytes.len());
    let mut index = 0;
    let mut depth = 0_usize;
    while index < bytes.len() {
        let pair = &bytes[index..(index + 2).min(bytes.len())];
        if pair == b"/*" {
            depth += 1;
            index += 2;
        } else if depth > 0 && pair == b"*/" {
            depth -= 1;
            index += 2;
            code.push(b' ');
        } else if depth > 0 {
            index += 1;
        } else if pair == b"//" {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
        } else {
            code.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&code).into_owned()
}

fn is_body(source: &str) -> bool {
    uses(source, "return")
}

pub(crate) fn function(custom: &CustomShader) -> Result<String, String> {
    function_with(custom, rules())
}

pub(crate) fn function_with(custom: &CustomShader, rules: Rules) -> Result<String, String> {
    if custom.expression.len() > MAX_SOURCE_BYTES || custom.variables.len() > 4 {
        return Err("Use at most 4096 source bytes and four variables.".into());
    }
    if custom
        .expression
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|word| matches!(word, "array" | "binding_array"))
    {
        return Err("Array types are disabled in color expressions.".into());
    }
    let mut names = std::collections::HashSet::new();
    let mut source = String::from(HEADER);
    for (index, variable) in custom.variables.iter().enumerate() {
        let name = &variable.name;
        if name.is_empty()
            || name.len() > 32
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || !name.as_bytes()[0].is_ascii_alphabetic()
            || name.starts_with("__")
            || RESERVED.contains(&name.as_str())
            || !names.insert(name)
        {
            return Err("Variable names must be unique letters/numbers/underscores, start with a letter, and avoid p, t, knobs, c1–c5, strength and bg.".into());
        }
        if ![variable.min, variable.max, variable.value]
            .into_iter()
            .all(f32::is_finite)
            || variable.min >= variable.max
            || variable.min < -100.0
            || variable.max > 100.0
        {
            return Err(
                "Variable ranges must be finite, increasing, and within -100 to 100.".into(),
            );
        }
        source.push_str(&format!("let {name} = knobs[{index}];\n"));
    }
    // A body with its own `return` (and `let` lines) is used as is; a bare
    // expression is wrapped. Bodies are far easier for models to get right.
    if is_body(&custom.expression) {
        source.push_str(custom.expression.trim());
        source.push_str("\n}");
    } else {
        // The `;` goes on its own line so a trailing `//` comment cannot eat it.
        source.push_str(&format!(
            "return {}\n;\n}}",
            custom.expression.trim().trim_end_matches(';')
        ));
    }
    // The same code can pass under one set of rules and fail under another.
    type Checked = std::collections::HashMap<(String, bool, u32, bool), Result<(), String>>;
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Checked>> = std::sync::OnceLock::new();
    let key = (source, rules.loops, rules.cost_limit, rules.feedback);
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    if let Some(result) = cache.get(&key) {
        result.clone()?;
        return Ok(key.0);
    }
    let result = validate(&key.0, rules);
    if cache.len() >= 64 {
        cache.clear();
    }
    cache.insert(key.clone(), result.clone());
    result?;
    Ok(key.0)
}

pub(crate) fn program(custom: &CustomShader) -> Result<String, String> {
    Ok(combine(&function(custom)?, custom.y_up))
}

#[cfg(all(test, target_os = "windows"))]
pub(crate) fn program_with(custom: &CustomShader, rules: Rules) -> Result<String, String> {
    Ok(combine(&function_with(custom, rules)?, custom.y_up))
}

/// Helpers (`hash`, `noise`, `cloud`, `stars`, `palette`, `hsv`) shared by the
/// presets and callable from custom expressions.
pub(crate) const HELPERS: &str = include_str!("shader_helpers.wgsl");
/// The complete preset program.
pub(crate) const SOURCE: &str = concat!(
    include_str!("shader_helpers.wgsl"),
    include_str!("shader_background.wgsl")
);

/// `y_up` flips p.y so it grows upward, as most shader code expects; screen
/// pixels (and the presets) count downward.
fn combine(source: &str, y_up: bool) -> String {
    let position = if y_up { "vec2<f32>(p.x, -p.y)" } else { "p" };
    let mut full = SOURCE.replace(
        PRESET_COLOR,
        &format!("let color = clamp(custom_pattern({position}, t, u.knobs, u.first.rgb, u.second.rgb, u.third.rgb, u.fourth.rgb, u.fifth.rgb, u.settings.y, u.base.rgb), vec3<f32>(0.0), vec3<f32>(1.0));\n    glow = 1.0;\n    vivid = vec3<f32>(-1.0);")
    );
    // Code that uses `strength` decides how it shows; otherwise the host fades
    // the pattern into the background by Strength, as before. A mention in a
    // comment does not count.
    let code = source.strip_prefix(HEADER).unwrap_or(source);
    if uses(code, "strength") {
        full = full.replace(HOST_FADE, "clamp(glow, 0.0, 1.0)");
    }
    if uses_feedback(code) {
        full.push('\n');
        full.push_str(&PREV_FRAME.replace("{Y}", if y_up { "-" } else { "" }));
    }
    format!("{full}\n{source}")
}

/// The two preset lines that `combine` swaps for the custom function. A test
/// checks the preset program still has them, so an edit there cannot quietly
/// turn every custom shader back into a preset.
const PRESET_COLOR: &str =
    "let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);";
const HOST_FADE: &str = "clamp(glow * u.settings.y, 0.0, 1.0)";

/// Line numbers in diagnostics refer to the user's expression, not the prelude.
fn helper_lines() -> usize {
    HELPERS.lines().count()
}

/// Rewrite `wgsl:LINE:COL` so it counts from the generated function's first line.
fn shift_line_numbers(line: &str, offset: usize) -> String {
    let Some(start) = line.find("wgsl:") else {
        return line.to_owned();
    };
    let digits_at = start + "wgsl:".len();
    let digits: String = line[digits_at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    match digits.parse::<usize>() {
        Ok(number) if number > offset => format!(
            "{}expression:{}{}",
            &line[..start],
            number - offset,
            &line[digits_at + digits.len()..]
        ),
        _ => line.to_owned(),
    }
}

fn validate(source: &str, rules: Rules) -> Result<(), String> {
    // The helpers come first so the expression can call them; anything beyond
    // them plus the one generated function is rejected below.
    let code = source.strip_prefix(HEADER).unwrap_or(source);
    if !rules.feedback && uses_feedback(code) {
        return Err("`prev()` (last frame's color) is an experiment: switch on Feedback in Settings → Debug to use it.".into());
    }
    let helper_count = naga::front::wgsl::parse_str(HELPERS)
        .map(|module| module.functions.len())
        .unwrap_or(0)
        + usize::from(rules.feedback);
    let stub = if rules.feedback { PREV_STUB } else { "" };
    let standalone = format!("{HELPERS}{stub}{source}");
    let module = naga::front::wgsl::parse_str(&standalone).map_err(|error| {
        error
            .emit_to_string(&standalone)
            .lines()
            .map(|line| shift_line_numbers(line, helper_lines() + stub.lines().count()))
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    if module.functions.len() != helper_count + 1
        || !module.entry_points.is_empty()
        || !module.global_variables.is_empty()
        || !module.constants.is_empty()
        || !module.overrides.is_empty()
    {
        return Err("Enter one color expression, without declarations or resources.".into());
    }
    if module.types.iter().any(|(_, ty)| {
        !matches!(
            ty.inner,
            naga::TypeInner::Scalar(_)
                | naga::TypeInner::Vector { .. }
                | naga::TypeInner::Matrix { .. }
        )
    }) {
        return Err("Only scalar, vector and matrix math is supported; arrays, structures and resources are disabled.".into());
    }
    let Some((_, function)) = module
        .functions
        .iter()
        .find(|(_, function)| function.name.as_deref() == Some("custom_pattern"))
    else {
        return Err("Enter one color expression, without declarations or resources.".into());
    };
    let strict_returns = function
        .body
        .iter()
        .filter(|statement| matches!(statement, naga::Statement::Return { .. }))
        .count();
    // Naga counts every value, name and operator, so this is a few hundred
    // real operations: room for a rich shader, still a bounded one.
    let size = function.expressions.len();
    if size > MAX_EXPRESSIONS {
        return Err(format!(
            "This shader is too long: it has {size} operations and the limit is {MAX_EXPRESSIONS}. Remove a layer or a few details."
        ));
    }
    if !rules.loops && strict_returns != 1 {
        return Err("End with exactly one `return <color>;` as the last line.".into());
    }
    let trips = if rules.loops {
        loop_trips(source.strip_prefix(HEADER).unwrap_or(source))?
    } else {
        Vec::new()
    };
    let mut remaining = trips.iter();
    let mut cost = 0_u64;
    block_cost(&function.body, &module, rules, &mut remaining, 1, &mut cost)?;
    if remaining.next().is_some() {
        return Err("A `for` loop here could not be checked; simplify it.".into());
    }
    let limit = rules.cost_limit;
    if limit > 0 && cost > u64::from(limit) {
        return Err(format!(
            "This shader is too heavy for slower computers: its helper calls cost {cost} and the limit is {limit}. Use fewer flow (44), voronoi (28), fbm (20), cloud (12), ridged (12) or noise (4) calls, and reuse a result in a `let` instead of calling again. Inside a loop every call counts once per turn."
        ));
    }
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|error| error.to_string())?;
    // Check the complete fragment too, including name collisions and stage rules,
    // before any user expression reaches wgpu's pipeline creation.
    let full = combine(source, true);
    let module =
        naga::front::wgsl::parse_str(&full).map_err(|error| error.emit_to_string(&full))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|error| error.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expression_accepts_color_math_and_rejects_source_injection() {
        let mut custom = CustomShader::default();
        assert!(function(&custom).is_ok());
        for invalid in [
            "1.0",
            "c1; loop {} return c2",
            "c1; } fn extra() -> f32 { return 0.0; } fn more() -> vec3<f32> { return c1",
            "missing",
            "c1; } struct Uniforms { bad: f32, //",
            "vec3<f32>(array<f32, 100000000>()[0])",
        ] {
            custom.expression = invalid.into();
            assert!(function(&custom).is_err(), "{invalid}");
        }
        custom = CustomShader::default();
        custom.variables[0].name = "p".into();
        assert!(function(&custom).is_err());
        custom = CustomShader::default();
        custom.variables[0].name = "noise".into();
        assert!(function(&custom).is_err(), "helper names are reserved");
    }

    #[test]
    fn code_that_uses_strength_owns_it_and_other_code_is_faded() {
        let owns = CustomShader {
            expression:
                "let color = palette(p.x + t);\nreturn mix(bg, color * (0.5 + strength), strength);"
                    .into(),
            ..Default::default()
        };
        let program = program(&owns).unwrap();
        assert!(!program.contains("glow * u.settings.y"));
        assert!(program.contains("u.settings.y, u.base.rgb)"));
        let faded = program_for("palette(p.x + t)");
        assert!(faded.contains("glow * u.settings.y"));
        // Y up (the default) flips p.y for the code; c4 and c5 reach it too.
        assert!(program.contains("custom_pattern(vec2<f32>(p.x, -p.y), t"));
        assert!(program.contains("u.fourth.rgb, u.fifth.rgb"));
        let screen_order = super::program(&CustomShader {
            y_up: false,
            ..owns.clone()
        })
        .unwrap();
        assert!(screen_order.contains("custom_pattern(p, t"));
        assert!(function(&CustomShader {
            expression: "return mix(c4, c5, 0.5);".into(),
            variables: Vec::new(),
            ..Default::default()
        })
        .is_ok());
        // A slider can't take a name the code already gets.
        let mut clash = CustomShader::default();
        clash.variables[0].name = "strength".into();
        assert!(function(&clash).is_err());
    }

    #[test]
    fn a_comment_cannot_claim_the_strength_slider() {
        for code in [
            "/* strength */ return c1;",
            "// uses strength\nreturn c1;",
            "/* outer /* strength */ still a comment */ return c1;",
        ] {
            assert!(
                program_for(code).contains(HOST_FADE),
                "the host still fades {code}"
            );
        }
        assert!(!program_for("return mix(bg, c1, strength); // done").contains(HOST_FADE));
        // A comment that says "return" does not turn an expression into a body.
        assert!(function(&CustomShader {
            expression: "c1 * 0.5 // return later".into(),
            variables: Vec::new(),
            ..Default::default()
        })
        .is_ok());
        assert!(SOURCE.contains(PRESET_COLOR) && SOURCE.contains(HOST_FADE));
    }

    #[test]
    fn helper_work_per_pixel_is_bounded() {
        let body = |calls: usize, helper: &str| CustomShader {
            expression: format!(
                "let v = {};\nreturn c1 * v;",
                (0..calls)
                    .map(|index| format!("{helper}(p + {index}.0)"))
                    .collect::<Vec<_>>()
                    .join(" + ")
            ),
            variables: Vec::new(),
            ..Default::default()
        };
        let twelve = function(&body(12, "fbm"));
        assert!(twelve.is_ok(), "{twelve:?}");
        let error = function(&body(13, "fbm")).unwrap_err();
        assert!(
            error.contains("too heavy") && error.contains("260"),
            "{error}"
        );
        assert!(function(&body(9, "voronoi(p).x + hash")).is_err());
        // Every helper has a name here, and every priced helper exists.
        let module = naga::front::wgsl::parse_str(HELPERS).unwrap();
        let names: Vec<&str> = module
            .functions
            .iter()
            .filter_map(|(_, function)| function.name.as_deref())
            .collect();
        for name in &names {
            assert!(
                HELPER_NAMES.contains(name),
                "{name} is missing from HELPER_NAMES"
            );
        }
        // `prev` comes with the previous frame, not the helpers file.
        assert_eq!(names.len() + 1, HELPER_NAMES.len());
        for (name, _) in HELPER_COSTS {
            assert!(
                names.contains(&name) || name == "prev",
                "{name} is priced but missing"
            );
        }
    }

    #[test]
    fn new_helpers_compile_and_old_code_may_reuse_their_names() {
        let code = "let cells = voronoi(p * 4.0);\nlet d = smin(sd_circle(p, 0.3), sd_box(rotate(p, t), vec2<f32>(0.2, 0.1)), 0.1);\nlet line = sd_segment(p, vec2<f32>(-0.5, 0.0), hash2(floor(p)));\nlet shape = fill(d) + stroke(line, 0.02) + halo(d, 0.2) * polar(p).y;\nlet smoke = flow(p, t) * fbm(p) * ridged(p) * cells.y;\nreturn mix(bg, tonemap(c1 * shape + c2 * smoke), strength);";
        let custom = CustomShader {
            expression: code.into(),
            variables: Vec::new(),
            ..Default::default()
        };
        assert!(program(&custom).is_ok(), "{:?}", program(&custom));
        // Shaders saved before these helpers existed may have used the names.
        let old = CustomShader {
            expression: "let flow = sin(p.x + t);\nlet fill = 0.5;\nlet rotate = c1;\nreturn rotate * flow * fill;".into(),
            variables: Vec::new(),
            ..Default::default()
        };
        assert!(function(&old).is_ok(), "{:?}", function(&old));
    }

    #[test]
    fn lab_rules_accept_fixed_loops_and_branches_and_still_bound_the_work() {
        let loose = Rules {
            loops: true,
            cost_limit: MAX_COST,
            feedback: false,
        };
        let check = |code: &str, rules: Rules| {
            function_with(
                &CustomShader {
                    expression: code.into(),
                    variables: Vec::new(),
                    ..Default::default()
                },
                rules,
            )
        };
        let orbs = "var glow = 0.0;\nfor (var i = 0; i < 8; i++) {\n    let k = f32(i);\n    let c = vec2<f32>(0.7 * sin(t * 0.3 + k * 1.7), 0.35 * cos(t * 0.4 + k * 2.3));\n    glow += halo(sd_circle(p - c, 0.03), 0.05);\n}\nif (p.x > 0.0) { glow = glow * 0.5; } else { glow = glow * 1.5; }\nreturn mix(bg, tonemap(c1 * glow), strength);";
        assert!(check(orbs, loose).is_ok(), "{:?}", check(orbs, loose));
        assert!(check(orbs, Rules::default()).is_ok(), "on by default");
        let strict = Rules {
            loops: false,
            ..loose
        };
        assert!(check(orbs, strict).is_err(), "refused when switched off");
        // Old code means the same under the looser rules.
        assert!(check("let r = length(p);\nreturn palette(r - t);", loose).is_ok());
        for (why, code) in [
            ("while", "var x = 0.0;\nwhile (x < 1.0) { x += 0.1; }\nreturn c1 * x;"),
            ("loop", "var x = 0.0;\nloop { x += 0.1; if (x > 1.0) { break; } }\nreturn c1 * x;"),
            ("no end", "var x = 0.0;\nfor (var i = 0; ; i++) { x += 0.1; }\nreturn c1 * x;"),
            ("slider bound", "var x = 0.0;\nfor (var i = 0; i < i32(t); i++) { x += 0.1; }\nreturn c1 * x;"),
            ("counter reset", "var x = 0.0;\nfor (var i = 0; i < 4; i++) { i = 0; x += 0.1; }\nreturn c1 * x;"),
            ("counter step back", "var x = 0.0;\nfor (var i = 0; i < 4; i++) { i -= 1; x += 0.1; }\nreturn c1 * x;"),
            ("counts down", "var x = 0.0;\nfor (var i = 4; i > 0; i--) { x += 0.1; }\nreturn c1 * x;"),
            ("too long", "var x = 0.0;\nfor (var i = 0; i < 65; i++) { x += 0.1; }\nreturn c1 * x;"),
            ("nested too long", "var x = 0.0;\nfor (var i = 0; i < 32; i++) { for (var j = 0; j < 9; j++) { x += 0.1; } }\nreturn c1 * x;"),
            ("heavy body", "var x = 0.0;\nfor (var i = 0; i < 16; i++) { x += fbm(p + f32(i)); }\nreturn c1 * x;"),
            ("own function", "return c1; } fn extra() -> f32 { return 1.0;"),
            ("array", "var a: array<f32, 4>;\nreturn c1;"),
            ("discard", "if (p.x > 0.0) { discard; }\nreturn c1;"),
        ] {
            assert!(check(code, loose).is_err(), "{why} must be refused");
        }
        // Inside a loop a helper counts once per turn: 8 * (2 + 20) = 176.
        let eight =
            "var x = 0.0;\nfor (var i = 0; i < 8; i++) { x += fbm(p + f32(i)); }\nreturn c1 * x;";
        assert!(check(eight, loose).is_ok(), "{:?}", check(eight, loose));
        let tight = Rules {
            cost_limit: 100,
            ..loose
        };
        assert!(check(eight, tight).unwrap_err().contains("176"));
        // No limit at all is a lab choice too.
        let heavy = "return c1 * (fbm(p) + flow(p, t) + flow(p * 2.0, t) + flow(p * 3.0, t) + flow(p * 4.0, t) + flow(p * 5.0, t) + flow(p * 6.0, t));";
        assert!(check(heavy, Rules::default()).is_err());
        let unlimited = Rules {
            cost_limit: 0,
            ..Rules::default()
        };
        assert!(check(heavy, unlimited).is_ok());
        assert_eq!(
            loop_trips(
                "for (var i: i32 = 2; i <= 5; i += 1) { } for (var j = 0u; j < 3u; j = j + 1) { }"
            ),
            Ok(vec![4, 3])
        );
    }

    #[test]
    fn feedback_code_reads_the_previous_frame_only_when_switched_on() {
        let trail = CustomShader {
            expression: "let d = halo(sd_circle(p - vec2<f32>(sin(t), 0.0), 0.05), 0.05);\nreturn max(c1 * d, prev(p) * 0.95);".into(),
            variables: Vec::new(),
            ..Default::default()
        };
        let off = Rules::default();
        let on = Rules {
            feedback: true,
            ..off
        };
        let error = function_with(&trail, off).unwrap_err();
        assert!(error.contains("Settings → Debug"), "{error}");
        let source = function_with(&trail, on).unwrap();
        let program = combine(&source, true);
        assert!(program.contains("var prev_frame: texture_2d<f32>"));
        assert!(program.contains("vec2<f32>(q.x, -q.y)"));
        assert!(combine(&source, false).contains("vec2<f32>(q.x, q.y)"));
        // Plain code gets no texture, so it keeps the plain pipeline.
        let plain = function_with(&CustomShader::default(), on).unwrap();
        assert!(!combine(&plain, true).contains("prev_frame"));
        // `let prev` or a comment is not a call; errors still count from line 1.
        assert!(!uses_feedback(
            "let prev = 0.5; // prev(p)\nreturn c1 * prev;"
        ));
        assert!(uses_feedback("return prev (p);"));
        assert!(!uses_feedback("return preview(p);"));
        let broken = CustomShader {
            expression: "return prev(p) + oops;".into(),
            variables: Vec::new(),
            ..Default::default()
        };
        let error = function_with(&broken, on).unwrap_err();
        let plain = function_with(
            &CustomShader {
                expression: "return c1 + oops;".into(),
                variables: Vec::new(),
                ..Default::default()
            },
            off,
        )
        .unwrap_err();
        let line = |error: &str| {
            error
                .split("expression:")
                .nth(1)
                .map(|rest| rest[..1].to_owned())
        };
        assert_eq!(line(&error), line(&plain), "{error}");
        // Reading the frame is priced like a cheap helper, per loop turn too.
        let heavy = CustomShader {
            expression: "var c = vec3<f32>(0.0);\nfor (var i = 0; i < 64; i++) { c += prev(p + f32(i) * 0.001); }\nfor (var i = 0; i < 64; i++) { c += prev(p - f32(i) * 0.001); }\nreturn c / 128.0;".into(),
            variables: Vec::new(),
            ..Default::default()
        };
        assert!(function_with(&heavy, on).unwrap_err().contains("too heavy"));
    }

    fn program_for(expression: &str) -> String {
        program(&CustomShader {
            expression: expression.into(),
            variables: Vec::new(),
            ..Default::default()
        })
        .unwrap()
    }

    #[test]
    fn expressions_can_call_helpers_but_not_add_functions() {
        let mut custom = CustomShader {
            expression:
                "palette(cloud(p) + t) * hsv(hash(p), 0.5, 1.0) + vec3<f32>(stars(p, t) * noise(p))"
                    .into(),
            ..Default::default()
        };
        assert!(function(&custom).is_ok());
        custom.expression =
            "c1; } fn palette2(x: f32) -> vec3<f32> { return vec3<f32>(x); } fn z() -> vec3<f32> { return c1".into();
        assert!(function(&custom).is_err());
        custom.expression = "vec3<f32>(oops)".into();
        let error = function(&custom).unwrap_err();
        assert!(
            !error.contains("wgsl:"),
            "line numbers point at the expression: {error}"
        );
    }

    #[test]
    fn short_bodies_work_and_strict_rules_refuse_loops_and_vars() {
        let mut custom = CustomShader {
            expression: "let r = length(p);\nlet glow = smoothstep(0.6, 0.0, r);\nreturn mix(c1, palette(r - t * 0.1), glow);".into(),
            ..Default::default()
        };
        assert!(function(&custom).is_ok(), "{:?}", function(&custom));
        let strict = Rules {
            loops: false,
            ..Rules::default()
        };
        for invalid in [
            "var x = 1.0;\nx = x + 1.0;\nreturn c1 * x;",
            "for (var i = 0; i < 3; i++) {}\nreturn c1;",
            "if (p.x > 0.0) { return c1; }\nreturn c2;",
            "let a = c1;",
        ] {
            custom.expression = invalid.into();
            assert!(function_with(&custom, strict).is_err(), "{invalid}");
        }
        custom.expression = "c1 * 0.5;".into();
        assert!(
            function(&custom).is_ok(),
            "a stray semicolon on an expression is fine"
        );
    }

    #[test]
    fn preset_program_validates_with_naga() {
        let module = naga::front::wgsl::parse_str(SOURCE).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
        assert!(program(&CustomShader::default()).is_ok());
    }
}
