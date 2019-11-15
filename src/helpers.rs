use std::error::Error;
use std::path::PathBuf;
use regex::Regex;

pub enum Build {
    None,
    Num(String),
    Regex(String, bool),
}

#[derive(Debug, Clone)]
pub struct Options {
    pub matcher: Matcher,
    pub neg: bool,
}

pub enum Operation {
    Subs([String; 3]),
    Write(PathBuf),
    Delete,
    Print,
    Skip,
    PrintLineNumber,
    Quit,
    InsertBefore(String),
    InsertAfter(String),
}

#[derive(Debug, Clone)]
pub enum Matcher {
    Range(usize, usize),
    Single(usize),
    None,
}

pub fn append_or_create(file_name: PathBuf, new_line: &str) -> Result<(), Box<dyn Error>> {
    let mut file_content = std::fs::read_to_string(&file_name).unwrap_or_default();
    file_content.push_str(new_line);
    std::fs::write(file_name, file_content)?;
    Ok(())
}

pub fn get_regex_position(re: Regex, lines: &[String]) -> usize {
    let mut pos: usize = 0;
    for (i, line) in lines.iter().enumerate() {
        if re.is_match(line) {
            pos = i + 1;
            break;
        }
    }
    pos
}
