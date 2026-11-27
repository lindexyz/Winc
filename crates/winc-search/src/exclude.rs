//! 用户排除的文件和目录（设置中的 `tree.exclude`）。
//!
//! 规则写法同 `.gitignore`，相对于各个打开的目录；文件树、快速打开和全局搜索共用，三处看到的
//! 文件相同。与目录自己的 `.gitignore` 无关：它只管显示，不影响 Git 状态，也不写回任何文件。

use std::path::{Path, PathBuf};

use ignore::gitignore::{Gitignore, GitignoreBuilder};

/// 一组打开的目录和它们的排除规则。
///
/// 每个目录各有一份匹配器：模式相对于该目录，`/` 开头的模式只从该目录起算，其余在任意一级
/// 生效；`!` 开头的模式重新收录前面排除的项。
#[derive(Debug, Clone, Default)]
pub struct Excludes {
    roots: Vec<(PathBuf, Gitignore)>,
}

impl Excludes {
    /// 按 `patterns` 为 `roots` 中的各个目录建规则。
    ///
    /// 写错的模式不起作用（只有那一条），不报错：排除规则不该让整个目录打不开。
    pub fn new(roots: impl IntoIterator<Item = PathBuf>, patterns: &[String]) -> Self {
        let mut built = Vec::new();
        if patterns.is_empty() {
            return Self { roots: built };
        }
        for root in roots {
            let mut builder = GitignoreBuilder::new(&root);
            let mut any = false;
            for pattern in patterns {
                any |= builder.add_line(None, pattern).is_ok();
            }
            if any
                && let Ok(matcher) = builder.build()
            {
                built.push((root, matcher));
            }
        }
        Self { roots: built }
    }

    /// 返回是否没有规则：没有时各处都不必检查。
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// 返回 `path` 是否被排除；`is_dir` 参与 `dist/` 这类只匹配目录的模式。
    ///
    /// 父目录被排除时其下的项同样排除。不在任何打开的目录下时不算排除。
    pub fn matched(&self, path: &Path, is_dir: bool) -> bool {
        self.roots.iter().any(|(root, matcher)| {
            (path.strip_prefix(root))
                .is_ok_and(|relative| matcher.matched_path_or_any_parents(relative, is_dir).is_ignore())
        })
    }

    /// 同 [`Excludes::matched`]，但 `relative` 已经相对于目录 `root`：快速打开的文件表按相对
    /// 路径收录，不必为每一项拼出完整路径。
    pub fn matched_in(&self, root: &Path, relative: &Path, is_dir: bool) -> bool {
        (self.roots.iter())
            .find(|(dir, _)| dir == root)
            .is_some_and(|(_, matcher)| {
                matcher.matched_path_or_any_parents(relative, is_dir).is_ignore()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn excludes(patterns: &[&str]) -> Excludes {
        let patterns: Vec<String> = patterns.iter().map(|p| (*p).to_owned()).collect();
        Excludes::new([PathBuf::from("/proj")], &patterns)
    }

    fn matched(excludes: &Excludes, path: &str, is_dir: bool) -> bool {
        excludes.matched(Path::new(path), is_dir)
    }

    #[test]
    fn without_patterns_nothing_is_excluded() {
        let excludes = excludes(&[]);
        assert!(excludes.is_empty());
        assert!(!matched(&excludes, "/proj/dist/app.js", false));
    }

    #[test]
    fn a_directory_pattern_excludes_it_and_everything_below() {
        let excludes = excludes(&["dist/"]);
        assert!(matched(&excludes, "/proj/dist", true));
        assert!(matched(&excludes, "/proj/dist/app.js", false));
        assert!(matched(&excludes, "/proj/web/dist", true));
        // 文件树问的是目录本身的种类：同名的文件不受 `dist/` 影响。
        assert!(!matched(&excludes, "/proj/dist", false));
    }

    #[test]
    fn a_pattern_without_a_slash_applies_at_any_level() {
        let excludes = excludes(&["*.log"]);
        assert!(matched(&excludes, "/proj/a.log", false));
        assert!(matched(&excludes, "/proj/logs/deep/a.log", false));
        assert!(!matched(&excludes, "/proj/a.log.txt", false));
    }

    #[test]
    fn a_leading_slash_anchors_the_pattern_to_the_folder() {
        let excludes = excludes(&["/build"]);
        assert!(matched(&excludes, "/proj/build", true));
        assert!(!matched(&excludes, "/proj/web/build", true));
    }

    #[test]
    fn a_negated_pattern_brings_a_name_back() {
        let excludes = excludes(&["*.log", "!keep.log"]);
        assert!(matched(&excludes, "/proj/a.log", false));
        assert!(!matched(&excludes, "/proj/keep.log", false));
    }

    #[test]
    fn paths_outside_the_folder_are_never_excluded() {
        let excludes = excludes(&["*.log"]);
        assert!(!matched(&excludes, "/elsewhere/a.log", false));
    }

    #[test]
    fn a_relative_path_matches_against_its_own_folder() {
        let excludes = excludes(&["dist/", "*.log"]);
        assert!(excludes.matched_in(Path::new("/proj"), Path::new("dist/app.js"), false));
        assert!(excludes.matched_in(Path::new("/proj"), Path::new("a.log"), false));
        assert!(!excludes.matched_in(Path::new("/proj"), Path::new("src/main.rs"), false));
        // 不属于任何打开的目录时没有规则可用。
        assert!(!excludes.matched_in(Path::new("/other"), Path::new("a.log"), false));
    }

    #[test]
    fn a_broken_pattern_is_ignored_while_the_others_apply() {
        let excludes = excludes(&["[", "*.log"]);
        assert!(matched(&excludes, "/proj/a.log", false));
        assert!(!matched(&excludes, "/proj/src/main.rs", false));
    }
}
