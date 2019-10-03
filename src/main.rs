use rsed::{execute, build_ast, Opt};
use std::fs;
use structopt::StructOpt;

fn main() {
    let opt = Opt::from_args();
    let file_name = opt.get_file_name();
    let file_content = fs::read_to_string(&file_name).expect("File does not exist");
    let mut file_lines: Vec<String> = file_content.lines().map(|l| l.to_string()).collect();
    let mut result = Vec::new();
    let expressions = build_ast(&opt.get_expressions());
    for (i, line) in file_lines.iter_mut().enumerate() {
        line.push('\n');
        result.append(&mut execute(&opt, &expressions, i, line));
    }
    let result = result.join("");
    if let Some(in_place) = opt.in_place {
        if in_place.is_empty() {
            fs::write(&file_name, result).expect("Error writing to file");
        } else {
            let mut tmp = file_name.clone();
            tmp.set_extension(in_place);
            fs::write(tmp, file_content).expect("Error writing to default file");
            fs::write(&file_name, result).expect("Error writing to file");
        }
    } else {
        print!("{}", result);
    }
}
