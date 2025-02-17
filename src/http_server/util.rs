use regex::Regex;

pub fn glob_to_regex(glob: &str) -> String {
    let regex_pattern = glob
        .replace('.', r"\.") // Escape `.`
        .replace("**", ".+") // Allow ** to match anything
        .replace("/*/", r"/[^/]+/") // Ensure `*` only matches segment
        .replace('*', "[^/]+"); // Ensure other `*` work too

    format!(r"^{}$", regex_pattern) // Anchor start and end
}

pub fn normalise_path(path: &str) -> String {
    let norm_path = if path.ends_with("/") {
        path.to_string()
    } else {
        format!("{}/", path)
    };
    norm_path
}

pub fn extract_nth_segment_from_url(url_path: &str, n: usize) -> Option<String> {
    let pattern = format!(r"^(?:[^/]*/){{{}}}(\w+)", n); // Replace {N} dynamically
    let regex = Regex::new(&pattern).unwrap();

    regex.captures(url_path).map(|cap| cap[1].to_string())
}
