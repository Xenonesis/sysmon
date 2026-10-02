// ─── Path Parsing ────────────────────────────────────────────

pub fn expand_env_vars(path: &str) -> String {
    let mut result = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut var_name = String::new();
            let mut found_end = false;
            while let Some(&nc) = chars.peek() {
                if nc == '%' {
                    chars.next();
                    found_end = true;
                    break;
                }
                if let Some(ch) = chars.next() {
                    var_name.push(ch);
                }
            }
            if found_end {
                if let Ok(val) = std::env::var(&var_name) {
                    result.push_str(&val);
                } else {
                    result.push('%');
                    result.push_str(&var_name);
                    result.push('%');
                }
            } else {
                result.push('%');
                result.push_str(&var_name);
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn parse_exe_from_command(cmd: &str) -> Option<String> {
    let t = cmd.trim();
    if t.is_empty() {
        return None;
    }

    // 1. Quoted string: "C:\path\app.exe" ...
    if let Some(stripped) = t.strip_prefix('"')
        && let Some(end) = stripped.find('"')
    {
        let p = &stripped[..end];
        if !p.is_empty() {
            return Some(p.to_string());
        }
    }

    // 2. rundll32 handling (case-insensitive, char-boundary safe)
    let lower = t.to_ascii_lowercase();
    if lower.starts_with("rundll32.exe") {
        let after = t.get(12..).unwrap_or("").trim().trim_start_matches('"');
        if let Some(comma) = after.find(',') {
            let dll = after[..comma].trim().trim_end_matches('"');
            if !dll.is_empty() {
                return Some(dll.to_string());
            }
        }
    } else if lower.starts_with("rundll32") {
        let after = t.get(8..).unwrap_or("").trim().trim_start_matches('"');
        if let Some(comma) = after.find(',') {
            let dll = after[..comma].trim().trim_end_matches('"');
            if !dll.is_empty() {
                return Some(dll.to_string());
            }
        }
    }

    // 3. Search for known executable extensions safely using char_indices
    for ext in &[".exe", ".cmd", ".bat", ".vbs", ".ps1"] {
        for (idx, _) in t.char_indices() {
            if let Some(slice) = t.get(idx..)
                && slice.to_ascii_lowercase().starts_with(ext)
            {
                let end_pos = idx + ext.len();
                let is_end = match t.get(end_pos..) {
                    None | Some("") => true,
                    Some(rest) => {
                        rest.starts_with(' ') || rest.starts_with('"') || rest.starts_with('/') || rest.starts_with(',')
                    }
                };
                if is_end && let Some(matched) = t.get(..end_pos) {
                    return Some(matched.trim_matches('"').to_string());
                }
            }
        }
    }

    // 4. Default: first whitespace-separated token
    t.split_whitespace().next().map(|s| s.trim_matches('"').to_string())
}
