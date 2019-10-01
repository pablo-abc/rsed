use regex::Regex;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "red", about = "A sed implementation using rust")]
pub struct Opt {
    /// Quiet mode
    #[structopt(short = "n", long)]
    pub quiet: bool,

    /// Expression command
    #[structopt(short, long, number_of_values = 1)]
    expression: Vec<String>,

    /// Substitution command
    #[structopt(name = "COMMAND")]
    expression_or_file: String,

    /// File name
    #[structopt(name = "FILE")]
    file_name: Option<PathBuf>,
}

impl Opt {
    pub fn get_expressions(&self) -> Vec<String> {
        if self.expression.is_empty() {
            if self.file_name.is_some() {
                let expression = &self.expression_or_file;
                vec![expression.clone()]
            } else {
                panic!("<expression> required");
            }
        } else {
            self.expression.clone()
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
    Subs(Option<char>, usize, [String; 3]),
    NumRange(String, String),
    LineNum(String),
}

enum Operation {
    None,
    Subs([String; 3]),
}

fn substitute(pattern: &str, replacement: &str, flags: &str, line: &mut String) {
    let re = Regex::new(&pattern).unwrap();
    if !flags.is_empty() {
        let new_line = if flags.contains('g') {
            re.replace_all(line, replacement)
        } else {
            re.replace(line, replacement)
        };
        if flags.contains('p') && re.is_match(line) {
            println!("{}", new_line);
        }
        *line = new_line.to_string();
    } else {
        *line = re.replace(line, replacement).to_string();
    }
}

fn build_operation(expression: &str, line_number: usize) -> Operation {
    let mut op = Operation::None;
    let mut bld = Build::None;
    for c in expression.chars() {
        match bld {
            Build::None => (),
            Build::Subs(sep, section, mut substitution) => {
                if let Some(separator) = sep {
                    if c == separator {
                        bld = Build::Subs(sep, section + 1, substitution);
                        continue;
                    } else if section <= 3 {
                        substitution[section - 1].push(c);
                        bld = Build::Subs(sep, section, substitution.clone());
                        op = Operation::Subs(substitution);
                        continue;
                    }
                    panic!("Invalid substitution command");
                } else {
                    bld = Build::Subs(Some(c), 1, substitution);
                    continue;
                }
            }
            Build::LineNum(mut num) => {
                if c.is_digit(10) {
                    num.push(c);
                    bld = Build::LineNum(num);
                    continue;
                } else if c == ',' {
                    bld = Build::NumRange(num, String::new());
                    continue;
                } else if c == ' ' {
                    bld = Build::LineNum(num);
                    continue;
                }
                bld = Build::None;
                let num: usize = num.parse().unwrap();
                if num == 0 || line_number != num - 1 {
                    break;
                }
            }
            Build::NumRange(from, mut to) => {
                if c.is_digit(10) {
                    to.push(c);
                    bld = Build::NumRange(from, to);
                    continue;
                }
                bld = Build::None;
                let from: usize = from.parse().unwrap();
                let to: usize = to.parse().unwrap();
                if from == 0 || to == 0 || line_number < from - 1 || line_number > to - 1 {
                    break;
                }
            }
        }
        match c {
            's' => {
                bld = Build::Subs(None, 0, [String::new(), String::new(), String::new()]);
            }
            num if c.is_digit(10) => bld = Build::LineNum(num.to_string()),
            _ => (),
        }
    }
    op
}

fn parse_expression(expression: &str, line_number: usize, line: &mut String) {
    match build_operation(expression, line_number) {
        Operation::Subs(substitution) => {
            substitute(&substitution[0], &substitution[1], &substitution[2], line)
        }
        _ => (),
    }
}

pub fn parse_line(opt: &Opt, line_number: usize, line: &mut String) {
    for expression in &opt.get_expressions() {
        parse_expression(expression, line_number, line);
    }
    if !opt.quiet {
        println!("{}", line);
    }
}
