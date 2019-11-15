pub mod substitution {
    use crate::helpers::{Operation, Options};
    pub fn build_subs(
        index: &mut usize,
        characters: &[char],
        options: Options,
    ) -> (Options, Operation) {
        *index += 1;
        let sep: char = characters[*index];
        let mut pos: usize = 0;
        let mut sub = [String::new(), String::new(), String::new()];
        while *index < characters.len() {
            let c = characters[*index];
            if c == sep {
                pos += 1;
            } else if pos <= 3 && c != ';' {
                sub[pos - 1].push(c);
            } else if c == ';' {
                *index -= 1;
                break;
            } else {
                panic!("Invalid substitution command");
            }
            *index += 1;
        }
        (options, Operation::Subs(sub.to_owned()))
    }
}

pub mod write {
    use crate::helpers::{Operation, Options};
    use std::path::PathBuf;
    pub fn build_write(
        index: &mut usize,
        characters: &[char],
        options: Options,
    ) -> (Options, Operation) {
        *index += 1;
        let mut file_name = String::new();
        while characters[*index] == ' ' {
            *index += 1;
        }
        while *index < characters.len() {
            let c = characters[*index];
            if c == ';' {
                *index -= 1;
                break;
            } else {
                file_name.push(c);
            }
            *index += 1;
        }
        if file_name.is_empty() {
            panic!("Cannot write to file with no name");
        }
        (options, Operation::Write(PathBuf::from(file_name)))
    }
}

pub mod insert {
    use crate::helpers::{Operation, Options};

    pub enum InsertType {
        Before,
        After,
    }

    pub fn build_insert(
        insert_type: InsertType,
        index: &mut usize,
        characters: &[char],
        options: Options,
    ) -> (Options, Operation) {
        *index += 1;
        let mut line = String::new();
        while characters[*index] == ' ' {
            *index += 1;
        }
        while *index < characters.len() {
            line.push(characters[*index]);
            *index += 1;
        }
        line.push('\n');
        (
            options,
            match insert_type {
                InsertType::Before => Operation::InsertBefore(line),
                InsertType::After => Operation::InsertAfter(line),
            },
        )
    }
}

pub mod options {
    use crate::helpers::{Options, Matcher, Build, get_regex_position};
    use regex::Regex;
    pub fn build_options(index: &mut usize, characters: &[char], lines: &[String]) -> Options {
        let mut options = Options {
            matcher: Matcher::None,
            neg: false,
        };
        let mut bld = Build::None;
        while *index < characters.len() {
            let c = characters[*index];
            match bld {
                Build::None => {
                    if c.is_digit(10) {
                        bld = Build::Num(c.to_string());
                    } else if c == '/' {
                        bld = Build::Regex(String::new(), false);
                    } else if c == '$' {
                        bld = Build::Num(lines.len().to_string());
                    } else if c != ' ' && c != ',' {
                        *index -= 1;
                        break;
                    }
                    *index += 1;
                }
                Build::Num(ref mut num) => {
                    if c.is_digit(10) {
                        num.push(c);
                        *index += 1;
                    } else {
                        if let Matcher::Single(from) = options.matcher {
                            options = Options {
                                matcher: Matcher::Range(from, num.parse().unwrap()),
                                neg: c == '!',
                            };
                        } else {
                            options = Options {
                                matcher: Matcher::Single(num.parse().unwrap()),
                                neg: c == '!',
                            };
                        }
                        bld = Build::None;
                    }
                }
                Build::Regex(ref mut search, ref mut finished) => {
                    if c != '/' && !*finished {
                        search.push(c);
                    } else if c == '/' {
                        *finished = true;
                    } else {
                        if let Matcher::Single(from) = options.matcher {
                            options = Options {
                                matcher: Matcher::Range(
                                    from,
                                    get_regex_position(
                                        Regex::new(search).expect("Invalid regular expression"),
                                        lines,
                                    ),
                                ),
                                neg: c == '!',
                            }
                        } else {
                            options = Options {
                                matcher: Matcher::Single(get_regex_position(
                                    Regex::new(search).expect("Invalid regular expression"),
                                    lines,
                                )),
                                neg: c == '!',
                            };
                        }
                        bld = Build::None;
                        continue;
                    }
                    *index += 1;
                }
            }
        }
        options
    }
}
