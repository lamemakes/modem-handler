use std::error::Error;

use regex::Regex;

pub fn is_valid_imei(intended_imei: &String) -> bool {
    let numbers = intended_imei.split("").collect::<Vec<&str>>();
    let check_digit = numbers[numbers.len()-2].parse::<i32>().expect("Failed to convert string to int!");

    let mut sum: i32 = 0;

    for (index, str_number) in numbers[1..numbers.len()-2].iter().enumerate() {
        let number = str_number.parse::<i32>().expect("Failed to convert string to int!");

        if index % 2 != 0 {
            let mut new_number = number * 2;
            if new_number >= 10 { new_number -= 9}
            sum += new_number
        } else {
            sum += number
        }
    }

    // Run luhn algorithm to calculate the check digit
    check_digit == (10 - (sum % 10)) % 10
}

// Convert a hex string to u16 bytes
fn hex_to_bytes(s: &str) -> Option<Vec<u16>> {
    if s.len() % 2 == 0 {
        (0..s.len())
            .step_by(2)
            .map(|i| s.get(i..i + 2)
                      .and_then(|sub| u16::from_str_radix(sub, 16).ok()))
            .collect()
    } else {
        None
    }
}

pub fn hex_to_utf16(hex: &str) -> Result<String, Box<dyn Error>> {
    if let Some(hex_vec) = hex_to_bytes(hex) {
        return Ok(String::from_utf16(hex_vec.as_slice())?)
    } else {
        return Err("Failed to parse hex to UTF16".into())
    };
}

/// Converts the GSM given timestamp format to ISO 8601
pub fn timestamp_to_iso_8601(timestamp: &str) -> Result<String, Box<dyn Error>> {
    // Extracts each component via regex and indivdually pull them out, probably a more efficent way to do this
    let timestamp_re = Regex::new(r"(\d{2})/(\d{2})/(\d{2}),(\d{2}):(\d{2}):(\d{2})((?:-|\+)\d{0,3})?")?;

    let captures = timestamp_re.captures(&timestamp).ok_or_else(|| Err::<String, Box<dyn Error>>("Failed to parse timezone!".into()).unwrap())?;

    let year = captures.get(1).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse year value!").unwrap())?.as_str();

    let month = captures.get(2).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse month value!").unwrap())?.as_str();

    let day = captures.get(3).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse day value!").unwrap())?.as_str();

    let hour = captures.get(4).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse hour value!").unwrap())?.as_str();

    let min = captures.get(5).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse minute value!").unwrap())?.as_str();

    let sec = captures.get(6).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse second value!").unwrap())?.as_str();

    let tz = captures.get(7).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse timezone value!").unwrap())?.as_str().parse::<f32>()?/4_f32;

    let sign: char;

    if tz.trunc() < 0_f32 {
        sign = '-';
    } else {
        sign = '+';
    }

    let converted_tz = format!("{}{:02}:{:02}", sign, tz.abs().trunc(), tz.fract() * 60_f32);

    // I hate the year format that comes from the modem (`25`) but whatever
    let converted_stamp = format!("20{}-{}-{}T{}:{}:{}{}", year, month, day, hour, min, sec, converted_tz);

    Ok(converted_stamp)

}