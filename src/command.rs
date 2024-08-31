use crate::agiparse::AGIStatusGeneric;

pub mod answer;
pub mod verbose;
pub mod get_full_variable;

/// AGI Commands that we can issue
/// Inputs are always the inputs asterisk expects, in the same order as in the documentation
/// https://docs.asterisk.org/Asterisk_22_Documentation/API_Documentation/AGI_Commands
// #[derive(Debug, PartialEq)]
// pub enum AGICommand {
//     /// Answer the channel, if not already in Answer state.
//     /// Returns: -1 on failure, 0 on success
//     Answer,
//     /// Break the AGI command execution and return to the caller.
//     /// NOTE: it is enough to simply drop the connection; this command is usually not required
//     AsyncAGIBreak,
//     /// Get the status of the current channel
//     /// Returns:
//     /// 0 - Channel is down and available.
//     /// 1 - Channel is down, but reserved.
//     /// 2 - Channel is off hook.
//     /// 3 - Digits (or equivalent) have been dialed.
//     /// 4 - Line is ringing.
//     /// 5 - Remote end is ringing.
//     /// 6 - Line is up.
//     /// 7 - Line is busy.
//     ChannelStatus(String),
//     /// Stream a file with fast-forward, pause etc. controlled by the user
//     /// INPUTS:
//     /// - filename WITHOUT FILE EXTENSION
//     /// - escape digits TODO what does this mean?
//     /// - skipms: TODO was tut das
//     /// - ffchr: character the user can press to fast-forward
//     /// - rewchr: character the user can press to reverse
//     /// - pausechr: character the user can press to pause
//     /// - offsetms: Start playback at this offset in whole ms
//     /// TODO: input Type hierfür
//     /// OUTPUTS:
//     /// - 0: Playback stopped and no digit was pressed
//     /// - ASCII VALUE OF A DIGIT: a digit was pressed to end playback
//     /// - -1: Error or channel disconected
//     /// SIDEEFFECTS:
//     /// - Sets the following variables:
//     ///     - CPLAYBACKSTATUS: "SUCCESS", "USERSTOPPED", "REMOTESTOPPED", "ERROR"
//     ///     - CPLAYBACKOFFSET: position in ms where playback was stopped
//     ///     - CPLAYBACKSTOPKEY: Key pressed to stop playback
//     ControlStreamFile(String, Vec<Characters>, u64, Characters, Characters, Characters, u64),
//     /// In the Database family (param 1), delete the given key (param 2)
//     /// Returns: 1 on success, 0 otherwise
//     DatabaseDel(String, String),
//     /// In the database family (param 1), delete the given keytree (param 2)
//     /// Returns: 1 on success, 0 otherwise
//     DatabaseDeltree(String, String),
//     /// IN the database family (param 1), get the value of a key (param 2)
//     /// Returns: 0 if key does not exist; 1, opdata (the value) if the key does exist
//     DatabaseGet(String, String),
//     /// In the databse family (param 1), set the value of (param 2) to (param 3)
//     /// Returns: 1 on success, 0 otherwise
//     DatabasePut(String, String, String),
//     /// Execute the application (param 1) and pass it the options given in (param 2)
//     /// Returns: The return value of the application, or -2 if the application was not found
//     Exec(String, String),
//     /// TODO: builder type for the input
//     /// Play the file (param 1) for at most (param 2) ..TODO..which..time..unit.. and allow the
//     /// user to enter at most (param 3) digits via DTMF
//     /// Returns: the digits entered
//     GetData(String, u64, u64),
//     /// Sends the following line:
//     /// `GET FULL VARIABLE x [y]`
//     /// This interpolates the first value in the channel given by the second value
//     /// or the current channel if the second value is None
//     GetFullVariable(String, Option<String>),
//     /// Play the file (param 1), record DTMF, break on a digit in (param 2) or after (param 3)
//     /// seconds
//     /// Returns: the entered Digits
//     GetOption(String, Vec<Digit>, u64),
//     /// Get the value of variable (param 1). Can only get user variables. Prefer to use GetFullVariable.
//     /// Returns 0 if the variable does not exist and 1 opdata: (value) otherwise
//     GetVariable(String),
//     /// Call the subroutine at context (param 1), extension (param 2), priority (param 3) and pass
//     /// it (param 4) as argument
//     GoSub(String, String, u16, Option<String>),
// 
//     // TODO: weitermachen. Ich glaube es ist doch besser, die Dinge direkt zu implementieren und zu
//     // testen
// 
//     /// Emit the string as a log message in Asterisk.
//     Verbose(String),
//     /// Given values x, y: set the Value of variable x to y
//     /// This is equivalent to Set(x=y) in extensions.conf, normal channel rules apply
//     SetVariable(String, String),
// }
// impl std::fmt::Display for AGICommand {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         match self {
//             Self::Verbose(msg) => {
//                 write!(f, "VERBOSE \"{msg}\"\n")
//             }
//             Self::SetVariable(name, value) => {
//                 write!(f, "SET VARIABLE \"{name}\" \"{value}\"\n")
//             }
//             Self::GetFullVariable(expr, channel) => {
//                 if let Some(x) = channel {
//                     write!(f, "GET FULL VARIABLE \"{expr}\" \"{x}\"\n")
//                 } else {
//                     write!(f, "GET FULL VARIABLE \"{expr}\"\n")
//                 }
//             }
//         }
//     }
// }



#[derive(Debug,PartialEq)]
pub struct AGIStatusParseError {
    result: String,
    op_data: Option<String>,
    pub response_to_command: &'static str,
}
impl std::fmt::Display for AGIStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Unable to parse result {}, data {:?} as {} Status.", self.result, self.op_data, self.response_to_command)
    }
}
impl std::error::Error for AGIStatusParseError {}

#[derive(Debug,PartialEq)]
pub enum AGIResponse<H> where H: InnerAGIResponse + Sized {
    Ok(H),
    Invalid,
    DeadChannel,
    EndUsage,
}
impl<H> Into<u16> for AGIResponse<H> where H: InnerAGIResponse + Sized {
    fn into(self) -> u16 {
        match self {
            AGIResponse::Ok(_) => 200,
            AGIResponse::Invalid => 510,
            AGIResponse::DeadChannel => 511,
            AGIResponse::EndUsage => 520,
        }
    }
}


pub trait InnerAGIResponse: std::fmt::Debug + for<'a> TryFrom<(&'a str, Option<&'a str>), Error = AGIStatusParseError>  + Send + Sync {
}

pub trait AGICommand: std::fmt::Display + std::fmt::Debug + Send + Sync {
    type Response: InnerAGIResponse;
}


#[derive(Debug,PartialEq)]
pub enum Characters {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Star,
    Pound,
}

#[derive(Debug,PartialEq)]
pub enum Digit {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}
impl Into<Characters> for Digit{
    fn into(self) -> Characters {
        match self {
            Digit::Zero => Characters::Zero,
            Digit::One => Characters::One,
            Digit::Two => Characters::Two,
            Digit::Three => Characters::Three,
            Digit::Four => Characters::Four,
            Digit::Five => Characters::Five,
            Digit::Six => Characters::Six,
            Digit::Seven => Characters::Seven,
            Digit::Eight => Characters::Eight,
            Digit::Nine => Characters::Nine,
        }
    }
}

