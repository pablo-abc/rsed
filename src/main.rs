use rsed::{build_ast, execute, Opt};
use std::fs;

fn main() {
    let opt = Opt::from_args();
    let file_name = opt.get_file_names();
    let file_lines = opt.get_lines();
    let expressions = build_ast(&opt.get_expressions(), &file_lines);
    let result = execute(&opt, &expressions, &file_lines).join("");
    if let Some(in_place) = opt.in_place {
        if in_place.is_empty() {
            fs::write(&file_name[0], result).expect("Error writing to file");
        } else {
            let mut tmp = file_name[0].clone();
            tmp.set_extension(in_place);
            fs::write(tmp, file_lines.join("")).expect("Error writing to default file");
            fs::write(&file_name[0], result).expect("Error writing to file");
        }
    } else {
        print!("{}", result);
    }
}
