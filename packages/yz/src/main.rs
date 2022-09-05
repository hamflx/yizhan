use std::{
    io::{stdin, Read, Write},
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
            let mut buffer = vec![0; 1048576];
            let size = stream.read(buffer.as_mut_slice()).unwrap();
            let response = std::str::from_utf8(&buffer.as_slice()[..size]).unwrap();
            println!("{}", response);
        }
    }
}
