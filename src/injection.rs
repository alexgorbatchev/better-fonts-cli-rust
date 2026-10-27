use crate::{Result, invalid};
fn marker(id: &str) -> Result<String> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid(
            "app ID must contain only ASCII letters, digits, hyphen or underscore",
        ));
    }
    Ok(format!("fonted-{id}-patch"))
}
fn range(text: &str, id: &str) -> Result<Option<std::ops::Range<usize>>> {
    let marker = marker(id)?;
    let begin = format!("\n;/*{marker}*/(function(){{var __better_fonts_font=");
    let Some(start) = text.find(&begin) else {
        return existing(text, &marker).map(|value| value.map(|(range, _)| range));
    };
    let end_marker = format!("/*{marker}-end*/\n");
    let end = text[start..].find(&end_marker).ok_or_else(|| {
        invalid("incomplete better-fonts injection; refusing destructive removal")
    })? + start
        + end_marker.len();
    Ok(Some(start..end))
}
pub fn strip(text: &str, id: &str) -> Result<String> {
    let mut result = text.to_owned();
    while let Some(range) = range(&result, id)? {
        result.replace_range(range, "");
    }
    Ok(result)
}
pub fn detect(text: &str, id: &str) -> Result<Option<String>> {
    if let Some((_, font)) = existing(text, &marker(id)?)? {
        return Ok(Some(font));
    }
    let Some(r) = range(text, id)? else {
        return Ok(None);
    };
    let start = text[r.start..r.end]
        .find("var __better_fonts_font=")
        .unwrap()
        + r.start
        + "var __better_fonts_font=".len();
    let mut stream = serde_json::Deserializer::from_str(&text[start..r.end]).into_iter::<String>();
    Ok(Some(
        stream
            .next()
            .ok_or_else(|| invalid("missing injected font"))??,
    ))
}
// Read the actual persisted single-quoted JavaScript data emitted by the Go tool.
// Parse its quoted value before checking the exact suffix, so font contents cannot
// turn a marker-looking substring into a destructive removal boundary.
fn existing(text: &str, marker: &str) -> Result<Option<(std::ops::Range<usize>, String)>> {
    let begin = format!(
        "\n;/*{marker}*/(function(){{document.addEventListener(\"DOMContentLoaded\",function(){{var s=document.createElement(\"style\");s.textContent='"
    );
    let Some(start) = text.find(&begin) else {
        return Ok(None);
    };
    let value_start = start + begin.len();
    let mut characters = text[value_start..].char_indices();
    let mut css = String::new();
    while let Some((offset, character)) = characters.next() {
        match character {
            '\\' => match characters.next().map(|(_, c)| c) {
                Some('\'') => css.push('\''),
                Some('\\') => css.push('\\'),
                Some('n') => css.push('\n'),
                _ => return Err(invalid("invalid existing font injection escape")),
            },
            '\'' => {
                let end = value_start + offset;
                let suffix = "';document.head.appendChild(s)})})();\n";
                if !text[end..].starts_with(suffix) {
                    return Err(invalid("incomplete existing font injection"));
                }
                let font = css
                    .split_once(" { font-family: \"")
                    .and_then(|(_, rest)| rest.split_once("\", monospace !important; }"))
                    .map(|(font, _)| font.to_owned())
                    .ok_or_else(|| invalid("existing font injection has invalid CSS"))?;
                return Ok(Some((start..end + suffix.len(), font)));
            }
            character => css.push(character),
        }
    }
    Err(invalid("unterminated existing font injection"))
}
fn css_string(font: &str) -> String {
    let mut result = String::from("\"");
    for c in font.chars() {
        match c {
            '"' | '\\' => {
                result.push('\\');
                result.push(c);
            }
            c if c.is_control() => result.push_str(&format!("\\{:x} ", c as u32)),
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
pub fn inject(text: &str, id: &str, font: &str) -> Result<String> {
    if font.trim().is_empty() || font.contains('\0') {
        return Err(invalid("font must be nonempty and contain no NUL"));
    }
    let marker = marker(id)?;
    let clean = strip(text, id)?;
    let css = format!(
        "*:not([data-cds*=\"Icon\"]):not([data-cds*=\"Icon\"] *):not([class*=\"icon\" i]):not([class*=\"Icon\"]):not([data-icon]):not(i):not(svg):not(svg *) {{ font-family: {}, monospace !important; }} [data-cds*=\"Icon\"], [data-cds*=\"Icon\"] * {{ font-family: var(--font-anthropicons, Anthropicons-Variable), sans-serif !important; }}",
        css_string(font)
    );
    Ok(format!(
        "{clean}\n;/*{marker}*/(function(){{var __better_fonts_font={};function apply(){{var s=document.createElement('style');s.textContent={};document.head.appendChild(s);}}if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',apply);else apply();}})();/*{marker}-end*/\n",
        serde_json::to_string(font)?,
        serde_json::to_string(&css)?
    ))
}
