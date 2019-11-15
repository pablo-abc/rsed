mod builders;
mod helpers;
mod options;

use builders::insert::{build_insert, InsertType};
use builders::options::build_options;
use builders::substitution::build_subs;
use builders::write::build_write;
pub use helpers::{append_or_create, get_regex_position, Build, Matcher, Operation, Options};
pub use options::Opt;
use regex::Regex;
use std::path::PathBuf;

fn line_to_edit(re: &Regex, flags: &str, line: &str) -> [String; 2] {
    let nre = Regex::new(r"\d+").unwrap();
    if let Some(from) = nre.captures(flags) {
        if &from[0] == "0" {
            panic!("Number option to 's' command may not be 0");
        }
        let from = from[0].parse::<usize>().unwrap() - 1;
        let occurrences: Vec<_> = re.find_iter(line).collect();
        if let Some(matched) = occurrences.get(from) {
            let split_pos = matched.start();
            [line[..split_pos].to_string(), line[split_pos..].to_string()]
        } else {
            [line.to_string(), String::new()]
        }
    } else {
        [String::new(), String::from(line)]
    }
}

fn edit_line(re: &Regex, replacement: &str, flags: &str, line: &str) -> String {
    if flags.contains('g') {
        re.replace_all(&line, replacement).to_string()
    } else {
        re.replace(&line, replacement).to_string()
    }
}

fn substitute(pattern: &str, replacement: &str, flags: &str, line: &str) -> Vec<String> {
    let re = Regex::new(&pattern).expect("Invalid regular expression");
    if !flags.is_empty() {
        let [mut new_line, line] = line_to_edit(&re, flags, line);
        let edited_line = edit_line(&re, replacement, flags, &line);
        new_line.push_str(&edited_line);
        if flags.contains('w') && re.is_match(&line) {
            if let Some(file_name) = flags.split(' ').collect::<Vec<&str>>().pop() {
                append_or_create(PathBuf::from(&file_name), &new_line)
                    .expect("Failed to write file");
            } else {
                panic!("File name is required for w");
            }
        }
        if flags.contains('p') && re.is_match(&line) {
            vec![new_line.clone(), new_line]
        } else {
            vec![new_line]
        }
    } else {
        vec![re.replace(line, replacement).to_string()]
    }
}

pub fn build_ast(expressions: &[String], lines: &[String]) -> Vec<(Options, Operation)> {
    let mut options = Options {
        matcher: Matcher::None,
        neg: false,
    };
    let mut bldv: Vec<(Options, Operation)> = Vec::new();
    for expression in expressions {
        let characters: Vec<_> = expression.chars().collect();
        let mut index = 0usize;
        while index < characters.len() {
            let c = characters[index];
            match c {
                'a' => bldv.push(build_insert(
                    InsertType::After,
                    &mut index,
                    &characters,
                    options.clone(),
                )),
                'd' => bldv.push((options.clone(), Operation::Delete)),
                'i' => bldv.push(build_insert(
                    InsertType::Before,
                    &mut index,
                    &characters,
                    options.clone(),
                )),
                'n' => bldv.push((options.clone(), Operation::Skip)),
                'p' => bldv.push((options.clone(), Operation::Print)),
                'q' => bldv.push((options.clone(), Operation::Quit)),
                's' => bldv.push(build_subs(&mut index, &characters, options.clone())),
                'w' => bldv.push(build_write(&mut index, &characters, options.clone())),
                '=' => bldv.push((options.clone(), Operation::PrintLineNumber)),
                '!' => options.neg = true,
                ';' => {
                    options = Options {
                        matcher: Matcher::None,
                        neg: false,
                    };
                }
                ',' => (),
                ' ' => (),
                _ if c.is_digit(10) || c == '$' || c == '/' => {
                    options = build_options(&mut index, &characters, lines)
                }
                command => panic!("Invalid command: {}", command),
            }
            index += 1;
        }
    }
    bldv
}

fn is_valid(options: &Options, line_number: usize) -> bool {
    let valid = match options.matcher {
        Matcher::None => true,
        Matcher::Range(from, to) => {
            from == 0 || to == 0 || line_number >= from - 1 && line_number < to
        }
        Matcher::Single(n) => {
            let n: usize = n;
            n == 0 || line_number == n - 1
        }
    };
    if options.neg {
        !valid
    } else {
        valid
    }
}

pub fn execute(opt: &Opt, expressions: &[(Options, Operation)], lines: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut lines = lines.to_owned();
    let mut line_number: usize = 0;
    let lines_len = lines.len();
    let mut inserted_before: Vec<String> = Vec::new();
    let mut inserted_after: Vec<String> = Vec::new();
    while line_number < lines_len {
        let mut line = &mut lines[line_number];
        let mut printed: Vec<String> = Vec::new();
        for op in expressions {
            if !is_valid(&op.0, line_number) {
                continue;
            }
            match &op.1 {
                Operation::Subs(substitution) => {
                    let mut new_line =
                        substitute(&substitution[0], &substitution[1], &substitution[2], &line);
                    *line = new_line[0].clone();
                    new_line.pop();
                    printed.append(&mut new_line);
                }
                Operation::Write(file_name) => {
                    append_or_create(file_name.to_path_buf(), &line).expect("Failed to write file");
                }
                Operation::Delete => line.clear(),
                Operation::Print => printed.push(line.clone()),
                Operation::Skip => {
                    if !opt.quiet {
                        result.push(line.to_string());
                    }
                    line_number += 1;
                    line = &mut lines[line_number];
                }
                Operation::PrintLineNumber => {
                    printed.push(format!("{}\n", line_number + 1));
                }
                Operation::Quit => {
                    line_number = lines_len;
                    break;
                }
                Operation::InsertAfter(line) => {
                    inserted_after.push(line.to_string());
                }
                Operation::InsertBefore(line) => {
                    inserted_before.push(line.to_string());
                }
            }
        }
        result.append(&mut inserted_before);
        result.append(&mut printed);
        if !opt.quiet {
            result.push(line.to_string());
        }
        result.append(&mut inserted_after);
        line_number += 1;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid_returns_true() {
        let options = Options {
            matcher: Matcher::Single(2),
            neg: false,
        };
        assert_eq!(is_valid(&options, 1usize), true);
    }

    #[test]
    fn is_valid_returns_false() {
        let options = Options {
            matcher: Matcher::Single(1),
            neg: false,
        };
        assert_eq!(is_valid(&options, 1usize), false);
    }
}
