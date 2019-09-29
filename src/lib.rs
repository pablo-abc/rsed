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
    command_or_file: String,

    /// File name
    #[structopt(name = "FILE")]
    file_name: Option<PathBuf>,
}

impl Opt {
    pub fn get_commands(&self) -> Vec<String> {
        if self.expression.is_empty() {
            if self.file_name.is_some() {
                let command = &self.command_or_file;
                vec![command.clone()]
            } else {
                panic!("<command> required");
            }
        } else {
            self.expression.clone()
        }
    }

    pub fn get_file_name(&self) -> PathBuf {
        if let Some(file_name) = &self.file_name {
            file_name.clone()
        } else {
            PathBuf::from(&self.command_or_file)
        }
    }
}

pub fn parse_line(opt: &Opt, line: &mut String) {
    let commands = opt.get_commands();
    for command in &commands {
        let separator = command.chars().collect::<Vec<_>>()[1];
        let subs: Vec<_> = command.split(separator).collect();
        if subs.len() < 3 || subs[0] != "s" {
            panic!("Invalid command: {}", command);
        }
        let re = Regex::new(subs[1]).unwrap();
        if let Some(flags) = subs.get(3) {
            let new_line = if flags.contains('g') {
                re.replace_all(line, subs[2])
            } else {
                re.replace(line, subs[2])
            };
            if flags.contains('p') && re.is_match(line) {
                println!("{}", new_line);
            }
            *line = new_line.to_string();
        } else {
            *line = re.replace(line, subs[2]).to_string();
        }
    }
    if !opt.quiet {
        println!("{}", line);
    }
}
