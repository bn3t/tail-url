#![allow(warnings)] // remove when error_chain is fixed

extern crate argparse;
#[macro_use]
extern crate error_chain;
extern crate reqwest;

use argparse::{ArgumentParser, Print, Store, StoreOption};
use std::env;
use std::thread;

use reqwest::{Client, Response, StatusCode};
use reqwest::header::{AcceptRanges, ContentLength, Range};

struct Options {
    tail_offset: Option<u64>,
    url: String,
}

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
    let mut options = Options {
        tail_offset: None,
        url: String::new(),
    };
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Tail text coming from an URL");
        ap.refer(&mut options.tail_offset).add_option(
            &["-t"],
            StoreOption,
            "Ttarting tail offset in bytes",
        );
        ap.refer(&mut options.url)
            .add_argument("url", Store, "URL to tail")
            .required();
        ap.add_option(
            &["-v", "--version"],
            Print(format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")).to_string()),
            "Show version",
        );
        ap.parse_args_or_exit();
    }

    if check_http_range(options.url.as_str())? {
        let mut length = get_length(options.url.as_str())?;
        let mut offset = match options.tail_offset {
            Some(n) => if n < length {
                n
            } else {
                length
            },
            None => length,
        };
        loop {
            if offset < length {
                let body = get_body(options.url.as_str(), offset, length);
                print!("{}", body?);
                offset = length;
            }
            thread::sleep_ms(1000);
            length = get_length(options.url.as_str())?;
        }
    } else {
        println!("Http Range not supported by server, sorry!");
    }
    Ok(())
}

quick_main!(run);
