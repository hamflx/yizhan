use std::{
    io::{stdin, Write},
    net::TcpStream,
};

fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:3778").unwrap();
    let stdin = stdin();
    for line in stdin.lines() {
        let line = line.unwrap();
        let mut line = line.trim().to_string();
        line.push('\n');
        if !line.is_empty() {
            stream.write_all(line.as_bytes()).unwrap();
        }
    }
}
