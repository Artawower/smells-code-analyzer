use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref LINE_COMMENTS: Regex = Regex::new(r"//([^\r\n]*)").unwrap();
    static ref BLOCK_COMMENTS: Regex = Regex::new(r"/\*[\s\S]*?\*/").unwrap();
    static ref CONSOLE_LOGS: Regex = Regex::new(r"console\.log\([^)]*\);\s*").unwrap();
    static ref CYRILLIC: Regex =
        Regex::new(r"[аАбБвВгГдДеЕёЁжЖзЗиИйЙкКлЛмМнНоОпПрРсСтТуУфФхХцЦчЧшШщЩъЪыЫьЬэЭюЮяЯ]+")
            .unwrap();
}

pub fn sanitize_source(input: &str) -> String {
    let without_line = LINE_COMMENTS.replace_all(input, "");
    let without_block = BLOCK_COMMENTS.replace_all(&without_line, "");
    let without_console = CONSOLE_LOGS.replace_all(&without_block, "");
    let ascii_only: String = without_console
        .chars()
        .map(|c| if c.is_ascii() { c } else { ' ' })
        .collect();
    CYRILLIC.replace_all(&ascii_only, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_comments_and_non_ascii() {
        let source = r#"
        // comment
        /*
           block comment
        */
        const текст = "значение";
        console.log("trace");
        "#;

        let sanitized = sanitize_source(source);
        assert!(!sanitized.contains("comment"));
        assert!(!sanitized.contains("console.log"));
        assert!(!sanitized.contains("текст"));
    }
}
