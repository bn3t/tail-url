# tail-url - Tail text from an URL [![Build Status](https://travis-ci.org/bn3t/tail-url.svg?branch=master)](https://travis-ci.org/bn3t/tail-url)

This is a utility to tail text from an HTTP endpoint. It works similarly to doing tail -f with a local file. Typically, the source should be a growing text file (like a log file) served by a server supporting the Range header in http.

## Usage

```
$ tail-url -h
Usage:
    tail-url [OPTIONS] URL

Tail text coming from an URL

positional arguments:
  url                   URL to tail

optional arguments:
  -h,--help             show this help message and exit
  -t                    Starting tail offset in bytes
```

### Example

The following example will tail from the URL http://localhost:8080/logfile.

```
tail-url http://localhost:8080/logfile
```

## Requirements

For the utility to work, the HTTP server needs to support the Range header with a range of bytes in the GET request. The tool checks it by calling the HEAD method on the resource and checking for the AcceptRanges header.
