mod help;

use regex::Regex;
use std::process::{exit, Command};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const RESET: &'static str = "\u{1b}[0m";
const GREEN: &'static str = "\u{1b}[32m";
const CYAN: &'static str = "\u{1b}[36m";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        help::print_help();
        eprint!("Error: missing URL\n");
    }

    let url = &args[1];
    if url == "-h" || url == "--help" {
        help::print_help();
        exit(0);
    } else if url == "--version" {
        println!("httpstat {}", VERSION);
        exit(0);
    }

    let curl_args: Vec<String> = args[2..].to_vec();

    let exclude_options = vec![
        "-w",
        "--write-out",
        "-D",
        "--dump-header",
        "-o",
        "--output",
        "-s",
        "--silent",
    ];
    for opt in &exclude_options {
        if curl_args.contains(&opt.to_string()) {
            eprintln!("Error: {} is not allowed in extra curl args", opt);
            exit(1);
        }
    }

    let curl_format = r#"{
            "time_namelookup": %{time_namelookup},
            "time_connect": %{time_connect},
            "time_appconnect": %{time_appconnect},
            "time_pretransfer": %{time_pretransfer},
            "time_redirect": %{time_redirect},
            "time_starttransfer": %{time_starttransfer},
            "time_total": %{time_total},
            "speed_download": %{speed_download},
            "speed_upload": %{speed_upload},
            "remote_ip": "%{remote_ip}",
            "remote_port": "%{remote_port}",
            "local_ip": "%{local_ip}",
            "local_port": "%{local_port}"
        }"#;

    let output = Command::new("curl")
        .arg("-w")
        .arg(curl_format)
        .arg("-D")
        .arg("-") // Output headers to stdout
        .arg("-o")
        .arg("-") // Output body to stdout
        .arg("-s") // Silent mode to suppress progress meter
        .arg(url)
        .output()
        .expect("Failed to execute curl command");

    if !output.status.success() {
        eprintln!("curl error: {}", String::from_utf8_lossy(&output.stderr));
        exit(output.status.code().unwrap_or(1));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = stdout.split("\r\n\r\n").collect();

    if parts.len() > 1 {
        let headers = parts[0];
        let mut buffer = Vec::new();
        for (index, line) in headers.lines().enumerate() {
            match index {
                0 => {
                    let re = Regex::new("(.+?)/(.*)").unwrap();
                    buffer.push(String::new());
                    buffer.push(
                        re.replace(
                            line,
                            format!("{}$1{}/{}$2{}", GREEN, RESET, CYAN, RESET).as_str(),
                        )
                        .to_string(),
                    );
                }
                _ => {
                    let re = Regex::new("(.+?):(.*)").unwrap();
                    buffer.push(
                        re.replace(line, format!("$1:{}$2{}", CYAN, RESET).as_str())
                            .to_string(),
                    );
                }
            }
        }

        println!("{}", buffer.join("\n"))
    } else {
        todo!()
    }

    // Extract JSON metrics from stdout
    let json_start = stdout.rfind('{').unwrap();
    let json_end = stdout.rfind('}').unwrap();
    let json_str = &stdout[json_start..=json_end];

    if let Ok(metrics) = serde_json::from_str::<serde_json::Value>(json_str) {
        let time_namelookup = metrics["time_namelookup"].as_f64().unwrap_or(0.0) * 1000.0;
        let time_connect = metrics["time_connect"].as_f64().unwrap_or(0.0) * 1000.0;
        // let time_appconnect = metrics["time_appconnect"].as_f64().unwrap_or(0.0) * 1000.0;
        let time_pretransfer = metrics["time_pretransfer"].as_f64().unwrap_or(0.0) * 1000.0;
        let time_starttransfer = metrics["time_starttransfer"].as_f64().unwrap_or(0.0) * 1000.0;
        let time_total = metrics["time_total"].as_f64().unwrap_or(0.0) * 1000.0;

        let output = format!(
            "
            DNS Lookup   TCP Connection   SSL Handshake   Server Processing   Content Transfer
            [ {a0000} |     {a0001}    |    {a0002}    |      {a0003}      |      {a0004}     ]
                      |                |               |                   |                  |
                namelookup:{b0000}     |               |                   |                  |
                                    connect:{b0001}    |                   |                  |
                                                pretransfer:{b0002}        |                  |
                                                                  starttransfer:{b0003}       |
                                                                                          total:{b0004}
            ",
            // BOLD, RESET,
            a0000 = format_a(time_namelookup),
            a0001 = format_a(time_connect - time_namelookup),
            a0002 = format_a(time_pretransfer - time_connect),
            a0003 = format_a(time_starttransfer - time_pretransfer),
            a0004 = format_a(time_total - time_starttransfer),
            b0000 = format_b(time_namelookup),
            b0001 = format_b(time_connect),
            b0002 = format_b(time_pretransfer),
            b0003 = format_b(time_starttransfer),
            b0004 = format_b(time_total)
        );

        println!("{}", output);
    } else {
        eprintln!("Failed to parse timing metrics.");
    }
}

fn format_a(x: f64) -> String {
    format!("{}{:^7}{}", CYAN, format!("{:.0}ms", x), RESET)
}

fn format_b(x: f64) -> String {
    format!("{}{:<7}{}", CYAN, format!("{:.0}ms", x), RESET)
}
