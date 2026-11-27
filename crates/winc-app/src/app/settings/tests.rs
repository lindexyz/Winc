//! 设置的测试辅助：按语言取格式化页上显示的行。

use super::*;

impl SettingsState {
    /// 返回语言 `id` 的格式化命令设置项；测试用（只有 Unix 上的格式化测试用到）。
    #[cfg(unix)]
    pub fn format_row(&self, id: &'static str) -> Option<&FormatRow> {
        self.tool_row(Setting::FormatCommand(id))
    }
}

#[test]
fn excludes_are_written_and_read_back_as_comma_separated_text() {
    let patterns = ["dist/".to_owned(), "*.log".to_owned()];
    assert_eq!(excludes_text(&patterns).as_deref(), Some("dist/, *.log"));
    assert_eq!(excludes_text(&[]), None);

    assert_eq!(
        parse_excludes("dist/， *.log\n  !keep.log ,, "),
        ["dist/", "*.log", "!keep.log"]
    );
    assert!(parse_excludes("  \n ").is_empty());
}
