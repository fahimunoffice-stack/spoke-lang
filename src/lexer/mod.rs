pub mod token;
pub use token::{Token, SpannedToken};
use crate::error::SpokeError;

pub struct Lexer {
    indent_stack: Vec<usize>,
}

impl Lexer {
    pub fn new(_source: &str) -> Self {
        Lexer { indent_stack: vec![0] }
    }

    pub fn tokenize_str(&mut self, source: &str) -> Result<Vec<SpannedToken>, SpokeError> {
        let mut tokens: Vec<SpannedToken> = Vec::new();
        let lines: Vec<&str> = source.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            let line_num = line_idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

            let indent = line.len() - line.trim_start().len();
            let current_indent = *self.indent_stack.last().unwrap_or(&0);

            if indent > current_indent {
                self.indent_stack.push(indent);
                tokens.push(SpannedToken::new(Token::Indent, line_num, 1));
            } else {
                while *self.indent_stack.last().unwrap_or(&0) > indent {
                    self.indent_stack.pop();
                    tokens.push(SpannedToken::new(Token::Dedent, line_num, 1));
                }
            }

            let line_tokens = self.lex_line(trimmed, line_num)?;
            tokens.extend(line_tokens);
        }

        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            tokens.push(SpannedToken::new(Token::Dedent, 0, 1));
        }
        tokens.push(SpannedToken::new(Token::Eof, 0, 1));
        Ok(tokens)
    }

    fn lex_line(&self, line: &str, line_num: usize) -> Result<Vec<SpannedToken>, SpokeError> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        let mut col = 1usize;

        while i < chars.len() {
            let c = chars[i];
            if c == ' '  { i += 1; col += 1; continue; }
            if c == '#'  { break; }

            // String — only double quotes (single quote = possessive)
            if c == '"' {
                let (s, consumed) = self.read_string_from(&chars, i, line_num, col)?;
                tokens.push(SpannedToken::new(Token::StringLit(s), line_num, col));
                i += consumed; col += consumed; continue;
            }

            // Number
            if c.is_ascii_digit() {
                let (n, consumed) = self.read_number_from(&chars, i);
                tokens.push(SpannedToken::new(Token::NumberLit(n), line_num, col));
                i += consumed; col += consumed; continue;
            }

            // Arrow → (unicode single char)
            if c == '\u{2192}' {
                tokens.push(SpannedToken::new(Token::Arrow, line_num, col));
                i += 1; col += 1; continue;
            }
            // Arrow ->
            if c == '-' && chars.get(i+1) == Some(&'>') {
                tokens.push(SpannedToken::new(Token::Arrow, line_num, col));
                i += 2; col += 2; continue;
            }

            if c == ':' { tokens.push(SpannedToken::new(Token::Colon, line_num, col)); i+=1; col+=1; continue; }
            if c == ',' { tokens.push(SpannedToken::new(Token::Comma, line_num, col)); i+=1; col+=1; continue; }

            // possessive 's
            if c == '\'' {
                if chars.get(i+1) == Some(&'s') { i += 2; col += 2; }
                else { i += 1; col += 1; }
                continue;
            }

            // Path /something
            if c == '/' {
                let (path, consumed) = self.read_path_from(&chars, i);
                tokens.push(SpannedToken::new(Token::Identifier(path), line_num, col));
                i += consumed; col += consumed; continue;
            }

            // Word
            if c.is_alphabetic() || c == '_' {
                let (word, consumed) = self.read_word_from(&chars, i);
                let tok = Token::keyword(&word).unwrap_or(Token::Identifier(word));
                tokens.push(SpannedToken::new(tok, line_num, col));
                i += consumed; col += consumed; continue;
            }

            i += 1; col += 1;
        }
        Ok(tokens)
    }

    fn read_string_from(&self, chars: &[char], start: usize, line: usize, col: usize)
        -> Result<(String, usize), SpokeError>
    {
        let quote = chars[start];
        let mut s = String::new();
        let mut i = start + 1;
        loop {
            match chars.get(i) {
                None => return Err(SpokeError::UnterminatedString { line, col }),
                Some(&c) if c == quote => { i += 1; break; }
                Some(&'\\') => {
                    i += 1;
                    match chars.get(i) {
                        Some(&'n') => { s.push('\n'); i += 1; }
                        Some(&c)   => { s.push(c);   i += 1; }
                        None       => break,
                    }
                }
                Some(&c) => { s.push(c); i += 1; }
            }
        }
        Ok((s, i - start))
    }

    fn read_number_from(&self, chars: &[char], start: usize) -> (f64, usize) {
        let mut s = String::new();
        let mut i = start;
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            s.push(chars[i]); i += 1;
        }
        (s.parse().unwrap_or(0.0), i - start)
    }

    fn read_word_from(&self, chars: &[char], start: usize) -> (String, usize) {
        let mut word = String::new();
        let mut i = start;
        while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '-') {
            word.push(chars[i]); i += 1;
        }
        (word, i - start)
    }

    fn read_path_from(&self, chars: &[char], start: usize) -> (String, usize) {
        let mut path = String::new();
        let mut i = start;
        while i < chars.len() && !chars[i].is_whitespace() {
            path.push(chars[i]); i += 1;
        }
        (path, i - start)
    }
}

// Public API — matches what main.rs expects
pub fn tokenize(source: &str) -> Result<Vec<SpannedToken>, SpokeError> {
    Lexer::new(source).tokenize_str(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<Token> {
        tokenize(src).unwrap().into_iter()
            .map(|t| t.token)
            .filter(|t| !matches!(t, Token::Eof))
            .collect()
    }

    #[test]
    fn test_keywords() {
        let t = lex("app users can");
        assert!(t.contains(&Token::App));
        assert!(t.contains(&Token::Users));
        assert!(t.contains(&Token::Can));
    }

    #[test]
    fn test_string() {
        let t = lex(r#"app "Todo""#);
        assert!(t.contains(&Token::StringLit("Todo".into())));
    }

    #[test]
    fn test_indent() {
        let src = "app \"x\"\n  auth:";
        let t = lex(src);
        assert!(t.contains(&Token::Indent));
    }

    #[test]
    fn test_comment() {
        let t = lex("# comment");
        assert!(t.is_empty());
    }
}
