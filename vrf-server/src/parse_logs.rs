use std::collections::HashMap;

use anchor_client::solana_sdk::pubkey::Pubkey;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub enum ParseLogError {
    ProgramIdMismatch {
        line: String,
        current: Option<String>,
        expect: String,
    },
    NoCurrentProgramId {
        line: String,
    },
    Base64Decode {
        line: String,
        error: String,
    },
    ParseInvoke {
        line: String,
    },
    ParseReturn {
        line: String,
    },
}

#[derive(Debug)]
enum LogType<'a> {
    Trivia,
    Data(&'a str),
    Invoke(&'a str),
    Return(&'a str),
    InProgram(&'a str),
}

fn parse_log_line<'a>(log: &'a str) -> Result<LogType<'a>, ParseLogError> {
    static RE_INVOKE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^Program (.*) invoke.*$").unwrap());
    static RE_RETURN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^Program (.*) success*$").unwrap());

    if let Some(log) = log
        .strip_prefix("Program log: ")
        .or_else(|| log.strip_prefix("Program data: "))
    {
        // Check if log contains only base64 character
        if log
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '+' && c != '/' && c != '=')
            .is_none()
        {
            return Ok(LogType::Data(log));
        }

        return Ok(LogType::Trivia);
    }

    if let Some(capture) = RE_INVOKE.captures(log) {
        return Ok(LogType::Invoke(
            capture
                .get(1)
                .ok_or_else(|| ParseLogError::ParseInvoke {
                    line: log.to_string(),
                })?
                .as_str(),
        ));
    };

    if let Some(capture) = RE_RETURN.captures(log) {
        return Ok(LogType::Return(
            capture
                .get(1)
                .ok_or_else(|| ParseLogError::ParseReturn {
                    line: log.to_string(),
                })?
                .as_str(),
        ));
    }

    if let Some(log) = log.strip_prefix("Program ") {
        if let Some(end_index) = log.find(' ') {
            return Ok(LogType::InProgram(&log[..end_index]));
        }
    }

    return Ok(LogType::Trivia);
}

pub struct AnchorEvent {
    pub program_id: Pubkey,
    pub data: Vec<u8>,
}

pub fn parse_logs<S: AsRef<str>>(
    logs: &[S],
    program_ids: &[Pubkey],
) -> (Vec<AnchorEvent>, Vec<ParseLogError>) {
    let mut events = Vec::new();
    let mut errors = Vec::new();
    let mut cpi_stack = Vec::new();

    let program_ids: HashMap<String, Pubkey> = program_ids
        .iter()
        .map(|pubkey| (pubkey.to_string(), *pubkey))
        .collect();

    for log in logs {
        let log = log.as_ref();
        match parse_log_line(log) {
            Ok(LogType::Invoke(program_id)) => cpi_stack.push(program_id),
            Ok(LogType::Return(program_id)) => {
                if let Some(popped_program_id) = cpi_stack.pop() {
                    if popped_program_id != program_id {
                        errors.push(ParseLogError::ProgramIdMismatch {
                            line: log.to_string(),
                            current: Some(popped_program_id.to_string()),
                            expect: program_id.to_string(),
                        });
                    }
                }
            }
            Ok(LogType::InProgram(program_id)) => match cpi_stack.last() {
                Some(current) if *current != program_id => {
                    errors.push(ParseLogError::ProgramIdMismatch {
                        line: log.to_string(),
                        current: Some(current.to_string()),
                        expect: program_id.to_string(),
                    });

                    continue;
                }
                Some(_) => {}
                None => {
                    errors.push(ParseLogError::ProgramIdMismatch {
                        line: log.to_string(),
                        current: None,
                        expect: program_id.to_string(),
                    });

                    continue;
                }
            },
            Ok(LogType::Data(data)) => {
                let current_program_id = match cpi_stack.last() {
                    Some(id) => id,
                    None => {
                        errors.push(ParseLogError::NoCurrentProgramId {
                            line: log.to_string(),
                        });
                        continue;
                    }
                };

                if let Some(program_id) = program_ids.get(*current_program_id) {
                    use base64::Engine;

                    let bytes = match base64::engine::general_purpose::STANDARD.decode(data) {
                        Ok(b) => b,
                        Err(err) => {
                            errors.push(ParseLogError::Base64Decode {
                                line: log.to_string(),
                                error: format!("{err:#}"),
                            });

                            continue;
                        }
                    };

                    events.push(AnchorEvent {
                        program_id: *program_id,
                        data: bytes.to_vec(),
                    });
                }
            }
            Ok(LogType::Trivia) => {}
            Err(err) => {
                errors.push(err);
            }
        }
    }

    (events, errors)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_parse_log_line() {
        let logs = [
            "Program DEoxdV1CCWvbeGp8PpwkUifmm3pV5AgtFwFaS4P7qZeZ invoke [1]",
            "Program log: Instruction: Spin",
            "Program log: Transfering stake to pool",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
            "Program log: Instruction: Transfer",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4645 of 182491 compute units",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
            "Program data: aGVsbG93b3JsZCE=",
            "Program log: Transfering token to user",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
            "Program log: Instruction: Transfer",
            "Program data: bmVzdGVkIGRhdGE=",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4740 of 159826 compute units",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
            "Program log: Transfering tax to treasury",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
            "Program log: Instruction: Transfer",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4740 of 152166 compute units",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
            "Program log: Burning part of the tax",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]",
            "Program log: Instruction: Burn",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4844 of 144508 compute units",
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success",
            "Program DEoxdV1CCWvbeGp8PpwkUifmm3pV5AgtFwFaS4P7qZeZ consumed 63281 of 200000 compute units",
            "Program DEoxdV1CCWvbeGp8PpwkUifmm3pV5AgtFwFaS4P7qZeZ success",
        ];

        let pubkey_a = Pubkey::from_str("DEoxdV1CCWvbeGp8PpwkUifmm3pV5AgtFwFaS4P7qZeZ").unwrap();
        let pubkey_b = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();

        let (events, errors) = parse_logs(&logs, &[pubkey_a, pubkey_b]);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].program_id, pubkey_a);
        assert_eq!(events[0].data, b"helloworld!");
        assert_eq!(events[1].program_id, pubkey_b);
        assert_eq!(events[1].data, b"nested data");

        assert!(errors.is_empty());
    }
}
