//! Handles export of results in JSON format

use crate::output::ResultsFormatter;

#[derive(Default)]
pub struct JsonFormatter {
    /// Column values in the order they were emitted. Stored as a `Vec` rather
    /// than a map so we (a) preserve the user's SELECT column order and
    /// (b) never collapse duplicate column names to a single entry.
    row: Vec<(String, String)>,
}

impl ResultsFormatter for JsonFormatter {
    fn header(&mut self, _: &str, _: usize) -> Option<String> {
        Some("[".to_owned())
    }

    fn row_started(&mut self) -> Option<String> {
        None
    }

    fn format_element(&mut self, name: &str, record: &str, _is_last: bool) -> Option<String> {
        self.row.push((name.to_owned(), record.to_owned()));
        None
    }

    fn row_ended(&mut self) -> Option<String> {
        let mut out = String::from("{");
        for (i, (name, value)) in self.row.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&serde_json::to_string(name).unwrap_or_else(|_| "\"\"".to_string()));
            out.push(':');
            out.push_str(&serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string()));
        }
        out.push('}');
        self.row.clear();
        Some(out)
    }

    fn footer(&mut self) -> Option<String> {
        Some("]".to_owned())
    }

    fn row_separator(&self) -> Option<String> {
        Some(",".to_owned())
    }
}

#[cfg(test)]
mod test {
    use crate::output::ResultsFormatter;
    use crate::output::json::JsonFormatter;
    use crate::output::test::write_test_items;

    #[test]
    fn test() {
        let result = write_test_items(&mut JsonFormatter::default());
        // Columns must appear in the order they were emitted (foo, bar),
        // not alphabetised by a BTreeMap.
        assert_eq!(
            r#"[{"foo":"foo_value","bar":"BAR value"},{"foo":"123","bar":""}]"#,
            result
        );
    }

    #[test]
    fn duplicate_column_names_are_preserved() {
        // If a query emits the same column name twice (e.g. selecting a
        // field and a function of that field with the same display string),
        // we must not silently collapse the two values to one.
        let mut f = JsonFormatter::default();
        let mut out = String::new();
        if let Some(s) = f.header("", 2) { out.push_str(&s); }
        if let Some(s) = f.row_started() { out.push_str(&s); }
        if let Some(s) = f.format_element("name", "first", false) { out.push_str(&s); }
        if let Some(s) = f.format_element("name", "second", true) { out.push_str(&s); }
        if let Some(s) = f.row_ended() { out.push_str(&s); }
        if let Some(s) = f.footer() { out.push_str(&s); }
        // Both values must be visible in the output (whatever shape we
        // choose for duplicates); the old code dropped "first".
        assert!(out.contains("first"), "first value should be preserved, got: {}", out);
        assert!(out.contains("second"), "second value should be preserved, got: {}", out);
    }
}
