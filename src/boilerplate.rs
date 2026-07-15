use std::path::Path;

fn relative_prefix(path: &Path) -> String {
    let depth = path.parent().map_or(0, |p| {
        p.components().filter(|c| matches!(c, std::path::Component::Normal(_))).count()
    });
    "../".repeat(depth)
}

pub fn ensure_temml(html: &str, path: &Path, has_math: bool) -> Option<String> {
    if !has_math || html.contains("temml.min.js") {
        return None;
    }
    let pos = html.rfind("</body>")?;
    let prefix = relative_prefix(path);
    let block = format!(
        "  <!-- temml -->\n    <link rel=\"stylesheet\" href=\"{prefix}Temml-Local.css\" />\n    \
         <script src=\"{prefix}temml.min.js\"></script>\n    \
         <script>temml.renderMathInElement(document.body, {{trust: true, wrap: \
         \"tex\"}});</script>\n  "
    );
    let mut out = html.to_owned();
    out.insert_str(pos, &block);
    Some(out)
}

pub fn ensure_highlight_js(html: &str, path: &Path, needs_color: bool) -> Option<String> {
    if !needs_color || html.contains("highlight.js") {
        return None;
    }
    let pos = html.find("</head>")?;
    let prefix = relative_prefix(path);
    let block = format!("  <script src=\"{prefix}highlight.js\"></script>\n  ");
    let mut out = html.to_owned();
    out.insert_str(pos, &block);
    Some(out)
}
