use std::fs;
use std::path::PathBuf;

pub struct Opt {
    expressions: Vec<String>,
    pub quiet: bool,
    pub in_place: Option<String>,
    files: Vec<PathBuf>,
}

impl Opt {
    pub fn from_args() -> Opt {
        let args = std::env::args().collect::<Vec<String>>();
        let mut pos = 1usize;
        let mut opt = Opt {
            expressions: Vec::new(),
            quiet: false,
            in_place: None,
            files: Vec::new(),
        };
        while pos < args.len() {
            let arg = args[pos].to_string();
            match arg.as_ref() {
                "-e" | "--expression" => {
                    pos += 1;
                    opt.expressions.push(args[pos].to_string());
                }
                "-f" | "--file" => {
                    pos += 1;
                    let mut expressions = std::fs::read_to_string(args[pos].to_string())
                        .expect("File does not exist")
                        .lines()
                        .map(String::from)
                        .collect::<Vec<String>>();
                    opt.expressions.append(&mut expressions);
                }
                "-q" | "--quiet" => opt.quiet = true,
                "-i" | "--in-place" => {
                    pos += 1;
                    opt.in_place = Some(args[pos].to_string());
                }
                _ => {
                    if opt.expressions.is_empty() {
                        opt.expressions.push(arg);
                    } else {
                        opt.files.push(PathBuf::from(arg));
                    };
                }
            };
            pos += 1;
        }
        opt
    }

    pub fn get_expressions(&self) -> Vec<String> {
        self.expressions.clone()
    }

    pub fn get_file_names(&self) -> Vec<PathBuf> {
        self.files.clone()
    }

    pub fn get_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        for file_name in self.files.iter() {
            lines.append(
                &mut fs::read_to_string(file_name)
                    .expect("File does not exist")
                    .lines()
                    .map(|l| format!("{}\n", l))
                    .collect(),
            );
        }
        lines
    }
}
