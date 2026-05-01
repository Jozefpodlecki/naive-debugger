use std::env;

mod fixtures;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let example = args.get(1).map(|s| s.as_str());
    
    let example = match example {
        Some(cli_value) => cli_value.to_string(),
        None => env::var("EXAMPLE").unwrap_or_else(|_| {
            eprintln!("Error: No example specified. Use --example <name> or set EXAMPLE environment variable");
            std::process::exit(1);
        }),
    };

    match example.as_str() {
        "--example=sleep" | "sleep" => {
            println!("Running sleep example");
            fixtures::sleep();
        }
        "--example=infinite" | "infinite" | "--example=loop" => {
            println!("Running infinite loop example");
            fixtures::infinite_loop();
        }
        "--example=infinite-sleep" | "infinite-sleep" | "--example=sleep-loop" => {
            println!("Running infinite sleep loop example");
            fixtures::infinite_sleep_loop();
        }
        _ => {
            println!("Usage: {} --example=<sleep|infinite>", args[0]);
            println!("Examples:");
            println!("  --example=sleep    - Sleep for 60 seconds");
            println!("  --example=infinite - Busy infinite loop");
        }
    }
}