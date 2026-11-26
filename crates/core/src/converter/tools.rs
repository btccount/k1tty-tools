use super::*;

#[derive(Clone, Debug)]
pub(super) enum CanonicalTool {
    Read {
        path: String,
        start_line: Option<i64>,
        end_line: Option<i64>,
    },
    Edit {
        path: String,
        old_string: String,
        new_string: String,
        diff: Option<String>,
        content: Option<String>,
    },
    Write {
        path: String,
        content: String,
    },
    Delete {
        path: String,
    },
    Shell {
        command: String,
        description: Option<String>,
    },
    ListDir {
        path: String,
        pattern: Option<String>,
    },
    Grep {
        pattern: String,
        path: Option<String>,
    },
    WebSearch {
        query: String,
    },
    Generic {
        name: String,
        args: Value,
    },
}

#[derive(Clone, Debug)]
pub(super) struct ToolInvocation {
    pub(super) canonical: CanonicalTool,
    pub(super) call_text: String,
    pub(super) result_text: String,
    pub(super) is_error: bool,
    pub(super) ts_call: f64,
    pub(super) ts_result: f64,
    pub(super) standalone_output: bool,
}

pub(super) fn clean_tool_name(raw: &str) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("Команда: ")
        .or_else(|| trimmed.strip_prefix("Инструмент: "))
        .or_else(|| trimmed.strip_prefix("Вывод инструмента: "))
        .or_else(|| trimmed.strip_prefix("Tool: "))
        .unwrap_or(trimmed)
        .trim();
    if stripped.is_empty() {
        "tool".to_owned()
    } else {
        stripped.to_owned()
    }
}

pub(super) fn is_tool_result_entry(e: &Value) -> bool {
    if s(e, "kind") != "tool" {
        return false;
    }
    let subtype = s(e, "subtype");
    if subtype == "tool_result" {
        return true;
    }
    if subtype == "tool" {
        let tool = s(e, "tool");
        let label = s(e, "label");
        return tool == "вывод"
            || label == "вывод"
            || label.starts_with("Вывод")
            || tool.starts_with("toolu_");
    }
    false
}

fn pick_str(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(val) = obj.get(*key) {
            match val {
                Value::String(s) if !s.trim().is_empty() => return Some(s.clone()),
                _ => {}
            }
        }
    }
    None
}

fn pick_str_allow_empty(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(Value::String(s)) = obj.get(*key) {
            return Some(s.clone());
        }
    }
    None
}

fn pick_i64(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<i64> {
    for key in keys {
        if let Some(v) = obj.get(*key) {
            if let Some(n) = v.as_i64() {
                return Some(n);
            }
            if let Some(s) = v.as_str() {
                if let Ok(n) = s.trim().parse::<i64>() {
                    return Some(n);
                }
            }
        }
    }
    None
}

pub(super) fn classify_tool(
    raw_name: &str,
    call_text: &str,
    standalone_output: bool,
) -> CanonicalTool {
    let trimmed = call_text.trim();
    let parsed_val = serde_json::from_str::<Value>(trimmed).ok();
    let obj = parsed_val.as_ref().and_then(Value::as_object);
    let low = raw_name.to_ascii_lowercase();

    if standalone_output {
        if matches!(
            low.as_str(),
            "bash"
                | "shell"
                | "shell_command"
                | "exec"
                | "exec_command"
                | "run_command"
                | "terminal"
        ) && !trimmed.is_empty()
        {
            if let Some(map) = obj {
                if let Some(cmd) = pick_str(map, &["command", "CommandLine", "cmd"]) {
                    return CanonicalTool::Shell {
                        command: cmd,
                        description: pick_str(map, &["description", "toolSummary"]),
                    };
                }
            }
            return CanonicalTool::Shell {
                command: trimmed.to_owned(),
                description: None,
            };
        }
        return CanonicalTool::Generic {
            name: raw_name.to_owned(),
            args: json!({}),
        };
    }

    if let Some(map) = obj {
        let path = pick_str(
            map,
            &[
                "path",
                "file_path",
                "Path",
                "TargetFile",
                "DirectoryPath",
                "SearchPath",
            ],
        );
        let command = pick_str(map, &["command", "CommandLine", "cmd"]);
        let description = pick_str(map, &["description", "toolSummary"]);
        let old_string =
            pick_str_allow_empty(map, &["old_string", "oldString", "Target", "TargetContent"]);
        let new_string = pick_str_allow_empty(
            map,
            &[
                "new_string",
                "newString",
                "Replacement",
                "ReplacementContent",
            ],
        );
        let content = pick_str_allow_empty(map, &["content", "CodeContent"]);
        let diff = pick_str(map, &["diff", "patch"]);
        let pattern = pick_str(map, &["pattern", "Query", "query"]);
        let start_line = pick_i64(map, &["start_line", "StartLine", "offset"]);
        let end_line = pick_i64(map, &["end_line", "EndLine", "limit"]);

        match low.as_str() {
            "read" | "read_file" | "view_file" | "view_file_outline" => {
                if let Some(path) = path {
                    return CanonicalTool::Read {
                        path,
                        start_line,
                        end_line,
                    };
                }
            }
            "edit"
            | "edit_file"
            | "replace_file_content"
            | "multi_replace_file_content"
            | "str_replace_editor" => {
                if let Some(path) = path {
                    let has_old = old_string.as_deref().is_some_and(|s| !s.is_empty());
                    if has_old || diff.is_some() {
                        return CanonicalTool::Edit {
                            path,
                            old_string: old_string.unwrap_or_default(),
                            new_string: new_string.or(content.clone()).unwrap_or_default(),
                            diff,
                            content,
                        };
                    }
                    if let Some(cont) = content.or(new_string).filter(|s| !s.is_empty()) {
                        return CanonicalTool::Edit {
                            path,
                            old_string: String::new(),
                            new_string: cont.clone(),
                            diff: None,
                            content: Some(cont),
                        };
                    }
                }
            }
            "write" | "write_file" | "write_to_file" | "create_file" => {
                if let Some(path) = path {
                    return CanonicalTool::Write {
                        path,
                        content: content.unwrap_or_default(),
                    };
                }
            }
            "delete_file" | "remove_file" => {
                if let Some(path) = path {
                    return CanonicalTool::Delete { path };
                }
            }
            "bash" | "shell" | "shell_command" | "exec" | "exec_command" | "run_command"
            | "terminal" => {
                if let Some(cmd) = command {
                    return CanonicalTool::Shell {
                        command: cmd,
                        description,
                    };
                }
            }
            "list_dir" | "glob" | "find_by_name" | "list_files" => {
                let dir_path = path.unwrap_or_else(|| ".".to_owned());
                return CanonicalTool::ListDir {
                    path: dir_path,
                    pattern,
                };
            }
            "grep" | "grep_search" | "ripgrep" | "search_files" => {
                if let Some(pat) = pattern {
                    return CanonicalTool::Grep { pattern: pat, path };
                }
            }
            "web_search" | "websearch" | "search_web" => {
                if let Some(q) = pattern.or_else(|| pick_str(map, &["q", "search_query"])) {
                    return CanonicalTool::WebSearch { query: q };
                }
            }
            _ => {}
        }

        return CanonicalTool::Generic {
            name: raw_name.to_owned(),
            args: Value::Object(map.clone()),
        };
    }

    match low.as_str() {
        "bash" | "shell" | "shell_command" | "exec" | "exec_command" | "run_command"
        | "terminal"
            if !trimmed.is_empty() =>
        {
            CanonicalTool::Shell {
                command: trimmed.to_owned(),
                description: None,
            }
        }
        "web_search" | "websearch" | "search_web" if !trimmed.is_empty() => {
            CanonicalTool::WebSearch {
                query: trimmed.to_owned(),
            }
        }
        _ => CanonicalTool::Generic {
            name: raw_name.to_owned(),
            args: if trimmed.is_empty() {
                json!({})
            } else {
                json!({"summary": shorten(trimmed, 300)})
            },
        },
    }
}

pub(super) fn consume_tool_pair(
    entries: &[Value],
    times: &[f64],
    idx: &mut usize,
) -> ToolInvocation {
    let first_idx = *idx;
    let first = &entries[first_idx];
    let ts_call = times[first_idx];
    *idx += 1;

    let raw_name = nonempty(s(first, "tool"))
        .or_else(|| nonempty(s(first, "label")))
        .map(|raw| clean_tool_name(&raw))
        .unwrap_or_else(|| "tool".to_owned());

    let subtype = s(first, "subtype");
    let first_text = s(first, "text");
    let first_err = s(first, "status").eq_ignore_ascii_case("error");

    if matches!(subtype.as_str(), "tool_output" | "generic") || is_tool_result_entry(first) {
        let canonical = classify_tool(&raw_name, &first_text, true);
        return ToolInvocation {
            canonical,
            call_text: String::new(),
            result_text: first_text,
            is_error: first_err,
            ts_call,
            ts_result: ts_call,
            standalone_output: true,
        };
    }

    let (result_text, is_error, ts_result) =
        if *idx < entries.len() && is_tool_result_entry(&entries[*idx]) {
            let next = &entries[*idx];
            let next_ts = times[*idx];
            *idx += 1;
            (
                s(next, "text"),
                s(next, "status").eq_ignore_ascii_case("error"),
                next_ts,
            )
        } else {
            (String::new(), first_err, ts_call)
        };

    let canonical = classify_tool(&raw_name, &first_text, false);
    ToolInvocation {
        canonical,
        call_text: first_text,
        result_text,
        is_error,
        ts_call,
        ts_result,
        standalone_output: false,
    }
}

pub(super) fn build_unified_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = if old.is_empty() {
        Vec::new()
    } else {
        old.lines().collect()
    };
    let new_lines: Vec<&str> = if new.is_empty() {
        Vec::new()
    } else {
        new.lines().collect()
    };
    let old_start = if old_lines.is_empty() { 0 } else { 1 };
    let new_start = if new_lines.is_empty() { 0 } else { 1 };
    let mut out = format!(
        "@@ -{},{} +{},{} @@\n",
        old_start,
        old_lines.len(),
        new_start,
        new_lines.len()
    );
    for line in &old_lines {
        out.push('-');
        out.push_str(line);
        out.push('\n');
    }
    for line in &new_lines {
        out.push('+');
        out.push_str(line);
        out.push('\n');
    }
    out
}

pub(super) fn build_apply_patch(
    path: &str,
    old: &str,
    new: &str,
    is_add: bool,
    is_delete: bool,
) -> String {
    let mut out = String::from("*** Begin Patch\n");
    if is_add {
        out.push_str(&format!("*** Add File: {path}\n"));
        for line in new.lines() {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
    } else if is_delete {
        out.push_str(&format!("*** Delete File: {path}\n"));
    } else {
        out.push_str(&format!("*** Update File: {path}\n@@\n"));
        for line in old.lines() {
            out.push('-');
            out.push_str(line);
            out.push('\n');
        }
        for line in new.lines() {
            out.push('+');
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("*** End Patch");
    out
}

pub(super) fn count_diff_stats(diff: &str) -> (usize, usize) {
    let mut adds = 0usize;
    let mut dels = 0usize;
    for line in diff.lines() {
        if line.starts_with('+') && !line.starts_with("+++") {
            adds += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            dels += 1;
        }
    }
    (adds, dels)
}

pub(super) fn basename(path: &str) -> String {
    path.rsplit(['/', '\\'])
        .find(|s| !s.is_empty())
        .unwrap_or(path)
        .to_owned()
}

pub(super) fn shell_quote(arg: &str) -> String {
    if arg.is_empty() {
        return "\"\"".to_owned();
    }
    if arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-' | ':'))
    {
        arg.to_owned()
    } else {
        format!("\"{}\"", arg.replace('"', "\\\""))
    }
}
