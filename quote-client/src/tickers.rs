use std::fs;

pub fn read_tickers(path: &str) -> Vec<String> {
    let content = fs::read_to_string(path).expect("failed to read tickers file");
    content.lines().map(|l| l.trim().to_uppercase()).collect()
}
