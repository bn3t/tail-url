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
    // match result {
    //     Ok(res) => Ok(res.headers().has::<AcceptRanges>()),
    //     Err(err) => false,
    // }
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
    let mut result: Result<String>;
    let mut buf = String::new();

    let client = Client::new();
    let resp = client
        .get(url)
        .header(Range::bytes(offset, length - 1))
        .send();
    if let Ok(mut resp) = resp {
        result = match resp.status() {
            StatusCode::Ok | StatusCode::PartialContent => match resp.text() {
                Ok(s) => {
                    buf.push_str(s.as_str());
                    Ok(buf)
                }
                Err(err) => {
                    println!("Err: {}", err);
                    Err(format!("Error fetching text: {}", err).into())
                }
            },
            _ => return Err("".into()),
        }
    } else {
        result = Err("Request was not ok".into());
    }
    result
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
