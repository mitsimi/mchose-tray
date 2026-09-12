use mchose_tray::{battery, icons};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--preview") {
        let path = args
            .get(1)
            .ok_or("Usage: mchose-check --preview FILE.png")?;
        icons::write_preview(std::path::Path::new(path))?;
        println!("Icon preview saved to {path}");
        return Ok(());
    }

    if !args.is_empty() {
        return Err("Usage: mchose-check [--preview FILE.png]".into());
    }

    let status = battery::read()?;
    println!("{}: {}", battery::MODEL, status.label());
    eprintln!("{status:?}");
    if !matches!(status, battery::Status::Battery { .. }) {
        std::process::exit(1);
    }
    Ok(())
}
