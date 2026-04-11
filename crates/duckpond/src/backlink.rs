use std::path::{Path, PathBuf};

/// A parsed `@spec` backlink found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBacklink {
    /// File containing the backlink.
    pub file: PathBuf,
    /// 1-based line number.
    pub line: usize,
    /// Capability path (e.g. `auth/google`).
    pub cap_path: String,
    /// Requirement name (e.g. `Email-password login`).
    pub requirement: String,
    /// Scenario name (e.g. `Valid credentials`).
    pub scenario: String,
}

/// Comment markers that can precede `@spec`.
///
/// Ordered longest-first so `///` matches before `//`.
const COMMENT_MARKERS: &[&str] = &[
    "///", "//",   // Rust, JS, TS, Go, C, C++, Java, etc.
    "#",           // Python, Ruby, Shell, TOML, YAML
    ";;",          // Lisp, Clojure, Scheme
    "--",          // Lua, SQL, Haskell
    "%",           // Erlang, LaTeX
];

/// Try to parse a `@spec` backlink from a single source line.
///
/// The line must have the form:
///   <comment-marker> @spec <cap-path> <Requirement Name>: <Scenario Name>
///
/// Returns `None` if the line doesn't contain a backlink.
pub fn parse_backlink_line(line: &str) -> Option<BacklinkRef> {
    let trimmed = line.trim();

    // Find and strip the comment marker.
    let after_marker = COMMENT_MARKERS
        .iter()
        .find_map(|marker| trimmed.strip_prefix(marker))?;

    // Must have whitespace then @spec.
    let after_ws = after_marker.strip_prefix(' ')
        .or_else(|| after_marker.strip_prefix('\t'))?;
    let rest = after_ws.trim_start();

    let rest = rest.strip_prefix("@spec")?;

    // Must be followed by whitespace (not @specfoo).
    if rest.is_empty() {
        return None;
    }
    let rest = rest.strip_prefix(' ')
        .or_else(|| rest.strip_prefix('\t'))?;
    let rest = rest.trim();

    if rest.is_empty() {
        return None;
    }

    // First token is the capability path (no whitespace).
    let (cap_path, rest) = match rest.find(' ') {
        Some(pos) => (&rest[..pos], rest[pos..].trim_start()),
        None => return None,
    };

    if rest.is_empty() {
        return None;
    }

    // Everything up to the first colon is the requirement name.
    // Everything after is the scenario name.
    let (requirement, scenario) = match rest.find(':') {
        Some(pos) => {
            let req = rest[..pos].trim();
            let scn = rest[pos + 1..].trim();
            if req.is_empty() || scn.is_empty() {
                return None;
            }
            (req, scn)
        }
        None => return None,
    };

    // Normalize whitespace: collapse runs to single spaces.
    let requirement = collapse_whitespace(requirement);
    let scenario = collapse_whitespace(scenario);

    Some(BacklinkRef {
        cap_path: cap_path.to_string(),
        requirement,
        scenario,
    })
}

/// A parsed backlink reference (without file/line context).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BacklinkRef {
    pub cap_path: String,
    pub requirement: String,
    pub scenario: String,
}

impl BacklinkRef {
    /// Format as `cap_path Requirement: Scenario` for display.
    pub fn display_key(&self) -> String {
        format!("{} {}: {}", self.cap_path, self.requirement, self.scenario)
    }
}

/// Scan a file's content for `@spec` backlinks.
pub fn scan_file(path: &Path, content: &str) -> Vec<SourceBacklink> {
    content
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let bref = parse_backlink_line(line)?;
            Some(SourceBacklink {
                file: path.to_path_buf(),
                line: i + 1,
                cap_path: bref.cap_path,
                requirement: bref.requirement,
                scenario: bref.scenario,
            })
        })
        .collect()
}

fn collapse_whitespace(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                result.push(' ');
            }
            prev_ws = true;
        } else {
            result.push(c);
            prev_ws = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_line_comment() {
        let r = parse_backlink_line("// @spec auth Email-password login: Valid credentials");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth".into(),
                requirement: "Email-password login".into(),
                scenario: "Valid credentials".into(),
            })
        );
    }

    #[test]
    fn rust_doc_comment() {
        let r = parse_backlink_line("/// @spec auth/google OAuth callback: Valid callback");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth/google".into(),
                requirement: "OAuth callback".into(),
                scenario: "Valid callback".into(),
            })
        );
    }

    #[test]
    fn python_comment() {
        let r = parse_backlink_line("# @spec billing Invoice generation: Monthly invoice");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "billing".into(),
                requirement: "Invoice generation".into(),
                scenario: "Monthly invoice".into(),
            })
        );
    }

    #[test]
    fn lua_comment() {
        let r = parse_backlink_line("-- @spec auth Login: Success");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth".into(),
                requirement: "Login".into(),
                scenario: "Success".into(),
            })
        );
    }

    #[test]
    fn lisp_comment() {
        let r = parse_backlink_line(";; @spec auth Login: Success");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth".into(),
                requirement: "Login".into(),
                scenario: "Success".into(),
            })
        );
    }

    #[test]
    fn latex_comment() {
        let r = parse_backlink_line("% @spec auth Login: Success");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth".into(),
                requirement: "Login".into(),
                scenario: "Success".into(),
            })
        );
    }

    #[test]
    fn scenario_with_colons() {
        let r =
            parse_backlink_line("// @spec api/users Create user: Email validation: rejects bad");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "api/users".into(),
                requirement: "Create user".into(),
                scenario: "Email validation: rejects bad".into(),
            })
        );
    }

    #[test]
    fn leading_whitespace() {
        let r = parse_backlink_line("    // @spec auth Login: Success");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth".into(),
                requirement: "Login".into(),
                scenario: "Success".into(),
            })
        );
    }

    #[test]
    fn no_comment_marker() {
        assert_eq!(parse_backlink_line("@spec auth Login: Success"), None);
    }

    #[test]
    fn no_scenario() {
        assert_eq!(
            parse_backlink_line("// @spec auth Login without colon"),
            None
        );
    }

    #[test]
    fn empty_scenario() {
        assert_eq!(parse_backlink_line("// @spec auth Login:"), None);
    }

    #[test]
    fn spec_prefix_not_standalone() {
        assert_eq!(
            parse_backlink_line("// @specification auth Login: Success"),
            None
        );
    }

    #[test]
    fn whitespace_collapse() {
        let r = parse_backlink_line("// @spec auth Email-password  login:  Valid  credentials");
        assert_eq!(
            r,
            Some(BacklinkRef {
                cap_path: "auth".into(),
                requirement: "Email-password login".into(),
                scenario: "Valid credentials".into(),
            })
        );
    }

    #[test]
    fn scan_file_multiple() {
        let content = "fn foo() {}\n// @spec auth Login: Success\nfn bar() {}\n// @spec auth Logout: Explicit\n";
        let results = scan_file(Path::new("test.rs"), content);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line, 2);
        assert_eq!(results[0].scenario, "Success");
        assert_eq!(results[1].line, 4);
        assert_eq!(results[1].scenario, "Explicit");
    }
}
