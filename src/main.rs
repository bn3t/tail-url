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

fn get_body(url: &str, offset: u64, length: u64) -> &str {
    let client = Client::new();
    let result = client.get(url).header(Range::bytes(offset, length)).send();
    let text = result.map(|mut r: Response | -> reqwest::Result<String> { r.text() });
    match text {
        Ok(s) => s.as_str(),
        Err(_) => ""
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        for argument in env::args() {
            println!("Argument: {}", argument);
        }
        let url = &args[1];

        env_logger::init();

        println!("url {}", url);

        if check_http_range(url) {
            println!("Ok http range");
            let mut length = get_length(url);
            println!("Length: {}", length);
            let mut offset = length - 1000;
            loop {
                if offset < length {
                    println!("Fetching offset=[{}], length=[{}]", offset, length);
                    let body = get_body(url, offset, length);
                    println!("Body {}", body);
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
