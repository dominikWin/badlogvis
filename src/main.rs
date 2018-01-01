extern crate structopt;
#[macro_use]
extern crate structopt_derive;

use std::fs::File;
use std::io::prelude::*;

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "badlogvis", about = "Create html from badlog data")]
struct Opt {
    #[structopt(help = "Input file")]
    input: String,

    #[structopt(help = "Output file, default to <input>.html")]
    output: Option<String>,
}

fn main() {
    let opt: Opt = Opt::from_args();
    println!("{:?}", opt);

    let input = opt.input;

    let mut f = File::open(input).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    println!("{}", contents);

    let json_header = contents.lines().take(1).last().unwrap().to_string();

    println!("{}", json_header);
}
