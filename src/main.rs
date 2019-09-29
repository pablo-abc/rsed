use regex::Regex;
use std::fs;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "red", about = "A sed implementation using rust")]
struct Opt {
    /// Quiet mode
    #[structopt(short = "n", long)]
    quiet: bool,

    /// Expression command
    #[structopt(short, long)]
    expression: Vec<String>,

    /// Substitution command
    #[structopt(name = "COMMAND")]
    command: Option<String>,

    /// File name
    #[structopt(name = "FILE")]
    file_name: Option<String>,
}

fn get_commands(opt: &Opt) -> Vec<String> {
    if opt.expression.is_empty() {
        if let Some(command) = &opt.command {
            vec![command.clone()]
        } else {
            panic!("<command> required");
        }
    } else {
        opt.expression.clone()
    }
}

fn main() {
    let opt = Opt::from_args();
    let commands = get_commands(&opt);
    let file_string = fs::read_to_string(opt.file_name.unwrap()).unwrap();
    for command in commands {
        let separator = command.chars().collect::<Vec<_>>()[1];
        let subs: Vec<_> = command.split(separator).collect();
        if subs.len() < 3 || subs[0] != "s" {
            panic!("Invalid command: {}", command);
        }
        let re = Regex::new(subs[1]).unwrap();
        for line in file_string.lines() {
            if let Some(flags) = subs.get(3) {
                let new_line;
                if flags.contains("g") {
                    new_line = re.replace_all(line, subs[2]);
                } else {
                    new_line = re.replace(line, subs[2]);
                }
                if flags.contains("p") && re.is_match(line) {
                    println!("{}", new_line);
                }
                if !opt.quiet {
                    println!("{}", new_line);
                }
            } else {
                println!("{}", re.replace(line, subs[2]));
            }
        }
    }
}
