use std::time::Duration;

use async_modem::gsm_modem::GsmModem;
use tokio_serial::SerialPortBuilderExt;
use tokio::sync::mpsc::{Receiver, Sender};
use async_modem::constants::{SmsFormat, SmsStatus};

use async_modem::utils::timestamp_to_iso_8601;


async fn dummy_send(modem: &GsmModem) {

    // let command1 = String::from("AT+CTZU=1\r");
    // let command2 = String::from("AT+CSQ\r");

    //modem.write_data(String::from("AT+CMGF=1\r"), None).await.unwrap();

    // modem.send_text_sms(&String::from("13153352552"), &String::from("they wouldn't be your friend\nif it wasn't worth it")).await.unwrap();

    // clear all messages:
    // modem.write_data(String::from("AT+CMGD=0,4\r"), None).await.unwrap();
    // modem.set_sms_format(SmsFormat::Text).await.unwrap();AT+CMEE=1\r
    //println!("{}", modem.write_data(String::from("AT+CMEE=1\r"), None).await.unwrap());
    //println!("{}", modem.write_data(String::from("AT+CMGR=56\r"), None).await.unwrap());
    // println!("{}", modem.write_data(String::from("AT+COPS?\r"), None).await.unwrap());
    // modem.set_auto_timezone_updates_config(true).await.unwrap();
    // println!("{}", modem.get_auto_timezone_updates_config().await.unwrap());
    // modem.set_auto_timezone_updates_config(false).await.unwrap();
    // println!("{}", modem.get_auto_timezone_updates_config().await.unwrap());
    
    //println!("{}", modem.write_data(String::from("AT+CSCS=\"UCS2\"\r"), None).await.unwrap());

    // let messages = modem.get_sms_messages(SmsStatus::All).await.unwrap();
    // for message in messages {
    //     println!("{}", message);
    //     println!("-----------------------------");
    // }

    let message = modem.get_sms_message(9).await.unwrap();


    println!("{}", message)
    // modem.set_sms_format(SmsFormat::Text).await.unwrap();

    // let resp = modem.get_sms_format().await.unwrap();
    // match resp {
    //     SmsFormat::Text => println!("TEXT!"),
    //     SmsFormat::ProtocolDataUnit => println!("PDU MODE!")
    // }

    // println!("Signal Quality: {:?}", modem.get_signal_quality().await.unwrap());
}


#[tokio::main]
async fn main() {
    println!("STARTING");

    //let port = tokio_serial::new("/dev/ttyS0", 115_200).timeout(Duration::from_millis(10)).open_native_async().expect("Failed to open port");

    //let mut modem = GsmModem::new(port);

    // tokio::spawn(async move {
    //     let mut port = tokio_serial::new("/dev/ttyS0", 115_200).timeout(Duration::from_millis(10)).open_native_async().expect("Failed to open port");
    //     port.set_exclusive(false).unwrap();
    //     GsmModem::recieve_data(port).await;
    // });

    let port_path = "/dev/ttyS0";

    let mut modem = GsmModem::new(port_path, 115_200, Duration::from_millis(10));

    let res = tokio::join!(
        modem.recieve_data_loop(),
        dummy_send(&modem)
    );

    res.1;

    // let mut port = tokio_serial::new("/dev/ttyS0", 115_200).timeout(Duration::from_millis(10)).open_native_async().expect("Failed to open port");
    // port.set_exclusive(false).unwrap();

    // GsmModem::write_data(port, &String::from("ATI\r")).await.unwrap();
}
