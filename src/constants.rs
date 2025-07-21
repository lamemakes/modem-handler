use std::{collections::HashMap, error::Error, fmt};

use chrono::{format::Fixed, DateTime, FixedOffset, Utc};
use regex::Regex;

use crate::utils::{hex_to_utf16, timestamp_to_iso_8601};


impl Into<u8> for SmsStatus {
    /// Used only for PDU mode
    fn into(self) -> u8 {
        match self {
            SmsStatus::ReceivedUnread => 0,
            SmsStatus::ReceivedRead => 1,
            SmsStatus::StoredUnsent => 2,
            SmsStatus::StoredSent => 3,
            SmsStatus::All => 4
        }
    }
}


/// Formats for SMS modes
pub enum SmsFormat {
    ProtocolDataUnit,
    Text
}

impl Into<u8> for SmsFormat {
    // Convert from the given 0/1 into the enum
    fn into(self) -> u8 {
        match self {
            SmsFormat::ProtocolDataUnit => 0,
            SmsFormat::Text => 1
        }
    }
}

impl TryFrom<String> for SmsFormat {
    type Error = Box<dyn Error>;

    /// Take a string that is a "1" or "0" and convert it into the SmsFormat enum
    fn try_from(mode: String) -> Result<SmsFormat, Box<dyn Error>> {
        let mode_int: isize = mode.trim().parse()?;
        match mode_int {
            0 => Ok(SmsFormat::ProtocolDataUnit),
            1 => Ok(SmsFormat::Text),
            _ => Err("Failed to parse given SMS Format!".into())
        }
    }
}

pub enum SmsStatus {
    ReceivedUnread,
    ReceivedRead,
    StoredUnsent,
    StoredSent,
    All
}

impl SmsStatus {
    /// Used only for text mode
    pub fn as_str(&self) -> &'static str {
        match self {
            SmsStatus::ReceivedUnread => "REC UNREAD",
            SmsStatus::ReceivedRead => "REC READ",
            SmsStatus::StoredUnsent => "STO UNSENT",
            SmsStatus::StoredSent => "STO SENT",
            SmsStatus::All => "ALL"
        }
    }

    pub fn try_from_text_status(&self, status: String) -> Result<SmsStatus, Box<dyn Error>> {
        match status.as_str() {
            "REC UNREAD" => Ok(SmsStatus::ReceivedUnread),
            "REC READ" => Ok(SmsStatus::ReceivedRead),
            "STO UNSENT" => Ok(SmsStatus::StoredUnsent),
            "STO SENT" => Ok(SmsStatus::StoredSent),
            "ALL" => Ok(SmsStatus::All),
            _ => Err("Failed to parse text SMS status!".into())
        }
    }

    pub fn try_from_pdu_status(&self, status: u8) -> Result<SmsStatus, Box<dyn Error>> {
        match status {
            0 => Ok(SmsStatus::ReceivedUnread),
            1 => Ok(SmsStatus::ReceivedRead),
            2 => Ok(SmsStatus::StoredUnsent),
            3 => Ok(SmsStatus::StoredSent),
            4 => Ok(SmsStatus::All),
            _ => Err("Failed to parse PDU SMS status!".into())
        }
    }
}

// impl Into<u8> for SmsStatus {
//     /// Used only for PDU mode
//     fn into(self) -> u8 {
//         match self {
//             SmsStatus::ReceivedUnread => 0,
//             SmsStatus::ReceivedRead => 1,
//             SmsStatus::StoredUnsent => 2,
//             SmsStatus::StoredSent => 3,
//             SmsStatus::All => 4
//         }
//     }
// }


#[derive(Debug)]
pub struct SmsMessage {
    mem_index: u32,
    address: String,
    content: String,
    timestamp: DateTime<FixedOffset>
}

impl SmsMessage {
    /// Takes the modem output of AT+CMGR (getting a single message) and returns a SmsMessages struct
    pub fn from_cmgr(raw_string: String, mem_index: u32) -> Result<SmsMessage, Box<dyn Error>> {
        let msg_captures = Regex::new(r#"\+CMGR: "([A-Z ]*)","([0-9A-F]*)","","(\d{2}/\d{2}/\d{2},\d{2}:\d{2}:\d{2}-\d{0,3})"\r\n([0-9A-F]*)\r\n\r\nOK\r\n"#)?.captures(&raw_string).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse SMS message!")).unwrap();

        let address = hex_to_utf16(msg_captures.get(2).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse phone number in SMS message!").unwrap())?.as_str())?;

        let timestamp_iso8601 = timestamp_to_iso_8601(msg_captures.get(3).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse message timezone in SMS message!").unwrap())?.as_str())?;

        let timestamp = DateTime::parse_from_rfc3339(&timestamp_iso8601)?;

        let content = hex_to_utf16(msg_captures.get(4).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse message content in SMS message!").unwrap())?.as_str())?;

        Ok(SmsMessage { mem_index: mem_index, address: address, content: content, timestamp: timestamp })
        
    }

    /// Takes the modem output of AT+CMGL (listing of multiple messages) and returns a vec of SmsMessages
    pub fn from_cmgl(raw_string: String) -> Result<Vec<SmsMessage>, Box<dyn Error>> {
        let msg_regex= Regex::new(r#"\+CMGL: (\d{0,3}),"[A-Z ]*","([0-9A-F]*)","","(\d{2}/\d{2}/\d{2},\d{2}:\d{2}:\d{2}-\d{0,3})"\r\n([0-9A-F]*)\r\n"#)?;

        let msg_captures = msg_regex.captures_iter(&raw_string);

        let mut messages: Vec<SmsMessage> = Vec::new();
        for msg_capture in msg_captures {
            let index = msg_capture.get(1).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse message memory index!").unwrap())?.as_str().parse::<u32>()?;

            let address = hex_to_utf16(msg_capture.get(2).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse phone number in SMS message!").unwrap())?.as_str())?;

            let timestamp_iso8601 = timestamp_to_iso_8601(msg_capture.get(3).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse message timezone in SMS message!").unwrap())?.as_str())?;

            let timestamp = DateTime::parse_from_rfc3339(&timestamp_iso8601)?;

            let content = hex_to_utf16(msg_capture.get(4).ok_or_else(|| Err::<Box<dyn std::error::Error>, &str>("Failed to parse message content in SMS message!").unwrap())?.as_str())?;

            messages.push(SmsMessage { mem_index: index, address: address, content: content, timestamp: timestamp });
        }

        Ok(messages)
    }

    /// Returns the message memory index
    pub fn memory_index(&self) -> u32 {
        self.mem_index
    }

    /// Returns the message's associated address (phone number)
    pub fn address(&self) -> String {
        self.address.clone()
    }

    /// Returns the message content
    pub fn content(&self) -> String {
        self.content.clone()
    }

    /// Returns the message timestamp
    pub fn timestamp(&self) -> DateTime<FixedOffset> {
        self.timestamp
    }
}

impl fmt::Display for SmsMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Memory: {}\nAddress: {}\nContent: \"{}\"\nTimestamp: {}", self.mem_index, self.address, self.content, self.timestamp)
    }
}


pub enum ResultCodes {
    Ok,
    Error,
    ErrorAndCode,
    AwaitingInput,
}

impl ResultCodes {
    pub fn as_regex_str(&self) -> &'static str {
        match self {
            ResultCodes::Ok => r"\r\nOK\r\n",
            ResultCodes::Error => r"\r\nERROR\r\n",
            // Capture both CMS & CME error codes - see "3.3 Summary of CME ERROR codes" & "3.4 Summary of CMS ERROR codes"
            ResultCodes::ErrorAndCode => r"\r\n\+(CM(?:E|S)) ERROR: (\d{1,3})\r\n",
            ResultCodes::AwaitingInput => r"\r\n> ",
        }
    }

    /// Concats the regex strs for both generic errors and typed errors
    /// 
    /// Still handles captures for type/code if typed error is thrown (ie. CME or CMS)
    /// *concating regex prob isn't a greaaaaaat idea but this works for now*
    pub fn get_error_catchall() -> String {
        format!("({})|({})", ResultCodes::Error.as_regex_str(), ResultCodes::ErrorAndCode.as_regex_str())
    }
}

/// Result codes that are unsolicited and happen async
#[derive(Clone, Copy)]
pub enum UnsolicitedResultCode {
    /// The modem is ready to begin taking commands
    Ready,

    /// A new SMS message has been received
    CMTI,

    /// Incoming call
    Ring,

    /// A call was missed
    MissedCall,

    /// The carrier is unavailable
    NoCarrier,

    /// A voice call has started
    VoiceCallBegin,

    /// A voice call has ended
    VoiceCallEnd,

    /// User's timezone has changed
    TimeZoneChange,

    /// SMS storage is full and needs to be cleared
    SmsFull,
}

impl UnsolicitedResultCode {
    /// Get the proper regex str for the corresponding URC
    /// 
    /// If the URC returns data, it will (mostly) be captured by the regex str
    pub fn as_regex_str(&self) -> &'static str {
        match self {
            UnsolicitedResultCode::Ready => r"RDY\r\n",
            // Not entire sure what this digit is, memory location?
            // TODO: find what digit this is
            UnsolicitedResultCode::CMTI => r#"\+CMTI: "SM",(\d{1,3})\r\n"#,
            UnsolicitedResultCode::Ring => r"RING\r\r\n",
            // Captures (1) the time the call was missed and (2) the number that called
            // Time format looks like it's 24H but still includes AM/PM which is weird
            UnsolicitedResultCode::MissedCall => r"MISSED_CALL: (\d{2}:\d{2}[AP]M) (.*)\r\n",
            UnsolicitedResultCode::NoCarrier => r"NO CARRIER\r\r\n",
            UnsolicitedResultCode::VoiceCallBegin => r"VOICE CALL: BEGIN\r\n",
            // Captures the call time (in the format of HHMMSS)
            UnsolicitedResultCode::VoiceCallEnd => r"VOICE CALL: END: (\d{6})",
            // Will only extract the timezone digit, ignores other data
            // TODO: fully implement
            UnsolicitedResultCode::TimeZoneChange => r"\r\n\+CTZV: (\d+)(?:,.*)?\r\n",
            UnsolicitedResultCode::SmsFull => r"\r\n+SMS FULL\r\n"
        }
    }

    /// Get an array containing the regex structs of all the URCs
    /// 
    /// TODO: Maybe implement strum rather than doing this manually?
    pub fn get_regex_array() -> Vec<(UnsolicitedResultCode, Regex)> {
        [
            UnsolicitedResultCode::Ready,
            UnsolicitedResultCode::CMTI,
            UnsolicitedResultCode::Ring,
            UnsolicitedResultCode::MissedCall,
            UnsolicitedResultCode::NoCarrier,
            UnsolicitedResultCode::VoiceCallBegin,
            UnsolicitedResultCode::VoiceCallEnd,
            UnsolicitedResultCode::TimeZoneChange,
            UnsolicitedResultCode::SmsFull,
        ].iter().map(|&x| (x, Regex::new(x.as_regex_str()).unwrap())).collect()

        
    }
}

/// Modem Error Types
pub enum ModemErrorType {
    CmeError,
    CmsError
}

impl ModemErrorType {
    /// Returns the string format of the modem error types
    pub fn as_str(&self) -> &'static str {
        match self {
            ModemErrorType::CmeError => "CME",
            ModemErrorType::CmsError => "CMS"
        }
    }
}

/// An error type for errors returned by the modem
pub struct ModemError {
    e_type: ModemErrorType,
    code: i32,
    text: String
}

impl ModemError {
    /// Create a new error from an error type & code
    /// 
    /// Strings pulled from https://www.smssolutions.net/tutorials/gsm/gsmerrorcodes/
    pub fn new(e_type: ModemErrorType, code: i32) -> ModemError {
        let cme_error_codes = HashMap::from([
            (0, "Phone failure"),
            (1, "No connection to phone"),
            (2, "Phone adapter link reserved"),
            (3, "Operation not allowed"),
            (4, "Operation not supported"),
            (5, "PH_SIM PIN required"),
            (6, "PH_FSIM PIN required"),
            (7, "PH_FSIM PUK required"),
            (10, "SIM not inserted"),
            (11, "SIM PIN required"),
            (12, "SIM PUK required"),
            (13, "SIM failure"),
            (14, "SIM busy"),
            (15, "SIM wrong"),
            (16, "Incorrect password"),
            (17, "SIM PIN2 required"),
            (18, "SIM PUK2 required"),
            (20, "Memory full"),
            (21, "Invalid index"),
            (22, "Not found"),
            (23, "Memory failure"),
            (24, "Text string too long"),
            (25, "Invalid characters in text string"),
            (26, "Dial string too long"),
            (27, "Invalid characters in dial string"),
            (30, "No network service"),
            (31, "Network timeout"),
            (32, "Network not allowed, emergency calls only"),
            (40, "Network personalization PIN required"),
            (41, "Network personalization PUK required"),
            (42, "Network subset personalization PIN required"),
            (43, "Network subset personalization PUK required"),
            (44, "Service provider personalization PIN required"),
            (45, "Service provider personalization PUK required"),
            (46, "Corporate personalization PIN required"),
            (47, "Corporate personalization PUK required"),
            (48, "PH-SIM PUK required"),
            (100, "Unknown error"),
            (103, "Illegal MS"),
            (106, "Illegal ME"),
            (107, "GPRS services not allowed"),
            (111, "PLMN not allowed"),
            (112, "Location area not allowed"),
            (113, "Roaming not allowed in this location area"),
            (126, "Operation temporary not allowed"),
            (132, "Service operation not supported"),
            (133, "Requested service option not subscribed"),
            (134, "Service option temporary out of order"),
            (148, "Unspecified GPRS error"),
            (149, "PDP authentication failure"),
            (150, "Invalid mobile class"),
            (256, "Operation temporarily not allowed"),
            (257, "Call barred"),
            (258, "Phone is busy"),
            (259, "User abort"),
            (260, "Invalid dial string"),
            (261, "SS not executed"),
            (262, "SIM Blocked"),
            (263, "Invalid block"),
            (772, "SIM powered down "),
        ]);

        let cms_error_codes = HashMap::from([
            (1, "Unassigned number"),
            (8, "Operator determined barring"),
            (10, "Call bared"),
            (21, "Short message transfer rejected"),
            (27, "Destination out of service"),
            (28, "Unindentified subscriber"),
            (29, "Facility rejected"),
            (30, "Unknown subscriber"),
            (38, "Network out of order"),
            (41, "Temporary failure"),
            (42, "Congestion"),
            (47, "Recources unavailable"),
            (50, "Requested facility not subscribed"),
            (69, "Requested facility not implemented"),
            (81, "Invalid short message transfer reference value"),
            (95, "Invalid message unspecified"),
            (96, "Invalid mandatory information"),
            (97, "Message type non existent or not implemented"),
            (98, "Message not compatible with short message protocol"),
            (99, "Information element non-existent or not implemente"),
            (111, "Protocol error, unspecified"),
            (127, "Internetworking , unspecified"),
            (128, "Telematic internetworking not supported"),
            (129, "Short message type 0 not supported"),
            (130, "Cannot replace short message"),
            (143, "Unspecified TP-PID error"),
            (144, "Data code scheme not supported"),
            (145, "Message class not supported"),
            (159, "Unspecified TP-DCS error"),
            (160, "Command cannot be actioned"),
            (161, "Command unsupported"),
            (175, "Unspecified TP-Command error"),
            (176, "TPDU not supported"),
            (192, "SC busy"),
            (193, "No SC subscription"),
            (194, "SC System failure"),
            (195, "Invalid SME address"),
            (196, "Destination SME barred"),
            (197, "SM Rejected-Duplicate SM"),
            (198, "TP-VPF not supported"),
            (199, "TP-VP not supported"),
            (208, "D0 SIM SMS Storage full"),
            (209, "No SMS Storage capability in SIM"),
            (210, "Error in MS"),
            (211, "Memory capacity exceeded"),
            (212, "Sim application toolkit busy"),
            (213, "SIM data download error"),
            (255, "Unspecified error cause"),
            (300, "ME Failure"),
            (301, "SMS service of ME reserved"),
            (302, "Operation not allowed"),
            (303, "Operation not supported"),
            (304, "Invalid PDU mode parameter"),
            (305, "Invalid Text mode parameter"),
            (310, "SIM not inserted"),
            (311, "SIM PIN required"),
            (312, "PH-SIM PIN required"),
            (313, "SIM failure"),
            (314, "SIM busy"),
            (315, "SIM wrong"),
            (316, "SIM PUK required"),
            (317, "SIM PIN2 required"),
            (318, "SIM PUK2 required"),
            (320, "Memory failure"),
            (321, "Invalid memory index"),
            (322, "Memory full"),
            (330, "SMSC address unknown"),
            (331, "No network service"),
            (332, "Network timeout"),
            (340, "No +CNMA expected"),
            (500, "Unknown error"),
            (512, "User abort"),
            (513, "Unable to store"),
            (514, "Invalid Status"),
            (515, "Device busy or Invalid Character in string"),
            (516, "Invalid length"),
            (517, "Invalid character in PDU"),
            (518, "Invalid parameter"),
            (519, "Invalid length or character"),
            (520, "Invalid character in text"),
            (521, "Timer expired"),
            (522, "Operation temporary not allowed"),
            (532, "SIM not ready"),
            (534, "Cell Broadcast error unknown"),
            (535, "Protocol stack busy"),
            (538, "Invalid parameter "),
        ]);


        let text = match e_type {
            ModemErrorType::CmsError => cms_error_codes.get(&code).map(|e| String::from(*e)),
            ModemErrorType::CmeError => cme_error_codes.get(&code).map(|e| String::from(*e))
        }.unwrap_or(String::from("Unrecognized error"));

        ModemError { e_type: e_type, code: code, text: text }
    }

    /// Get the error as a String
    pub fn as_string(&self) -> String {
        return format!("{} Error: {}", self.e_type.as_str(), self.text)
    } 
}
