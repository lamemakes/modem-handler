use dbus::Message;
use regex::{Captures, Regex};
use tokio::sync::mpsc::{Sender, Receiver};
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use std::{error::Error, io, sync::{Arc, Mutex}, time::Duration};

use crate::{constants::{ModemError, ModemErrorType, ResultCodes, SmsFormat, SmsMessage, SmsStatus, UnsolicitedResultCode}, utils::is_valid_imei};

pub struct GsmModem {
    port_path: &'static str,
    baud_rate: u32,
    timeout_duration: Duration,
    sender: Sender<String>,
    receiver: Arc<Mutex<Receiver<String>>>
}


/*

NOTE TO SELF

Maybe map URCs to mutexes to be able to handle states of things, pass that into recieve loop?

 - New texts
 - Ringing
 - Missed calls
 - etc

*/



impl GsmModem {
    pub fn new(port_path: &'static str, baud_rate: u32, timeout_duration: Duration) -> Self {
        let (tx, rx): (Sender<String>, Receiver<String>) = tokio::sync::mpsc::channel(200);
        let safe_rx = Arc::new(Mutex::new(rx));
        GsmModem { port_path: port_path, baud_rate: baud_rate, timeout_duration: timeout_duration, sender: tx, receiver: safe_rx}
    }

    async fn configure(&self) -> Result<(), Box<dyn Error>> {
        // Clear existing config on the modem
        self.write_data(String::from("ATZ\r"), None).await?;

        // Disable echo on the modem
        self.write_data(String::from("ATE0\r"), None).await?;

        // Set error codes to numeric
        self.write_data(String::from("AT+CMEE=1\r"), None).await?;

        // Set the PDU mode to text
        self.write_data(String::from("AT+CMGF=1\r"), None).await?;

        // Make all responses around SMS numbers/content hex that can be converted to UTF-16
        self.write_data(String::from("AT+CSCS=\"UCS2\"\r"), None).await?;

        Ok(())

    }

    fn get_port(&self) -> Result<SerialStream, Box<dyn Error>> {
        let mut port = tokio_serial::new(self.port_path, self.baud_rate).timeout(self.timeout_duration).open_native_async()?;
        port.set_exclusive(false)?;

        Ok(port)
    }

    pub async fn recieve_data_loop(&self) -> Result<(), Box<dyn Error>> {
        let mut port = self.get_port().unwrap();
        // All lines should end with '\r\n' except when sending a text message, which prompts with '\r\n'
        let line_end_re = Regex::new(r"(?:(\r\n)|(\r\n> ))$").unwrap();

        let mut string_buf = String::new();
        let mut serial_buf: Vec<u8> = vec![0; 1000];

        let urc_regex = UnsolicitedResultCode::get_regex_array();
        loop {
            use tokio::io::AsyncReadExt;
            match port.read(serial_buf.as_mut_slice()).await {
                Ok(t) => {
                    let mut urc_detected = false;
                    let encoded_str = str::from_utf8(&serial_buf[..t]).unwrap();
                    string_buf.push_str(encoded_str);
                    urc_regex.iter().for_each(|(urc, regex)| {
                        if !urc_detected {
                            if let Some(cap) = regex.captures(&string_buf) {
                                match urc {
                                    UnsolicitedResultCode::MissedCall => {
                                        println!("Missed call from {} at {}", cap.get(2).unwrap().as_str(), cap.get(1).unwrap().as_str())
                                    }
                                    _ => {
                                        println!("URC DETECTED!");
                                        println!("{:?}", string_buf);
                                    }
                                }
                                urc_detected = true;
                                string_buf.clear();
                            }
                        }
                    });
                    if line_end_re.is_match(&string_buf) && !urc_detected {
                        println!("MATCHED: {:?}", string_buf);
                        self.sender.send(string_buf.clone()).await.unwrap();
                        string_buf.clear();
                    }
                }
                Err(ref e) if e.kind() == io::ErrorKind::TimedOut => println!("timed out"),
                Err(e) => eprintln!("{:?}", e),
            }
        }
    }

    pub async fn write_data(&self, data: String, end_seqs: Option<(Regex, Regex)>) -> Result<String, Box<dyn Error>>{
        let default_seqs = (Regex::new(ResultCodes::Ok.as_regex_str())?, Regex::new(&ResultCodes::get_error_catchall())?);
        let end_seqs = end_seqs.unwrap_or(default_seqs);
        
        let (good_seq, error_seq) = end_seqs;

        let mut port = self.get_port()?;
        use tokio::io::AsyncWriteExt;
        match port.write(data.as_bytes()).await {
            Ok(_) => (),
            Err(_) => return Err("Failed to write data".into())
        }

        let mut loop_iter = 0;
        let mut recv_buf = String::new();

        loop {
            loop_iter += 1;
            if let Some(received) = self.receiver.lock().unwrap().recv().await {
                recv_buf.push_str(&received);
                if good_seq.is_match(&recv_buf) {
                    return Ok(recv_buf);
                } else if error_seq.is_match(&recv_buf) {
                    // If an error code was given, extract it
                    if let Some(capture) = Regex::new(ResultCodes::ErrorAndCode.as_regex_str())?.captures(&recv_buf) {
                        let Some(error_type) = (match &capture[1] {
                            "CME" => Some(ModemErrorType::CmeError),
                            "CMS" => Some(ModemErrorType::CmsError),
                            _ => None
                        }) else {
                            return Err("Failed to parse modem error type!".into())
                        };

                        let error_code: i32 = capture[2].parse()?;

                        // TODO: Definitely a better way to go about this
                        return Err(ModemError::new(error_type, error_code).as_string().into())
                    }

                    return Err("Generic error was returned".into())
                }
            };
            // TODO: Debug print, remove
            if loop_iter > 1000 {
                println!("WRITE DATA RECIEVE LOOP HAS RAN 1000 TIMES!")
            }
        }
    }

    pub async fn send_text_sms(&self, destination: &String, content: &String) -> Result<(), Box<dyn Error>> {
        self.set_sms_format(SmsFormat::Text).await?;
        let command = format!("AT+CMGS=\"{}\"\r", destination);
        let message = format!("{}\x1a", content);
        let expected_seqs = (Regex::new(ResultCodes::AwaitingInput.as_regex_str()).unwrap(), Regex::new(ResultCodes::get_error_catchall().as_str()).unwrap());
        self.write_data(command, Some(expected_seqs)).await?;
        self.write_data(message, None).await?;

        Ok(())
    }

    pub async fn get_imei(&self) -> Result<String, Box<dyn Error>> {
        let resp = self.write_data(String::from("AT+SIMEI?\r"), None).await?;

        let imei_captures: Result<regex::Captures<'_>, Box<dyn Error>> = Regex::new(r"\+SIMEI: (\d{15})")?.captures(&resp).ok_or_else(|| Err("Failed to parse IMEI!").unwrap());

        Ok(String::from(imei_captures?.get(1).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse IMEI!").unwrap())?.as_str()))

    }

    pub async fn set_imei(&self, intended_imei: String) -> Result<(), Box<dyn Error>> {
        if !is_valid_imei(&intended_imei) {
            return Err("IMEI is not valid!".into())
        }
        let command = format!("AT+SIMEI={}\r", intended_imei);
        self.write_data(command, None).await?;

        Ok(())
    }

    pub async fn get_sms_format(&self) -> Result<SmsFormat, Box<dyn Error>> {
        let resp = self.write_data(String::from("AT+CMGF?\r"), None).await?;

        let mode_captures: Result<regex::Captures<'_>, Box<dyn Error>> = Regex::new(r"\+CMGF: (1|0)")?.captures(&resp).ok_or_else(|| Err("Failed to retrieve SMS format!").unwrap());

        SmsFormat::try_from(String::from(mode_captures?.get(1).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to retrieve SMS format!").unwrap())?.as_str()))
    }

    pub async fn set_sms_format(&self, format: SmsFormat) -> Result<(), Box<dyn Error>> {
        let command = format!("AT+CMGF={}\r", Into::<u8>::into(format));
        self.write_data(command, None).await?;

        Ok(())
    }
    
    pub async fn get_signal_quality(&self) -> Result<(u8, u8), Box<dyn Error>> {
        // Helpful for understanding CSQ values: https://m2msupport.net/m2msupport/atcsq-signal-quality/

        let resp = self.write_data(String::from("AT+CSQ\r"), None).await?;

        let csq_captures = Regex::new(r"\+CSQ: (\d{0,3}),(\d{0,2})")?.captures(&resp).ok_or_else(|| Err::<String, Box<dyn Error>>("Failed to parse CSQ & Bit Error Rate!".into()).unwrap())?;

        let csq = csq_captures.get(1).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse CSQ value!").unwrap())?.as_str().parse::<u8>()?;

        // Seems like in most cases the bit error rate is unused (?) but include it anyway
        let ber = csq_captures.get(2).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse bit error rate!").unwrap())?.as_str().parse::<u8>()?;

        Ok((csq, ber))

    }

    pub async fn set_auto_timezone_updates_config(&self, enable: bool) -> Result<(), Box<dyn Error>> {
        let setting = if enable {"1"} else {"0"};

        let command = format!("AT+CTZU={}\r", setting);

        self.write_data(command, None).await?;

        Ok(())
    }

    pub async fn get_auto_timezone_updates_config(&self) -> Result<bool, Box<dyn Error>> {
        let resp = self.write_data(String::from("AT+CTZU?\r"), None).await?;

        let mode_captures: Result<regex::Captures<'_>, Box<dyn Error>> = Regex::new(r"\+CTZU: (1|0)")?.captures(&resp).ok_or_else(|| Err("Failed to retrieve SMS format!").unwrap());

        let mode = mode_captures?.get(1).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to retrieve SMS format!").unwrap())?.as_str();

        match mode {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err("Invalid timezone update config returned from modem".into())
        }
    }

    pub async fn get_sms_message(&self, mem_index: u32) -> Result<SmsMessage, Box<dyn Error>> {
        let command = format!("AT+CMGR={}\r", mem_index);
        let resp = self.write_data(command, None).await?;

        SmsMessage::from_cmgr(resp, mem_index)
    }

    pub async fn get_sms_messages(&self, status: SmsStatus) -> Result<Vec<SmsMessage>, Box<dyn Error>> {
        let command = format!("AT+CMGL=\"{}\"\r", status.as_str());
        let resp = self.write_data(command, None).await?;

        SmsMessage::from_cmgl(resp)
    }

}