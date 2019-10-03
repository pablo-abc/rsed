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
    fn get_expressions(&self) -> String {
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
    Subs {
        sep: Option<char>,
        pos: usize,
        sub: [String; 3],
    },
    NumRange(String, String),
    LineNum(String),
    Write(String),
    Skip,
}

pub enum Operation {
    Subs([String; 3]),
    Write(PathBuf),
    Delete,
    Print,
}

fn append_or_create(file_name: PathBuf, new_line: &str) -> Result<(), Box<dyn Error>> {
    let mut file_content = std::fs::read_to_string(&file_name).unwrap_or_default();
    file_content.push_str(new_line);
    std::fs::write(file_name, file_content)?;
    Ok(())
}

fn substitute(pattern: &str, replacement: &str, flags: &str, line: &str) -> Vec<String> {
    let re = Regex::new(&pattern).unwrap();
    if !flags.is_empty() {
        let new_line = if flags.contains('g') {
            re.replace_all(line, replacement).to_string()
        } else {
            re.replace(line, replacement).to_string()
        };
        if flags.contains('w') && re.is_match(line) {
            if let Some(file_name) = flags.split(' ').collect::<Vec<&str>>().pop() {
                append_or_create(PathBuf::from(&file_name), &new_line)
                    .expect("Failed to write file");
            } else {
                panic!("File name is required for w");
            }
        }
        if flags.contains('p') && re.is_match(line) {
            vec![new_line.clone(), new_line]
        } else {
            vec![new_line]
        }
    } else {
        vec![re.replace(line, replacement).to_string()]
    }
}

pub fn build_operation(expression: &str, line_number: usize) -> Vec<Operation> {
    let mut bld = Build::None;
    let mut bldv: Vec<Operation> = Vec::new();
    for c in expression.chars() {
        match bld {
            Build::None => (),
            Build::Subs {
                ref mut sep,
                ref mut pos,
                ref mut sub,
            } => {
                if let Some(separator) = *sep {
                    if c == separator {
                        *pos += 1;
                        continue;
                    } else if *pos <= 3 && c != ';' {
                        sub[*pos - 1].push(c);
                        continue;
                    } else if c == ';' {
                        bldv.push(Operation::Subs(sub.to_owned()));
                        bld = Build::None;
                        continue;
                    }
                    panic!("Invalid substitution command");
                } else {
                    *pos = 1;
                    *sep = Some(c);
                    continue;
                }
            }
            Build::Write(ref mut file_name) => {
                if c == ' ' {
                    continue;
                } else if c == ';' {
                    bldv.push(Operation::Write(PathBuf::from(file_name.to_string())));
                    bld = Build::None;
                    continue;
                }
                file_name.push(c);
                continue;
            }
            Build::LineNum(ref mut num) => {
                if c.is_digit(10) {
                    num.push(c);
                    continue;
                } else if c == ',' {
                    bld = Build::NumRange(num.to_string(), String::new());
                    continue;
                } else if c == ' ' {
                    continue;
                }
                let num: usize = num.parse().unwrap();
                let mut valid = num == 0 || line_number != num - 1;
                bld = Build::None;
                if c == '!' {
                    valid = !valid;
                }
                if valid {
                    bld = Build::Skip;
                    continue;
                }
            }
            Build::NumRange(ref from, ref mut to) => {
                if c.is_digit(10) {
                    to.push(c);
                    continue;
                } else if c == ' ' {
                    continue;
                }
                let from: usize = from.parse().unwrap();
                let to: usize = to.parse().unwrap();
                let mut valid =
                    from == 0 || to == 0 || line_number < from - 1 || line_number > to - 1;
                if c == '!' {
                    valid = !valid;
                }
                if valid {
                    bld = Build::Skip;
                    continue;
                }
                bld = Build::None;
            }
            Build::Skip => {
                if c == ';' {
                    bld = Build::None;
                }
                continue;
            }
        }
        match c {
            's' => {
                bld = Build::Subs {
                    sep: None,
                    pos: 0,
                    sub: [String::new(), String::new(), String::new()],
                };
            }
            'd' => bldv.push(Operation::Delete),
            'p' => bldv.push(Operation::Print),
            'n' => break,
            'w' => bld = Build::Write(String::new()),
            ';' => match bld {
                Build::Subs { sub, .. } => {
                    bld = Build::None;
                    bldv.push(Operation::Subs(sub));
                }
                _ => bld = Build::None,
            },
            num if c.is_digit(10) => bld = Build::LineNum(num.to_string()),
            _ => (),
        }
    }
    match bld {
        Build::Subs { sub, .. } => {
            bldv.insert(0, Operation::Subs(sub));
            bldv
        }
        Build::Write(file_name) => {
            bldv.insert(0, Operation::Write(PathBuf::from(file_name)));
            bldv
        }
        _ => bldv,
    }
}

pub fn parse_line(opt: &Opt, line_number: usize, line: &mut String) -> Vec<String> {
    let mut line: String = line.to_string();
    let mut printed: Vec<String> = Vec::new();
    let mut result = Vec::new();
    for op in build_operation(&opt.get_expressions(), line_number) {
        match op {
            Operation::Subs(substitution) => {
                if !line.is_empty() {
                    let mut new_line =
                        substitute(&substitution[0], &substitution[1], &substitution[2], &line);
                    line = new_line[0].clone();
                    printed = printed.iter().map(|_| line.clone()).collect();
                    new_line.pop();
                    printed.append(&mut new_line);
                }
            }
            Operation::Write(file_name) => {
                append_or_create(file_name, &line).expect("Failed to write file");
            }
            Operation::Delete => line.clear(),
            Operation::Print => printed.push(line.clone()),
        }
    }
    if !opt.quiet {
        result.push(line);
    }
    result.append(&mut printed);
    result
}
