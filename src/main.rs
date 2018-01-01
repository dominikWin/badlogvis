extern crate structopt;
#[macro_use]
extern crate structopt_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

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

#[derive(Serialize, Deserialize, Debug)]
struct Topic {
    name: String,
    unit: String,
    attrs: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
struct Value {
    name: String,
    value: String
}

#[derive(Serialize, Deserialize, Debug)]
struct JSONHeader {
    topics: Vec<Topic>,
    values: Vec<Value>
}

fn main() {
    let opt: Opt = Opt::from_args();

    let input = opt.input;

    let mut f = File::open(input).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();

    let json_header = contents.lines().take(1).last().unwrap().to_string();

    let p: JSONHeader = serde_json::from_str(&json_header).unwrap();

    println!("{:?}", p);
}
