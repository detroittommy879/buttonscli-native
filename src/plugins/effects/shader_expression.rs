//! A color expression only: no statements, loops, helper functions or resources.
use crate::theme::CustomShader;
use eframe::wgpu::naga;

pub(crate) fn function(custom: &CustomShader) -> Result<String, String> {
    if custom.expression.len() > 2048 || custom.variables.len() > 4 {
        return Err("Use at most 2048 source bytes and four variables.".into());
    }
    if custom
        .expression
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|word| matches!(word, "array" | "binding_array"))
    {
        return Err("Array types are disabled in color expressions.".into());
    }
    let mut names = std::collections::HashSet::new();
    let mut source = String::from("fn custom_pattern(p: vec2<f32>, t: f32, knobs: vec4<f32>, c1: vec3<f32>, c2: vec3<f32>, c3: vec3<f32>) -> vec3<f32> {\n");
    for (index, variable) in custom.variables.iter().enumerate() {
        let name = &variable.name;
        if name.is_empty()
            || name.len() > 32
            || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
            || !name.as_bytes()[0].is_ascii_alphabetic()
            || name.starts_with("__")
            || ["p", "t", "knobs", "c1", "c2", "c3"].contains(&name.as_str())
            || !names.insert(name)
        {
            return Err("Variable names must be unique letters/numbers/underscores, start with a letter, and avoid p, t, knobs and c1–c3.".into());
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
    source.push_str(&format!("return {};\n}}", custom.expression));
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
    Ok(combine(&function(custom)?))
}

/// Helpers (`hash`, `noise`, `cloud`, `stars`, `palette`, `hsv`) shared by the
/// presets and callable from custom expressions.
pub(crate) const HELPERS: &str = include_str!("shader_helpers.wgsl");
/// The complete preset program.
pub(crate) const SOURCE: &str = concat!(
    include_str!("shader_helpers.wgsl"),
    include_str!("shader_background.wgsl")
);

fn combine(source: &str) -> String {
    let full = SOURCE.replace(
        "let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);",
        "let color = clamp(custom_pattern(p, t, u.knobs, u.first.rgb, u.second.rgb, u.third.rgb), vec3<f32>(0.0), vec3<f32>(1.0));\n    glow = 1.0;\n    vivid = vec3<f32>(-1.0);"
    );
    format!("{full}\n{source}")
}

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
    if function.expressions.len() > 128
        || function.body.iter().any(|statement| {
            !matches!(
                statement,
                naga::Statement::Emit(_)
                    | naga::Statement::Return { .. }
                    // Calls can only reach the fixed, loop-free helpers: the
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
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|error| error.to_string())?;
    // Check the complete fragment too, including name collisions and stage rules,
    // before any user expression reaches wgpu's pipeline creation.
    let full = combine(source);
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
