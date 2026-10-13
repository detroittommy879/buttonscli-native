fn main() {
    let mut root = std::path::PathBuf::from("web");
    let mut port = 8080_u16;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--web-root" => {
                root =
                    std::path::PathBuf::from(args.next().expect("--web-root requires a directory"));
            }
            "--port" => {
                port = args
                    .next()
                    .expect("--port requires a port")
                    .parse()
                    .expect("invalid port");
            }
            "--help" | "-h" => {
                println!("Usage: buttonscli-web [--port 8080] [--web-root web]\nLoopback preview with optional AI simulator. Reads the saved simulation model and OS credentials; writes no native settings.");
                return;
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }
    if port == 0 {
        eprintln!("Port must be between 1 and 65535.");
        std::process::exit(2);
    }
    if let Err(error) = buttonscli::web_preview::serve(root, port) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
