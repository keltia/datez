use anyhow::Result;
use jiff::tz::TimeZone;

fn main() -> Result<()> {
    let tz = TimeZone::system();
    let name = tz.iana_name().unwrap_or("Local");
    println!("{}", name);
    Ok(())
}
