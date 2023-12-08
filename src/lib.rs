
use std::fs;
use std::fs::File;
use std::io::Write;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

pub fn test() {
    println!("test");
}

fn read_file(path: &str) -> String {
    println!("reading {path}");
    fs::read_to_string(path).unwrap()
}

fn write_file(path: &str, content: &str) {
    println!("writing {path}");
    fs::write(path, content).unwrap();
}

pub fn go(input: &str, output: &str) {
    let config = read_file(input);
    let name = "spacecraft";
    fs::create_dir_all(&format!("{output}/{name}")).unwrap();
    write_file(&format!("{output}/{name}/config.json"), &config);
}
