use regex::Regex;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rsed", about = "A sed implementation using rust")]
pub struct Opt {
    /// Quiet mode
    #[structopt(short = "n", long)]
    quiet: bool,

    /// Expression command
    #[structopt(short, long, number_of_values = 1)]
    expression: Vec<String>,

    /// Script file
    #[structopt(short, long)]
    file: Option<PathBuf>,

    /// Modify a file in place
    #[structopt(short, long)]
    pub in_place: Option<String>,

    /// Either an expression and a file name or just the file name if -e is set
    #[structopt(name = "ARGS", min_values = 1)]
    args: Vec<String>,
}

impl Opt {
    pub fn get_expressions(&self) -> String {
        if let Some(file) = &self.file {
            std::fs::read_to_string(file)
                .expect("File does not exist")
                .lines()
                .collect::<Vec<&str>>()
                .join(";")
        } else if !self.expression.is_empty() {
            self.expression.join(";")
        } else if self.args.len() >= 2 {
            self.args[0].clone()
        } else {
            panic!("<expression> required");
        }
    }

    pub fn get_file_name(&self) -> PathBuf {
        PathBuf::from(self.args.clone().pop().unwrap())
    }

    pub fn get_file_lines(&self) -> Vec<String> {
        let file_names = if self.expression.is_empty() && self.file.is_none() {
            &self.args[1..]
        } else {
            &self.args
        };
        let mut lines: Vec<String> = Vec::new();
        for file_name in file_names {
            lines.append(
                &mut fs::read_to_string(file_name)
                    .expect("File does not exist")
                    .lines()
                    .map(|l| format!("{}\n", l))
                    .collect::<Vec<String>>(),
            );
        }
        lines
    }
}

enum Build {
    None,
    Num(String),
    Regex(String, bool),
}

#[derive(Debug, Clone)]
pub struct Options {
    matcher: Matcher,
    neg: bool,
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
enum Matcher {
    Range(usize, usize),
    Single(usize),
    None,
}

fn append_or_create(file_name: PathBuf, new_line: &str) -> Result<(), Box<dyn Error>> {
    let mut file_content = std::fs::read_to_string(&file_name).unwrap_or_default();
    file_content.push_str(new_line);
    std::fs::write(file_name, file_content)?;
    Ok(())
}

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

fn get_regex_position(re: Regex, lines: &[String]) -> usize {
    let mut pos: usize = 0;
    for (i, line) in lines.iter().enumerate() {
        if re.is_match(line) {
            pos = i + 1;
            break;
        }
    }
    pos
}

fn build_subs(index: &mut usize, characters: &[char], options: Options) -> (Options, Operation) {
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

fn build_write(index: &mut usize, characters: &[char], options: Options) -> (Options, Operation) {
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

enum InsertType {
    Before,
    After,
}

fn build_insert(
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

fn build_options(index: &mut usize, characters: &[char], lines: &[String]) -> Options {
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

pub fn build_ast(expression: &str, lines: &[String]) -> Vec<(Options, Operation)> {
    let mut options = Options {
        matcher: Matcher::None,
        neg: false,
    };
    let mut bldv: Vec<(Options, Operation)> = Vec::new();
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
