#![feature(env)]
#![feature(io)]
#![feature(os)]
#![feature(path)]
#![feature(unicode)]

use compiler::Lexer;
use std::env;
use std::old_io::fs::File;
use std::old_io::{BufferedReader, IoErrorKind};

mod compiler;

#[allow(dead_code)]
fn main() {
    let mut args = env::args();
    let file_name = match args.nth(1).and_then(|s| s.into_string().ok()) {
        Some(v) => v,
        None => panic!("Must provide file to tokenize"),
    };

    println!("Tokenizing {:?}", file_name);
    let file = File::open(&Path::new(file_name)).unwrap();
    let reader = BufferedReader::new(file);
    let mut lexer = Lexer::new(reader);
    loop {
        let token = lexer.read_token();
        match token {
            Ok(t) => println!("{:?}", t),
            Err(ref e) if e.kind == IoErrorKind::EndOfFile => break,
            Err(e) => panic!("Error during tokenization: {}", e),
        }
    }
}
