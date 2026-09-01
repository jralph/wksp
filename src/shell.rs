//! Shell integration snippets for `wksp go`.
//!
//! `wksp` is a plain subprocess and cannot change its parent shell's working
//! directory on its own. `wksp go` therefore prints the resolved workspace
//! path to stdout (and nothing else) on success, and this module generates a
//! small shell function that wraps the real binary, captures that path, and
//! performs the `cd` in the calling shell. This mirrors the pattern used by
//! tools like `zoxide` and `direnv`.

/// Supported shells for `wksp init <shell>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            _ => None,
        }
    }
}

/// Render the shell init script for the given shell. Intended to be eval'd,
/// e.g. `eval "$(wksp init zsh)"` in `~/.zshrc`.
pub fn render_init_script(shell: Shell) -> String {
    match shell {
        Shell::Bash | Shell::Zsh => bash_like_script(),
        Shell::Fish => fish_script(),
    }
}

fn bash_like_script() -> String {
    r#"# wksp shell integration
# Add to your shell rc file: eval "$(wksp init bash)"  (or zsh)
wksp() {
    if [ "$1" = "go" ]; then
        local dest
        dest="$(command wksp "$@")"
        local status=$?
        if [ $status -ne 0 ]; then
            return $status
        fi
        if [ -n "$dest" ] && [ -d "$dest" ]; then
            cd "$dest" || return 1
        fi
        return 0
    fi
    command wksp "$@"
}
"#
    .to_string()
}

fn fish_script() -> String {
    r#"# wksp shell integration
# Add to your shell config: wksp init fish | source
function wksp
    if test "$argv[1]" = "go"
        set -l dest (command wksp $argv)
        set -l status_code $status
        if test $status_code -ne 0
            return $status_code
        end
        if test -n "$dest"; and test -d "$dest"
            cd "$dest"
        end
        return 0
    end
    command wksp $argv
end
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_shells() {
        assert_eq!(Shell::parse("zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::parse("BASH"), Some(Shell::Bash));
        assert_eq!(Shell::parse("fish"), Some(Shell::Fish));
        assert_eq!(Shell::parse("powershell"), None);
    }

    #[test]
    fn renders_non_empty_scripts() {
        assert!(render_init_script(Shell::Zsh).contains("wksp()"));
        assert!(render_init_script(Shell::Bash).contains("wksp()"));
        assert!(render_init_script(Shell::Fish).contains("function wksp"));
    }
}
