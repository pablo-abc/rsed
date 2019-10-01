use rsed::{parse_line, Opt};
use std::fs;
use structopt::StructOpt;

fn main() {
    let opt = Opt::from_args();
    let mut file_lines: Vec<String> = fs::read_to_string(opt.get_file_name())
        .expect("File does not exist")
        .lines()
        .map(|l| l.to_string())
        .collect();
    for (i, line) in file_lines.iter_mut().enumerate() {
        line.push('\n');
        parse_line(&opt, i, line);
    }
}
