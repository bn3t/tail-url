#![allow(warnings)] // remove when error_chain is fixed

#[macro_use]
extern crate error_chain;
extern crate reqwest;

use std::env;
use std::thread;

use reqwest::{Client, Response, StatusCode};
use reqwest::header::{AcceptRanges, ContentLength, Range};

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(std::io::Error);
    }
}

fn check_http_range(url: &str) -> Result<bool> {
    let client = Client::new();
    client
        .head(url)
        .send()
        .map(|res| res.headers().has::<AcceptRanges>())
        .chain_err(|| "Could not check http range")
}

fn get_length(url: &str) -> Result<u64> {
    let client = reqwest::Client::new();
    let res = client
        .head(url)
        .send()
        .chain_err(|| "Could not get length")?;

    res.headers()
        .get::<ContentLength>()
        .ok_or("No Content-Length in url".into())
        .map(|cl| {
            let &ContentLength(length) = cl;
            length
        })
}

fn get_body(url: &str, offset: u64, length: u64) -> Result<String> {
    let client = Client::new();
    let mut resp = client
        .get(url)
        .header(Range::bytes(offset, length - 1))
        .send()
        .chain_err(|| "Request was not ok")?;
    match resp.status() {
        StatusCode::Ok | StatusCode::PartialContent => match resp.text() {
            Ok(s) => {
                let buf = String::from(s.as_str());
                Ok(buf)
            }
            Err(err) => Err(format!("Error fetching text: {}", err).into()),
        },
        code => Err(format!("Unexpected status code from server: {}", code).into()),
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let url = &args[1];

        if check_http_range(url)? {
            let mut length = get_length(url)?;
            let mut offset = length;
            loop {
                if offset < length {
                    //println!("Fetching offset=[{}], length=[{}]", offset, length);
                    let body = get_body(url, offset, length);
                    print!("{}", body?);
                    offset = length;
                }
                thread::sleep_ms(1000);
                length = get_length(url)?;
            }
        } else {
            println!("Http Range not supported by server, sorry!");
        }
        println!("\n\nDone.");
        Ok(())
    } else {
        Err("No url provided".into())
    }
}

quick_main!(run);
