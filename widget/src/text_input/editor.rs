use crate::text_input::{Cursor, Value};

pub struct Editor<'a> {
    value: &'a mut Value,
    cursor: &'a mut Cursor,
}

impl<'a> Editor<'a> {
    pub fn new(value: &'a mut Value, cursor: &'a mut Cursor) -> Editor<'a> {
        Editor { value, cursor }
    }

    pub fn contents(&self) -> String {
        self.value.to_string()
    }

    pub fn insert(&mut self, content: impl Into<Value>) {
        self.insert_value(content.into());
    }

    pub fn paste(&mut self, content: Value) {
        self.insert_value(content);
    }

    fn insert_value(&mut self, content: Value) {
        if let Some((left, right)) = self.cursor.selection(self.value) {
            self.cursor.move_left(self.value);
            self.value.remove_many(left, right);
        }

        let cursor = self
            .value
            .insert_many_at(self.cursor.end(self.value), content);
        self.cursor.move_to(cursor);
    }

    pub fn backspace(&mut self) {
        match self.cursor.selection(self.value) {
            Some((start, end)) => {
                self.cursor.move_left(self.value);
                self.value.remove_many(start, end);
            }
            None => {
                let start = self.cursor.start(self.value);

                if start > 0 {
                    self.cursor.move_left(self.value);
                    self.value.remove(start - 1);
                }
            }
        }
    }

    pub fn delete(&mut self) {
        match self.cursor.selection(self.value) {
            Some(_) => {
                self.backspace();
            }
            None => {
                let end = self.cursor.end(self.value);

                if end < self.value.len() {
                    self.value.remove(end);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text_input::cursor::State;

    #[test]
    fn inserting_combining_mark_keeps_cursor_after_combined_grapheme() {
        let mut value = Value::new("");
        let mut cursor = Cursor::default();

        let mut editor = Editor::new(&mut value, &mut cursor);
        editor.insert("e");
        editor.insert("\u{301}");

        assert_eq!(editor.contents(), "e\u{301}");
        assert_eq!(value.len(), 1);
        assert_eq!(cursor.state(&value), State::Index(1));

        let mut editor = Editor::new(&mut value, &mut cursor);
        editor.backspace();

        assert_eq!(editor.contents(), "");
        assert_eq!(value.len(), 0);
        assert_eq!(cursor.state(&value), State::Index(0));
    }

    #[test]
    fn inserting_zwj_emoji_keeps_cursor_after_grapheme() {
        let mut value = Value::new("");
        let mut cursor = Cursor::default();
        let emoji = "👨\u{200d}👩\u{200d}👧\u{200d}👦";

        let mut editor = Editor::new(&mut value, &mut cursor);
        editor.insert(emoji);

        assert_eq!(editor.contents(), emoji);
        assert_eq!(value.len(), 1);
        assert_eq!(cursor.state(&value), State::Index(1));

        let mut editor = Editor::new(&mut value, &mut cursor);
        editor.backspace();

        assert_eq!(editor.contents(), "");
        assert_eq!(value.len(), 0);
        assert_eq!(cursor.state(&value), State::Index(0));
    }

    #[test]
    fn inserting_before_combining_mark_recomposes_next_grapheme() {
        let mut value = Value::new("\u{301}");
        let mut cursor = Cursor::default();

        let mut editor = Editor::new(&mut value, &mut cursor);
        editor.insert("e");

        assert_eq!(editor.contents(), "e\u{301}");
        assert_eq!(value.len(), 1);
        assert_eq!(cursor.state(&value), State::Index(1));
    }
}
