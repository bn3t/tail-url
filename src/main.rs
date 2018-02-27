#![allow(warnings)] // remove when error_chain is fixed

//! `cargo run --example simple`

extern crate env_logger;
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

fn check_http_range(url: &str) -> bool {
    let client = Client::new();
    let result = client.head(url).send();
    match result {
        Ok(res) => res.headers().has::<AcceptRanges>(),
        Err(err) => false,
    }
}

fn get_length(url: &str) -> u64 {
    let client = reqwest::Client::new();
    let result = client.head(url).send();
    match result {
        Ok(res) => match res.headers().get::<ContentLength>() {
            Some(cl) => {
                let &ContentLength(length) = cl;
                length
            }
            None => 0,
        },
        Err(err) => 0,
    }
}

fn get_body(url: &str, offset: u64, length: u64) -> String {
    let mut buf = String::new();

    let client = Client::new();
    let result = client
        .get(url)
        .header(Range::bytes(offset, length - 1))
        .send();
    if let Ok(mut resp) = result {
        match resp.status() {
            StatusCode::Ok | StatusCode::PartialContent => match resp.text() {
                Ok(s) => buf.push_str(s.as_str()),
                Err(err) => {
                    println!("Err: {}", err);
                }
            },
            _ => {}
        }
    }
    buf
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let url = &args[1];

        env_logger::init();
        if check_http_range(url) {
            let mut length = get_length(url);
            let mut offset = length;
            loop {
                if offset < length {
                    //println!("Fetching offset=[{}], length=[{}]", offset, length);
                    let body = get_body(url, offset, length);
                    print!("{}", body);
                    offset = length;
                }
                thread::sleep_ms(1000);
                length = get_length(url);
            }

            let mut res = reqwest::get(url)?;
            match res.status() {
                StatusCode::Ok => {
                    println!("Status OK: {}", res.status());
                }
                _ => {
                    println!("Status Not OK: {}", res.status());
                }
            }
        } else {
            println!("Http Range not supported by server, sorry!");
        }

        //println!("Headers:\n{}", res.headers());

        // copy the response body directly to stdout
        //let _ = std::io::copy(&mut res, &mut std::io::stdout())?;

        println!("\n\nDone.");
        Ok(())
    } else {
        Err("No url provided".into())
    }
}

quick_main!(run);
