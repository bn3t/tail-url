#![allow(warnings)] // remove when error_chain is fixed

extern crate argparse;
#[macro_use]
extern crate error_chain;
extern crate reqwest;

use argparse::{ArgumentParser, Print, Store, StoreOption, StoreTrue};

use std::io::{stderr, stdout};
use std::env;
use std::thread;
use std::process::exit;

use reqwest::{Client, Response, StatusCode};
use reqwest::header::{AcceptRanges, ContentLength, Range};

struct Options {
    tail_offset: Option<u64>,
    reverse_tail_offset: bool,
    url: String,
}

error_chain! {
    foreign_links {
        ReqError(reqwest::Error);
        IoError(std::io::Error);
    }
}

struct TrailHttpClient {
    client: Client,
}

struct StdoutOutput {}

impl TrailHttpClient {
    pub fn new() -> TrailHttpClient {
        let client = Client::new();
        TrailHttpClient { client: client }
    }
}

trait HttpClient {
    fn has_http_range(&self, url: &str) -> Result<bool>;
    fn get_length(&self, url: &str) -> Result<u64>;
    fn get_body(&self, url: &str, offset: u64, length: u64) -> Result<String>;
}

impl HttpClient for TrailHttpClient {
    fn has_http_range(&self, url: &str) -> Result<bool> {
        self.client
            .head(url)
            .send()
            .map(|res| res.headers().has::<AcceptRanges>())
            .chain_err(|| "Could not check http range")
    }

    fn get_length(&self, url: &str) -> Result<u64> {
        let res = self.client
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

    fn get_body(&self, url: &str, offset: u64, length: u64) -> Result<String> {
        let mut resp = self.client
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
}

trait Output {
    fn output(&mut self, text: &str) -> bool;
}

impl Output for StdoutOutput {
    fn output(&mut self, text: &str) -> bool {
        print!("{}", text);
        true
    }
}

fn parse_options(args: Vec<String>) -> std::result::Result<Options, i32> {
    let mut options = Options {
        tail_offset: None,
        reverse_tail_offset: false,
        url: String::new(),
    };
    {
        let mut ap = ArgumentParser::new();
        ap.set_description("Tail text coming from an URL");
        ap.refer(&mut options.tail_offset).add_option(
            &["-t"],
            StoreOption,
            "Starting tail offset in bytes",
        );
        ap.refer(&mut options.reverse_tail_offset).add_option(
            &["-r"],
            StoreTrue,
            "Consider the tail offset from the end",
        );
        ap.refer(&mut options.url)
            .add_argument("url", Store, "URL to tail")
            .required();
        ap.add_option(
            &["-v", "--version"],
            Print(
                format!(
                    "{} {} ({} {})",
                    env!("CARGO_PKG_NAME"),
                    env!("CARGO_PKG_VERSION"),
                    env!("GIT_COMMIT"),
                    env!("BUILD_DATE")
                ).to_string(),
            ),
            "Show version",
        );
        match ap.parse(args, &mut stdout(), &mut stderr()) {
            Ok(_) => {}
            Err(err) => {
                return Err(err);
            }
        }
    }
    Ok(options)
}

fn run_http_get<T: HttpClient, O: Output>(
    mut http_client: T,
    mut out: O,
    options: Options,
) -> Result<()> {
    if http_client.has_http_range(options.url.as_str())? {
        let mut length = http_client.get_length(options.url.as_str())?;
        let mut offset = match options.tail_offset {
            Some(n) => if n < length {
                if options.reverse_tail_offset {
                    length - n
                } else {
                    n
                }
            } else {
                length
            },
            None => length,
        };
        loop {
            if offset < length {
                let body = http_client.get_body(options.url.as_str(), offset, length);
                if !out.output(body?.as_str()) {
                    break;
                }
                offset = length;
            }
            thread::sleep_ms(1000);
            length = http_client.get_length(options.url.as_str())?;
        }
    } else {
        println!("Http Range not supported by server, sorry!");
    }
    Ok(())
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let options = parse_options(env::args().collect())
        .map_err(|c| exit(c))
        .unwrap();

    let http_client = TrailHttpClient::new();
    let output = StdoutOutput {};

    run_http_get(http_client, output, options)
}

quick_main!(run);

#[cfg(test)]
mod test_options {
    use super::parse_options;
    #[test]
    fn parse_verbose() {
        let args = vec![String::from("tail-url"), String::from("-v")];
        let options = parse_options(args);
        assert_eq!(options.is_err(), true);
        assert_eq!(options.err(), Some(0));
    }

    #[test]
    fn parse_valid_params_all_params() {
        let args = vec![
            String::from("tail-url"),
            String::from("-t"),
            String::from("100"),
            String::from("my-url"),
        ];
        let options = parse_options(args);
        assert_eq!(options.is_ok(), true, "Returned options should be Ok");
        let options = options.unwrap();
        assert_eq!(options.tail_offset, Some(100));
        assert_eq!(options.reverse_tail_offset, false);
        assert_eq!(options.url, "my-url");
    }

    #[test]
    fn parse_valid_params_tail_reverse() {
        let args = vec![
            String::from("tail-url"),
            String::from("-t"),
            String::from("100"),
            String::from("-r"),
            String::from("my-url"),
        ];
        let options = parse_options(args);
        assert_eq!(options.is_ok(), true, "Returned options should be Ok");
        let options = options.unwrap();
        assert_eq!(options.tail_offset, Some(100));
        assert_eq!(options.reverse_tail_offset, true);
        assert_eq!(options.url, "my-url");
    }

    #[test]
    fn parse_valid_params_2() {
        let args = vec![String::from("tail-url"), String::from("my-url")];
        let options = parse_options(args);
        assert_eq!(options.is_ok(), true, "Returned options should be Ok");
        let options = options.unwrap();
        assert_eq!(options.tail_offset, None);
        assert_eq!(options.url, "my-url");
    }
}

#[cfg(test)]
mod test_run_http_get {
    use super::run_http_get;
    use HttpClient;
    use Options;
    use Result;
    use super::Output;

    static mut stored_output: Option<Vec<String>> = None;
    static mut length_responses: Option<Vec<u64>> = None;

    struct StubHttpClient {}
    struct StubStdout {}

    impl HttpClient for StubHttpClient {
        fn has_http_range(&self, url: &str) -> Result<bool> {
            Ok(true)
        }

        fn get_length(&self, url: &str) -> Result<u64> {
            unsafe {
                match length_responses {
                    Some(ref mut lr) => Ok(lr.pop().unwrap_or(1010)),
                    None => panic!(),
                }
            }
        }

        fn get_body(&self, url: &str, offset: u64, length: u64) -> Result<String> {
            Ok(String::from("A text"))
        }
    }

    impl Output for StubStdout {
        fn output(&mut self, text: &str) -> bool {
            unsafe {
                match stored_output {
                    Some(ref mut so) => {
                        so.push(String::from(text));
                        so.len() < 2
                    }
                    _ => panic!(),
                }
            }
        }
    }

    #[test]
    fn test_run_http_get() {
        println!("test_run_http_get()");

        unsafe {
            stored_output = Some(Vec::new());
            length_responses = Some(vec![1020, 1010, 1000]);
        }
        let options = Options {
            tail_offset: None,
            reverse_tail_offset: false,
            url: String::from("my-url"),
        };
        let mut stub_output = StubStdout {};
        let mut stub_http_client = StubHttpClient {};

        run_http_get(stub_http_client, stub_output, options);
        unsafe {
            if let Some(ref so) = stored_output {
                assert_eq!(so.len(), 2);
                assert_eq!(so[0], "A text");
                assert_eq!(so[1], "A text");
            }
        }
    }
}
