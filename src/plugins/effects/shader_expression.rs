//! A color expression, or a short body of `let` lines ending in `return`.
//! No loops, branches, `var`, helper functions or resources.
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
pub(crate) const HELPER_NAMES: [&str; 21] = [
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
];

/// Rough work per pixel for one helper call, in `hash` lookups. Helpers that
/// are not listed cost 1.
const HELPER_COSTS: [(&str, u32); 8] = [
    ("hash2", 2),
    ("noise", 4),
    ("stars", 3),
    ("cloud", 12),
    ("ridged", 12),
    ("fbm", 20),
    ("voronoi", 28),
    ("flow", 44),
];

/// Most helper work one shader may ask for per pixel. Code has no loops, so
/// this and the 256-expression limit bound the cost of every frame: about
/// twenty `cloud` calls, twelve `fbm` or five `flow`.
pub(crate) const MAX_COST: u32 = 240;

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
    static CACHE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<String, Result<(), String>>>,
    > = std::sync::OnceLock::new();
    let mut cache = CACHE.get_or_init(Default::default).lock().unwrap();
    if let Some(result) = cache.get(&source) {
        result.clone()?;
        return Ok(source);
    }
    let result = validate(&source);
    if cache.len() >= 64 {
        cache.clear();
    }
    cache.insert(source.clone(), result.clone());
    result?;
    Ok(source)
}

pub(crate) fn program(custom: &CustomShader) -> Result<String, String> {
    Ok(combine(&function(custom)?, custom.y_up))
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
    if uses(source.strip_prefix(HEADER).unwrap_or(source), "strength") {
        full = full.replace(HOST_FADE, "clamp(glow, 0.0, 1.0)");
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

fn validate(source: &str) -> Result<(), String> {
    // The helpers come first so the expression can call them; anything beyond
    // them plus the one generated function is rejected below.
    let helper_count = naga::front::wgsl::parse_str(HELPERS)
        .map(|module| module.functions.len())
        .unwrap_or(0);
    let standalone = format!("{HELPERS}{source}");
    let module = naga::front::wgsl::parse_str(&standalone).map_err(|error| {
        error
            .emit_to_string(&standalone)
            .lines()
            .map(|line| shift_line_numbers(line, helper_lines()))
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
    if function.expressions.len() > 256
        || function.body.iter().any(|statement| {
            !matches!(
                statement,
                naga::Statement::Emit(_)
                    | naga::Statement::Return { .. }
                    // Calls can only reach the fixed, bounded helpers: the
                    // function count check above rejects any added function.
                    | naga::Statement::Call { .. }
            )
        })
        || function
            .body
            .iter()
            .filter(|statement| matches!(statement, naga::Statement::Return { .. }))
            .count()
            != 1
    {
        return Err(
            "Only a bounded color expression is supported; statements and loops are disabled."
                .into(),
        );
    }
    let cost: u32 = function
        .body
        .iter()
        .filter_map(|statement| match statement {
            naga::Statement::Call { function, .. } => module.functions[*function].name.as_deref(),
            _ => None,
        })
        .map(|name| {
            HELPER_COSTS
                .iter()
                .find(|(helper, _)| *helper == name)
                .map_or(1, |(_, cost)| *cost)
        })
        .sum();
    if cost > MAX_COST {
        return Err(format!(
            "This shader is too heavy for slower computers: its helper calls cost {cost} and the limit is {MAX_COST}. Use fewer flow (44), voronoi (28), fbm (20), cloud (12), ridged (12) or noise (4) calls, and reuse a result in a `let` instead of calling again."
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
        assert_eq!(names.len(), HELPER_NAMES.len());
        for (name, _) in HELPER_COSTS {
            assert!(names.contains(&name), "{name} is priced but missing");
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
    fn short_bodies_with_let_lines_work_but_loops_and_vars_do_not() {
        let mut custom = CustomShader {
            expression: "let r = length(p);\nlet glow = smoothstep(0.6, 0.0, r);\nreturn mix(c1, palette(r - t * 0.1), glow);".into(),
            ..Default::default()
        };
        assert!(function(&custom).is_ok(), "{:?}", function(&custom));
        for invalid in [
            "var x = 1.0;\nx = x + 1.0;\nreturn c1 * x;",
            "for (var i = 0; i < 3; i++) {}\nreturn c1;",
            "if (p.x > 0.0) { return c1; }\nreturn c2;",
            "let a = c1;",
        ] {
            custom.expression = invalid.into();
            assert!(function(&custom).is_err(), "{invalid}");
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
