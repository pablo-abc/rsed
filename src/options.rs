use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rsed", about = "A sed implementation using rust")]
pub struct Opt {
    /// Quiet mode
    #[structopt(short = "n", long)]
    pub quiet: bool,

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
    pub fn get_expressions(&self) -> Vec<String> {
        if let Some(file) = &self.file {
            std::fs::read_to_string(file)
                .expect("File does not exist")
                .lines()
                .map(String::from)
                .collect::<Vec<String>>()
        } else if !self.expression.is_empty() {
            self.expression.clone()
        } else if self.args.len() >= 2 {
            vec![self.args[0].clone()]
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
                    .collect(),
            );
        }
        lines
    }
}
