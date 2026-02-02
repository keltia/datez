//! `datez` is a small command-line utility to convert a time into
//! several timezones.
//!
//! ## usage
//!
//!     datez <time> <zone>...
//!
//! You should write the time in ISO 8601 / RFC 3339 format but
//! _without_ a UTC offset, and list as many tz database timezone names
//! as you want.
//!
//! The time is read using the first timezone; it is converted to UTC and
//! printed in UTC and in every timezone you listed, and in your local
//! timezone (if possible).
//!
//! The local timezone is discovered from the `TZ` environment variable
//! if that is set, or by an OS-specific mechanism; it isn't an error
//! if neither of those work, but you have to list your timezone
//! explicitly.
//!
//! `datez` relies on Jiff's cross-platform time zone discovery.

use anyhow::{anyhow, bail, Result};
use jiff::{
    civil::DateTime as CivilDateTime, fmt::strtime, tz::TimeZone, Timestamp,
};
use std::ffi::OsStr;

/// Try several formats for parsing time
///
/// Example:
/// ```
/// let ndt = parse_time(" 2021-07-21.16:00:00")?;
/// ```
fn parse_time(arg: &str) -> Result<CivilDateTime> {
    let fmts = [
        "%F.%T",
        "%FT%T",
        "%F %T",
        "%Y%m%d%H%M%S",
        "%Y%m%d.%H%M%S",
        "%Y%m%dT%H%M%S",
        "%Y%m%d %H%M%S",
    ];
    for fmt in fmts {
        if let Ok(time) = strtime::parse(fmt, arg).and_then(|t| t.to_datetime())
        {
            return Ok(time);
        }
    }
    bail!("time must be in RFC 3339 / ISO 8601 format, without a UTC offset");
}

/// Map an IANA TZDB timezone string into a Tz object
///
fn parse_tz(zone: &str) -> Result<TimeZone> {
    TimeZone::get(zone).map_err(|e| anyhow!("{}", e))
}

/// Validate the timezone
///
fn tz_ok(zone: &OsStr) -> Result<TimeZone> {
    let zone = zone.to_str().ok_or_else(|| anyhow!("not utf8"))?;
    parse_tz(zone)
}

/// Parse the time and set its timezone
///
fn get_time(time: &str, zone: &str, tz: &TimeZone) -> Result<Timestamp> {
    let civil = parse_time(time)?;
    tz.to_timestamp(civil).map_err(|e| {
        anyhow!("could not convert {} to {} timezone: {}", time, zone, e)
    })
}

/// Prints the specified time with its plain text timezone
///
fn print_time_tz(time: &Timestamp, zone: &str, tz: &TimeZone) -> Result<()> {
    let time = time.to_zoned(tz.clone());
    println!("{} ({})", time.strftime("%F.%T%z"), zone);
    Ok(())
}

/// Extracts the timezone before printing the result
///
fn print_time(time: &Timestamp, zone: &str) -> Result<()> {
    let tz = parse_tz(zone)?;
    print_time_tz(time, zone, &tz)?;
    Ok(())
}

/// Look for the local timezone
///
fn localzone() -> Result<TimeZone> {
    if let Some(zone) = std::env::var_os("TZ") {
        tz_ok(&zone)
    } else {
        Ok(TimeZone::system())
    }
}

fn tz_name(tz: &TimeZone) -> String {
    tz.iana_name().unwrap_or("Local").to_string()
}

/// Process the command line
///
fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
    let local_tz = localzone().ok();
    if let Some(zone) = &local_tz {
        if args.len() == 1 {
            let now = Timestamp::now();
            let now = now.to_zoned(zone.clone());
            args.push(now.strftime("%F.%T").to_string());
        }
    }
    if args.len() < 3 || args[1] == "-h" || args[1] == "--help" {
        bail!("usage: datez <datetime> <tz>...");
    }
    let time = get_time(&args[1], &args[2], &parse_tz(&args[2])?)?;
    print_time(&time, "UTC")?;
    for arg in args[2..].iter() {
        print_time(&time, arg)?;
    }
    if let Some(zone) = local_tz {
        print_time_tz(&time, &tz_name(&zone), &zone)?;
    }
    Ok(())
}

#[cfg(test)]

mod tests {
    use super::*;

    use rstest::rstest;

    #[test]
    fn test_localzone_from_env() {
        // this all needs to be one test function, because tests are run
        // in parallel on multiple threads, which is incompatible with
        // manipulating environment variables
        std::env::set_var("TZ", "Europe/Paris");
        let tz = localzone();
        assert!(tz.is_ok());
        assert_eq!(
            "Europe/Paris".to_string(),
            tz.unwrap().iana_name().unwrap()
        );
        std::env::remove_var("TZ");
    }

    #[rstest]
    #[case("")]
    #[case("bad")]
    #[case("30001300343433")]
    #[case("30001300 343433")]
    #[case("30001300.343433")]
    #[case("30001300T343433")]
    fn test_parsetime_nok(#[case] s: &str) {
        assert!(parse_time(s).is_err());
    }

    #[rstest]
    #[case("20211201213433")]
    #[case("20211202 213433")]
    #[case("20211203.213433")]
    #[case("20211204T213433")]
    fn test_parsetime_ok(#[case] s: &str) {
        assert!(parse_time(s).is_ok());
    }

    #[rstest]
    #[case("Europe/Paris")]
    #[case("Europe/London")]
    fn test_parse_tz_ok(#[case] s: &str) {
        assert!(parse_tz(s).is_ok());
    }

    #[rstest]
    #[case("Nowhere/None")]
    #[case("Europe/Marseille")]
    fn test_parse_tz_nok(#[case] s: &str) {
        assert!(parse_tz(s).is_err());
    }
}
