//! This module parses packets as AGI Requests or Responses.
use std::{collections::HashMap, error::Error, fmt::Display, path::PathBuf, str::FromStr};

use tracing::Level;
use url::Url;

/// The common Error type for all problems related to parsing.
#[derive(Debug, Eq, PartialEq)]
pub enum AGIParseError {
    /// A line contained no value
    NoValue(String),
    /// The agi_priority line was not parsable
    PriorityUnparsable(String),
    /// The agi_threadid line was not parsable
    ThreadIdUnparsable(String),
    /// the agi_enhanced line was not parsable
    EnhancedUnparsable(String),
    /// An unknown argument was found
    UnknownArg(String),
    /// A custom arg (agi_arg_n) hat a number (n) that was not parsable
    CustomArgNumberUnparsable(String),
    /// The same custom arg was defined more then once
    DuplicateCustomArg(String),
    /// A variable that is required was missing
    VariableMissing(String),
    /// A packet is supposed to be a Status Response, but contains no status code
    NoStatusCode(String),
    /// A status code was found but it was not parsable as a number
    StatusCodeUnparsable(String),
    /// A Status Response did not contain a result
    NoResult(String),
    /// The result of a Status Response was supposed to be parsable into some known type, but it
    /// was not parsable.
    ResultUnparsable(String),
    /// An empty packet was encountered
    NoBytes,
    /// A packet contained non-utf8 codepoints
    NotUtf8,
    /// A status line was encountered, but it was not properly ended by a `\n`
    StatusWithoutNewline,
    /// A status was parsable, but it is not known
    StatusDoesNotExist(u16),
    /// It was impossible to read bytes from a TcpStream
    ReadError,
    /// There was a network start line sent after another message
    NetworkStartAfterOtherMessage,
}
impl Display for AGIParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NoValue(x) => {
                write!(f, "The line {x} contained no value.")
            }
            Self::PriorityUnparsable(x) => {
                write!(f, "The value {x} is not parsable as priority.")
            }
            Self::ThreadIdUnparsable(x) => {
                write!(f, "The value {x} is not parsable as thread ID.")
            }
            Self::EnhancedUnparsable(x) => {
                write!(f, "The value {x} is not parsable as enhanced status.")
            }
            Self::UnknownArg(x) => {
                write!(f, "The argument {x} is not known.")
            }
            Self::CustomArgNumberUnparsable(x) => {
                write!(f, "The argument {x} has no parsable custom arg number.")
            }
            Self::DuplicateCustomArg(x) => {
                write!(f, "The argument {x} was passed multiple times.")
            }
            Self::VariableMissing(x) => {
                write!(f, "The argument {x} is required but was not passed.")
            }
            Self::NoStatusCode(x) => {
                write!(f, "The status line {x} has no status code.")
            }
            Self::StatusCodeUnparsable(x) => {
                write!(f, "The status code in status line {x} is not parsable.")
            }
            Self::NoResult(x) => {
                write!(f, "The status line {x} has no result.")
            }
            Self::ResultUnparsable(x) => {
                write!(f, "The result in status line {x} is not parsable.")
            }
            Self::NoBytes => {
                write!(f, "There are no bytes to read.")
            }
            Self::NotUtf8 => {
                write!(f, "The input is not utf8")
            }
            Self::StatusDoesNotExist(x) => {
                write!(f, "The status code {x} does not exist")
            }
            Self::StatusWithoutNewline => {
                write!(
                    f,
                    "A status message was contained in a buffer without a newline"
                )
            }
            Self::ReadError => {
                write!(f, "Unable to read literal bytes from TcpStream")
            }
            Self::NetworkStartAfterOtherMessage => {
                write!(
                    f,
                    "There was a line `agi_network: yes` after another message."
                )
            }
        }
    }
}
impl Error for AGIParseError {}

/// This types contains the different possible Status, *before* they are parsed into the specific
/// response we expected due to the sent command.
/// The response will be further parsed down to an [`AGIResponse`](crate::command::AGIResponse)
/// once we know to which Request this response is an answer.
#[derive(Debug, PartialEq, Eq)]
pub enum AGIStatusGeneric {
    /// 200
    Ok(String, Option<String>),
    // 510
    Invalid,
    // 511
    DeadChannel,
    // 520
    EndUsage,
}
impl std::fmt::Display for AGIStatusGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Ok(result, op_data) => match op_data {
                Some(x) => {
                    write!(f, "200 result={result} {x}")
                }
                None => {
                    write!(f, "200 result={result}")
                }
            },
            Self::Invalid => {
                write!(f, "510")
            }
            Self::DeadChannel => {
                write!(f, "511")
            }
            Self::EndUsage => {
                write!(f, "520")
            }
        }
    }
}
impl FromStr for AGIStatusGeneric {
    type Err = AGIParseError;
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // line format is
        // 200 result=some_result [some_operational_data]
        let mut splitline = s.trim_end().split(' ');
        let code = splitline
            .next()
            .ok_or(AGIParseError::NoStatusCode(s.to_owned()))?
            .parse::<u16>()
            .map_err(|_| AGIParseError::StatusCodeUnparsable(s.to_owned()))?;
        let result_part = splitline
            .next()
            .ok_or(AGIParseError::NoResult(s.to_owned()))?;
        if !result_part.starts_with("result=") {
            return Err(AGIParseError::ResultUnparsable(s.to_owned()));
        }
        let result = result_part[7..].to_owned();
        let operational_data = splitline.next().map(|x| x.to_owned());
        match code {
            200 => Ok(AGIStatusGeneric::Ok(result, operational_data)),
            510 => Ok(AGIStatusGeneric::Invalid),
            511 => Ok(AGIStatusGeneric::DeadChannel),
            520 => Ok(AGIStatusGeneric::EndUsage),
            x => Err(AGIParseError::StatusDoesNotExist(x)),
        }
    }
}

/// The different AGI Request types we may encounter in agi_request.
/// NOTE: only FastAGI is supported.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum AGIRequestType {
    File(PathBuf),
    FastAGI(Url),
}
impl FromStr for AGIRequestType {
    type Err = AGIParseError;
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // try to parse as URI
        if let Ok(parsed_uri) = s.parse::<Url>() {
            return Ok(Self::FastAGI(parsed_uri));
        } else {
            // then try to parse as path
            return Ok(Self::File(PathBuf::from(s)));
        }
    }
}
impl Display for AGIRequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::File(x) => {
                write!(f, "{x:?}")
            }
            Self::FastAGI(x) => {
                write!(f, "{x}")
            }
        }
    }
}

/// Parse the value in the agi_enhanced line
fn enhanced_status(input: &str) -> Result<bool, AGIParseError> {
    if input == "0.0" {
        return Ok(false);
    }
    if input == "1.0" {
        return Ok(true);
    }
    return Err(AGIParseError::EnhancedUnparsable(input.to_owned()));
}

/// The VariableDump (i.e. an AGI request). This is the second packet asterisk sends, after an
/// agi_network: yes has been sent to initiate the session.
/// The variables are in 1-1 map to the variables asterisk sends.
#[derive(Debug, PartialEq, Eq)]
pub struct AGIVariableDump {
    pub network_script: String,
    pub request: AGIRequestType,
    pub channel: String,
    pub language: String,
    pub channel_type: String,
    pub uniqueid: String,
    pub version: String,
    pub callerid: String,
    pub calleridname: String,
    pub callingpres: String,
    pub callingani2: String,
    pub callington: String,
    pub callingtns: String,
    pub dnid: String,
    pub rdnis: String,
    pub context: String,
    pub extension: String,
    pub priority: u16,
    pub enhanced: bool,
    pub accountcode: String,
    pub threadid: u64,
    /// All arguments of the form `agi_arg_n: value` are collected here (in this case as an entry
    /// (n)=>value )
    pub custom_args: HashMap<u8, String>,
}
impl Display for AGIVariableDump {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "agi_network_script: {}\n", self.network_script)?;
        write!(f, "agi_request: {}\n", self.request)?;
        write!(f, "agi_channel: {}\n", self.channel)?;
        write!(f, "agi_language: {}\n", self.language)?;
        write!(f, "agi_channel_type: {}\n", self.channel_type)?;
        write!(f, "agi_uniqueid: {}\n", self.uniqueid)?;
        write!(f, "agi_version: {}\n", self.version)?;
        write!(f, "agi_callerid: {}\n", self.callerid)?;
        write!(f, "agi_calleridname: {}\n", self.calleridname)?;
        write!(f, "agi_callingpres: {}\n", self.callingpres)?;
        write!(f, "agi_callingani2: {}\n", self.callingani2)?;
        write!(f, "agi_callington: {}\n", self.callington)?;
        write!(f, "agi_callingtns: {}\n", self.callingtns)?;
        write!(f, "agi_dnid: {}\n", self.dnid)?;
        write!(f, "agi_rdnis: {}\n", self.rdnis)?;
        write!(f, "agi_context: {}\n", self.context)?;
        write!(f, "agi_extension: {}\n", self.extension)?;
        write!(f, "agi_priority: {}\n", self.priority)?;
        write!(f, "agi_enhanced: {}\n", self.enhanced)?;
        write!(f, "agi_accountcode: {}\n", self.accountcode)?;
        write!(f, "agi_threadid: {}\n", self.threadid)?;
        for idx in 0..self.custom_args.len() {
            write!(
                f,
                "agi_arg_{}: {}\n",
                idx,
                self.custom_args
                    .get(&(idx as u8))
                    .expect("custom_args should contain consecutive u8s as key")
            )?;
        }
        Ok(())
    }
}
impl FromStr for AGIVariableDump {
    type Err = AGIParseError;
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut network_script: Option<String> = None;
        let mut request: Option<AGIRequestType> = None;
        let mut channel: Option<String> = None;
        let mut language: Option<String> = None;
        let mut channel_type: Option<String> = None;
        let mut uniqueid: Option<String> = None;
        let mut version: Option<String> = None;
        let mut callerid: Option<String> = None;
        let mut calleridname: Option<String> = None;
        let mut callingpres: Option<String> = None;
        let mut callingani2: Option<String> = None;
        let mut callington: Option<String> = None;
        let mut callingtns: Option<String> = None;
        let mut dnid: Option<String> = None;
        let mut rdnis: Option<String> = None;
        let mut context: Option<String> = None;
        let mut extension: Option<String> = None;
        let mut priority: Option<u16> = None;
        let mut enhanced: Option<bool> = None;
        let mut accountcode: Option<String> = None;
        let mut threadid: Option<u64> = None;
        // for maximum efficiency this could be a Vec<String>,
        // because it should always be contiguous.
        // Making it a HashMap makes the code much more readable however, so I decided for that.
        let mut custom_args: Option<HashMap<u8, String>> = None;

        for line in input.lines() {
            // stop on empty lines
            if line == "" {
                break;
            };
            let mut name_value = line.split(": ");
            let name = name_value.next();
            if name.is_none() {
                continue;
            };
            let value = name_value
                .next()
                .ok_or(AGIParseError::NoValue(line.to_owned()))?
                .trim_end();
            match name.expect("Should have been checked with is_none") {
                "agi_network_script" => {
                    network_script = Some(value.to_owned());
                }
                "agi_request" => {
                    request = Some(value.parse()?);
                }
                "agi_channel" => {
                    channel = Some(value.to_owned());
                }
                "agi_language" => {
                    language = Some(value.to_owned());
                }
                "agi_type" => {
                    channel_type = Some(value.to_owned());
                }
                "agi_uniqueid" => {
                    uniqueid = Some(value.to_owned());
                }
                "agi_version" => {
                    version = Some(value.to_owned());
                }
                "agi_callerid" => {
                    callerid = Some(value.to_owned());
                }
                "agi_calleridname" => {
                    calleridname = Some(value.to_owned());
                }
                "agi_callingpres" => {
                    callingpres = Some(value.to_owned());
                }
                "agi_callingani2" => {
                    callingani2 = Some(value.to_owned());
                }
                "agi_callington" => {
                    callington = Some(value.to_owned());
                }
                "agi_callingtns" => {
                    callingtns = Some(value.to_owned());
                }
                "agi_dnid" => {
                    dnid = Some(value.to_owned());
                }
                "agi_rdnis" => {
                    rdnis = Some(value.to_owned());
                }
                "agi_context" => {
                    context = Some(value.to_owned());
                }
                "agi_extension" => {
                    extension = Some(value.to_owned());
                }
                "agi_priority" => {
                    priority = Some(
                        value
                            .parse()
                            .map_err(|_| AGIParseError::PriorityUnparsable(value.to_owned()))?,
                    );
                }
                "agi_enhanced" => {
                    enhanced = Some(enhanced_status(value)?);
                }
                "agi_accountcode" => {
                    accountcode = Some(value.to_owned());
                }
                "agi_threadid" => {
                    threadid = Some(
                        value
                            .parse()
                            .map_err(|_| AGIParseError::ThreadIdUnparsable(value.to_owned()))?,
                    );
                }
                m => {
                    // custom args of the format
                    // agi_arg_n: value
                    if !m.starts_with("agi_arg_") {
                        return Err(AGIParseError::UnknownArg(m.to_owned()));
                    }
                    // at which position do we need to insert the value?
                    let custom_arg_number = &m[8..]
                        .parse::<u8>()
                        .map_err(|_| AGIParseError::CustomArgNumberUnparsable(m.to_owned()))?;
                    match custom_args {
                        // start with the value inserted at the correct position
                        None => {
                            custom_args = Some(HashMap::new());
                            custom_args
                                .as_mut()
                                .expect("Value should have been set to Some in the last statement")
                                .insert(*custom_arg_number, value.to_owned());
                        }
                        Some(ref mut x) => {
                            if x.contains_key(custom_arg_number) {
                                return Err(AGIParseError::DuplicateCustomArg(m.to_owned()));
                            }
                            x.insert(*custom_arg_number, value.to_owned());
                        }
                    }
                }
            }
        }
        // actually build the resulting dump and return it
        Ok(AGIVariableDump {
            network_script: network_script
                .ok_or(AGIParseError::VariableMissing("network_script".to_owned()))?,
            request: request.ok_or(AGIParseError::VariableMissing("request".to_owned()))?,
            channel: channel.ok_or(AGIParseError::VariableMissing("channel".to_owned()))?,
            language: language.ok_or(AGIParseError::VariableMissing("language".to_owned()))?,
            channel_type: channel_type
                .ok_or(AGIParseError::VariableMissing("channel_type".to_owned()))?,
            uniqueid: uniqueid.ok_or(AGIParseError::VariableMissing("uniqueid".to_owned()))?,
            version: version.ok_or(AGIParseError::VariableMissing("version".to_owned()))?,
            callerid: callerid.ok_or(AGIParseError::VariableMissing("callerid".to_owned()))?,
            calleridname: calleridname
                .ok_or(AGIParseError::VariableMissing("calleridname".to_owned()))?,
            callingpres: callingpres
                .ok_or(AGIParseError::VariableMissing("callingpres".to_owned()))?,
            callingani2: callingani2
                .ok_or(AGIParseError::VariableMissing("callingani2".to_owned()))?,
            callington: callington
                .ok_or(AGIParseError::VariableMissing("callington".to_owned()))?,
            callingtns: callingtns
                .ok_or(AGIParseError::VariableMissing("callingtns".to_owned()))?,
            dnid: dnid.ok_or(AGIParseError::VariableMissing("dnid".to_owned()))?,
            rdnis: rdnis.ok_or(AGIParseError::VariableMissing("rdnis".to_owned()))?,
            context: context.ok_or(AGIParseError::VariableMissing("context".to_owned()))?,
            extension: extension.ok_or(AGIParseError::VariableMissing("extension".to_owned()))?,
            priority: priority.ok_or(AGIParseError::VariableMissing("priority".to_owned()))?,
            enhanced: enhanced.ok_or(AGIParseError::VariableMissing("enhanced".to_owned()))?,
            accountcode: accountcode
                .ok_or(AGIParseError::VariableMissing("accountcode".to_owned()))?,
            threadid: threadid.ok_or(AGIParseError::VariableMissing("threadid".to_owned()))?,
            custom_args: custom_args.unwrap_or(HashMap::<u8, String>::new()),
        })
    }
}

/// All AGI Message that we may encounter.
/// The packet send by asterisk should always be parsable as AGIMessage.
#[derive(Debug, PartialEq, Eq)]
pub enum AGIMessage {
    /// VariableDump (i.e. a request)
    VariableDump(AGIVariableDump),
    /// Status (i.e. the Response after we have sent a command)
    Status(AGIStatusGeneric),
    /// the packet `agi_network: yes\n`
    NetworkStart,
}
impl FromStr for AGIMessage {
    type Err = AGIParseError;
    #[tracing::instrument(level=Level::TRACE, ret, err)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("agi_network: yes") {
            Ok(AGIMessage::NetworkStart)
        } else if s.contains(" result=") {
            Ok(AGIMessage::Status(
                s.split('\n')
                    .next()
                    .ok_or(AGIParseError::StatusWithoutNewline)?
                    .parse()?,
            ))
        } else {
            Ok(AGIMessage::VariableDump(s.parse()?))
        }
    }
}
impl Display for AGIMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            AGIMessage::VariableDump(x) => {
                write!(f, "{x}")
            }
            AGIMessage::Status(x) => {
                write!(f, "{x}")
            }
            AGIMessage::NetworkStart => {
                write!(f, "agi_network: yes")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agi_variable_dump_w_custom_args() {
        let message = "agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n\
            agi_calleridname: marcelog@mg \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dnid: 667 \n\
            agi_rdnis: unknown \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960 \n\
            agi_arg_1: arg1\n\
            agi_arg_2: arg2\n\
            agi_arg_3: arg3\n\n\0\0\0";
        let vardump = message.parse::<AGIVariableDump>().unwrap();
        let mut arghashmap = HashMap::new();
        arghashmap.insert(1u8, "arg1".to_owned());
        arghashmap.insert(2u8, "arg2".to_owned());
        arghashmap.insert(3u8, "arg3".to_owned());
        assert_eq!(
            vardump,
            AGIVariableDump {
                network_script: "agi.sh".to_owned(),
                request: AGIRequestType::File(PathBuf::from("/tmp/agi.sh"),),
                channel: "SIP/marcelog-e00d2760".to_owned(),
                language: "ar".to_owned(),
                channel_type: "SIP".to_owned(),
                uniqueid: "1297542965.8".to_owned(),
                version: "1.6.0.9".to_owned(),
                callerid: "marcelog".to_owned(),
                calleridname: "marcelog@mg".to_owned(),
                callingpres: "0".to_owned(),
                callingani2: "0".to_owned(),
                callington: "0".to_owned(),
                callingtns: "0".to_owned(),
                dnid: "667".to_owned(),
                rdnis: "unknown".to_owned(),
                context: "default".to_owned(),
                extension: "667".to_owned(),
                priority: 2,
                enhanced: false,
                accountcode: "".to_owned(),
                threadid: 1104922960,
                custom_args: arghashmap,
            }
        );
    }

    #[test]
    fn agi_variable_dump_wo_custom_args() {
        let message = "agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n\
            agi_calleridname: marcelog@mg \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dnid: 667 \n\
            agi_rdnis: unknown \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960\n\n\0\0";
        let vardump = message.parse::<AGIVariableDump>().unwrap();
        assert_eq!(
            vardump,
            AGIVariableDump {
                network_script: "agi.sh".to_owned(),
                request: AGIRequestType::File(PathBuf::from("/tmp/agi.sh"),),
                channel: "SIP/marcelog-e00d2760".to_owned(),
                language: "ar".to_owned(),
                channel_type: "SIP".to_owned(),
                uniqueid: "1297542965.8".to_owned(),
                version: "1.6.0.9".to_owned(),
                callerid: "marcelog".to_owned(),
                calleridname: "marcelog@mg".to_owned(),
                callingpres: "0".to_owned(),
                callingani2: "0".to_owned(),
                callington: "0".to_owned(),
                callingtns: "0".to_owned(),
                dnid: "667".to_owned(),
                rdnis: "unknown".to_owned(),
                context: "default".to_owned(),
                extension: "667".to_owned(),
                priority: 2,
                enhanced: false,
                accountcode: "".to_owned(),
                threadid: 1104922960,
                custom_args: HashMap::<u8, String>::new(),
            }
        );
    }

    #[test]
    fn agi_variable_dump_duplicate_custom_arg() {
        let message = "agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n\
            agi_calleridname: marcelog@mg \n\
            agi_rdnis: unknown \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dnid: 667 \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960 \n\
            agi_arg_1: 1234 \n\
            agi_arg_1: 567\n\n\0\0";
        assert!(message.parse::<AGIVariableDump>().is_err());
    }

    #[test]
    fn agi_variable_dump_missing_arg() {
        let message = "agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n\
            agi_calleridname: marcelog@mg \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dnid: 667 \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960\n\n\0\0";
        assert!(message.parse::<AGIVariableDump>().is_err());
    }

    #[test]
    fn agi_status_with_op_data() {
        let line = "200 result=1 done\n";
        assert_eq!(
            line.parse::<AGIStatusGeneric>(),
            Ok(AGIStatusGeneric::Ok(
                "1".to_owned(),
                Some("done".to_owned())
            ))
        );
    }

    #[test]
    fn agi_status_wo_op_data() {
        let line = "200 result=1 \n";
        assert_eq!(
            line.parse::<AGIStatusGeneric>(),
            Ok(AGIStatusGeneric::Ok("1".to_owned(), None))
        );
    }

    #[test]
    fn agi_status_unparsable_code() {
        let line = "2f00 result=1 \n";
        assert!(line.parse::<AGIStatusGeneric>().is_err());
    }

    #[test]
    fn agi_status_unparsable_result() {
        let line = "200 result:1 \n";
        assert!(line.parse::<AGIStatusGeneric>().is_err());
    }

    #[test]
    fn agi_status_line_empty() {
        let line = " \n";
        assert!(line.parse::<AGIStatusGeneric>().is_err());
    }

    #[test]
    fn agi_status_no_result() {
        let line = "404 \n";
        assert!(line.parse::<AGIStatusGeneric>().is_err());
    }

    #[test]
    fn agi_message_status() {
        let message = "200 result=1 done \ncript: lolli\nagi_request: gedöns\n";
        assert_eq!(
            message.parse::<AGIMessage>(),
            Ok(AGIMessage::Status(AGIStatusGeneric::Ok(
                "1".to_owned(),
                Some("done".to_owned())
            )))
        );
    }

    #[test]
    fn agi_message_dump() {
        let message = "\
            agi_network_script: agi.sh \n\
            agi_request: /tmp/agi.sh \n\
            agi_channel: SIP/marcelog-e00d2760 \n\
            agi_language: ar \n\
            agi_type: SIP \n\
            agi_uniqueid: 1297542965.8 \n\
            agi_version: 1.6.0.9 \n\
            agi_callerid: marcelog \n\
            agi_calleridname: marcelog@mg \n\
            agi_callingpres: 0 \n\
            agi_callingani2: 0 \n\
            agi_callington: 0 \n\
            agi_callingtns: 0 \n\
            agi_dnid: 667 \n\
            agi_rdnis: unknown \n\
            agi_context: default \n\
            agi_extension: 667 \n\
            agi_priority: 2 \n\
            agi_enhanced: 0.0 \n\
            agi_accountcode: \n\
            agi_threadid: 1104922960 \n\n\0\0";
        let vardump = message.parse::<AGIMessage>().unwrap();
        assert_eq!(
            vardump,
            AGIMessage::VariableDump(AGIVariableDump {
                network_script: "agi.sh".to_owned(),
                request: AGIRequestType::File(PathBuf::from("/tmp/agi.sh"),),
                channel: "SIP/marcelog-e00d2760".to_owned(),
                language: "ar".to_owned(),
                channel_type: "SIP".to_owned(),
                uniqueid: "1297542965.8".to_owned(),
                version: "1.6.0.9".to_owned(),
                callerid: "marcelog".to_owned(),
                calleridname: "marcelog@mg".to_owned(),
                callingpres: "0".to_owned(),
                callingani2: "0".to_owned(),
                callington: "0".to_owned(),
                callingtns: "0".to_owned(),
                dnid: "667".to_owned(),
                rdnis: "unknown".to_owned(),
                context: "default".to_owned(),
                extension: "667".to_owned(),
                priority: 2,
                enhanced: false,
                accountcode: "".to_owned(),
                threadid: 1104922960,
                custom_args: HashMap::<u8, String>::new(),
            })
        );
    }

    #[test]
    fn agi_message_garbage() {
        let message = "some stuff \n and some more stuff";
        assert!(message.parse::<AGIMessage>().is_err());
    }

    #[test]
    fn agi_message_network_start() {
        let message = "agi_network: yes";
        assert_eq!(message.parse::<AGIMessage>(), Ok(AGIMessage::NetworkStart));
    }
}
