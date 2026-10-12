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

fn combine(source: &str) -> String {
    let full = include_str!("shader_background.wgsl").replace(
        "let color = mix(mix(u.first.rgb, u.second.rgb, field), u.third.rgb, (1.0 - field) * 0.3);",
        "let color = clamp(custom_pattern(p, t, u.knobs, u.first.rgb, u.second.rgb, u.third.rgb), vec3<f32>(0.0), vec3<f32>(1.0));\n    glow = 1.0;"
    );
    format!("{full}\n{source}")
}

fn validate(source: &str) -> Result<(), String> {
    let module =
        naga::front::wgsl::parse_str(source).map_err(|error| error.emit_to_string(source))?;
    if module.functions.len() != 1
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
    let function = module.functions.iter().next().unwrap().1;
    if function.expressions.len() > 128
        || function.body.iter().any(|statement| {
            !matches!(
                statement,
                naga::Statement::Emit(_) | naga::Statement::Return { .. }
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
    }
}
