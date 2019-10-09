use regex::Regex;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "red", about = "A sed implementation using rust")]
pub struct Opt {
    /// Quiet mode
    #[structopt(short = "n", long)]
    quiet: bool,

    /// Expression command
    #[structopt(short, long, number_of_values = 1)]
    expression: Vec<String>,

    /// Modify a file in place
    #[structopt(short, long)]
    pub in_place: Option<String>,

    /// Substitution command
    #[structopt(name = "COMMAND")]
    expression_or_file: String,

    /// File name
    #[structopt(name = "FILE")]
    file_name: Option<PathBuf>,
}

impl Opt {
    pub fn get_expressions(&self) -> String {
        if self.expression.is_empty() {
            if self.file_name.is_some() {
                self.expression_or_file.clone()
            } else {
                panic!("<expression> required");
            }
        } else {
            self.expression.join(";")
        }
    }

    pub fn get_file_name(&self) -> PathBuf {
        if let Some(file_name) = &self.file_name {
            file_name.clone()
        } else {
            PathBuf::from(&self.expression_or_file)
        }
    }
}

enum Build {
    None,
    Num(String),
    Regex(String, bool),
}

#[derive(Clone)]
pub struct Options {
    matcher: Matcher,
    neg: bool,
}

pub enum Operation {
    Subs([String; 3]),
    Write(PathBuf),
    Delete,
    Print,
}

#[derive(Clone)]
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
                    bld = Build::Num(characters[*index].to_string());
                } else if c == '/' {
                    bld = Build::Regex(String::new(), false);
                } else if c == ' ' || c == ',' {
                    *index += 1;
                } else {
                    break;
                }
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
    let mut bld = Build::None;
    let mut bldv: Vec<(Options, Operation)> = Vec::new();
    let characters: Vec<_> = expression.chars().collect();
    let mut index = 0usize;
    while index < characters.len() {
        let c = characters[index];
        match bld {
            Build::None => {
                match c {
                    's' => bldv.push(build_subs(&mut index, &characters, options.clone())),
                    'd' => {
                        bldv.push((options.clone(), Operation::Delete));
                        options = Options {
                            matcher: Matcher::None,
                            neg: false,
                        };
                    }
                    'p' => {
                        bldv.push((options.clone(), Operation::Print));
                        options = Options {
                            matcher: Matcher::None,
                            neg: false,
                        };
                    }
                    'n' => break,
                    'w' => bldv.push(build_write(&mut index, &characters, options.clone())),
                    '/' => bld = Build::Regex(String::new(), false),
                    '!' => options.neg = true,
                    ';' => {
                        bld = Build::None;
                        options = Options {
                            matcher: Matcher::None,
                            neg: false,
                        };
                    }
                    ',' => (),
                    num if c.is_digit(10) => bld = Build::Num(num.to_string()),
                    command => panic!("Invalid command: {}", command),
                }
                index += 1;
            }
            Build::Num(ref mut num) => {
                if c.is_digit(10) {
                    num.push(c);
                    index += 1;
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
                index += 1;
            }
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
    for (line_number, line) in lines.iter_mut().enumerate() {
        let mut printed: Vec<String> = Vec::new();
        for op in expressions {
            match op {
                (options, Operation::Subs(substitution)) => {
                    if !line.is_empty() && is_valid(options, line_number) {
                        let mut new_line =
                            substitute(&substitution[0], &substitution[1], &substitution[2], &line);
                        *line = new_line[0].clone();
                        printed = printed.iter().map(|_| line.clone()).collect();
                        new_line.pop();
                        printed.append(&mut new_line);
                    }
                }
                (options, Operation::Write(file_name)) => {
                    if is_valid(options, line_number) {
                        append_or_create(file_name.to_path_buf(), &line)
                            .expect("Failed to write file");
                    };
                }
                (options, Operation::Delete) => {
                    if is_valid(options, line_number) {
                        line.clear()
                    }
                }
                (options, Operation::Print) => {
                    if is_valid(options, line_number) {
                        printed.push(line.clone())
                    }
                }
            }
        }
        if !opt.quiet {
            result.push(line.to_string());
        }
        result.append(&mut printed);
    }
    result
}
