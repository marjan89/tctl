use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: tctl <command> [options]");
        eprintln!("Commands: run, doctor");
        process::exit(1);
    }

    match args[1].as_str() {
        "run" => todo!("tctl run"),
        "doctor" => todo!("tctl doctor"),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            process::exit(1);
        }
    }
}
