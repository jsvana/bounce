use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InvalidMessageError {
    #[error("Message has no contents")]
    Empty,
}

#[derive(Debug)]
pub struct Prefix {
    entity: String,
    user: Option<String>,
    host: Option<String>,
}

impl PartialEq for Prefix {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity && self.user == other.user && self.host == other.host
    }
}

impl Prefix {
    fn split_on_string(s: &str, message: &str) -> (String, Option<String>) {
        let at = message.find(s);
        (
            at.map_or(message.to_string(), |idx| message[..idx].to_string()),
            at.map(|idx| message[idx + 1..].to_string()),
        )
    }
}

impl fmt::Display for Prefix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.entity)?;

        if let Some(user) = &self.user {
            write!(f, "!{}", user)?;
        }

        if let Some(host) = &self.host {
            write!(f, "@{}", host)?;
        }

        Ok(())
    }
}

impl FromStr for Prefix {
    type Err = Infallible;

    fn from_str(message: &str) -> Result<Self, Self::Err> {
        let (entity, rest) = Prefix::split_on_string("!", message);
        if let Some(rest) = rest {
            let (user, host) = Prefix::split_on_string("@", &rest);
            Ok(Prefix {
                entity,
                user: Some(user),
                host,
            })
        } else {
            let (entity, host) = Prefix::split_on_string("@", &entity);
            Ok(Prefix {
                entity,
                user: None,
                host,
            })
        }
    }
}

#[derive(Debug)]
pub struct Message {
    prefix: Option<Prefix>,
    command: String,
    params: Option<Vec<String>>,
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.prefix == other.prefix && self.command == other.command && self.params == other.params
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(prefix) = &self.prefix {
            write!(f, ":{} ", prefix)?;
        }

        write!(f, "{}", self.command)?;

        if let Some(ref params) = self.params {
            for (i, param) in params.iter().enumerate() {
                write!(f, " ")?;
                if i == params.len() - 1 {
                    write!(f, ":")?;
                }
                write!(f, "{}", param)?;
            }
        }

        Ok(())
    }
}

impl FromStr for Message {
    type Err = InvalidMessageError;

    fn from_str(message: &str) -> Result<Self, Self::Err> {
        if message.len() == 0 {
            return Err(InvalidMessageError::Empty);
        }

        let mut space = match message.find(" ") {
            Some(idx) => idx,
            None => {
                return Ok(Message {
                    prefix: None,
                    command: message.to_string(),
                    params: None,
                })
            }
        };

        let mut message_iter = message;
        let prefix = match &message[..1] {
            ":" => {
                let old_space = space;
                message_iter = &message[space + 1..];
                space = message_iter.find(" ").unwrap_or(message_iter.len());
                Some(message[1..old_space].parse().unwrap())
            }
            _ => None,
        };

        let command = message_iter[..space].to_string();

        if space == message_iter.len() {
            return Ok(Message {
                prefix,
                command,
                params: None,
            });
        }

        message_iter = &message_iter[space + 1..];

        let mut params = Vec::new();

        while message_iter.len() > 0 {
            if &message_iter[..1] == ":" {
                params.push(message_iter[1..].to_string());
                break;
            }

            message_iter = match &message_iter[..1] {
                ":" => {
                    params.push(message_iter[1..].to_string());
                    ""
                }
                _ => match message_iter.find(" ") {
                    Some(idx) => {
                        params.push(message_iter[..idx].to_string());
                        &message_iter[idx + 1..]
                    }
                    None => {
                        params.push(message_iter.to_string());
                        ""
                    }
                },
            }
        }

        Ok(Message {
            prefix,
            command: command.to_string(),
            params: Some(params),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Result;

    #[test]
    fn test_parse_prefix_only_entity() -> Result<()> {
        assert_eq!(
            Prefix::from_str("irc-west.hs.gy")?,
            Prefix {
                entity: "irc-west.hs.gy".to_string(),
                user: None,
                host: None
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_prefix_no_user() -> Result<()> {
        assert_eq!(
            Prefix::from_str("jay@localhost")?,
            Prefix {
                entity: "jay".to_string(),
                user: None,
                host: Some("localhost".to_string())
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_prefix_no_host() -> Result<()> {
        assert_eq!(
            Prefix::from_str("jay!jsvana")?,
            Prefix {
                entity: "jay".to_string(),
                user: Some("jsvana".to_string()),
                host: None
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_prefix_all_info() -> Result<()> {
        assert_eq!(
            Prefix::from_str("jay!jsvana@localhost")?,
            Prefix {
                entity: "jay".to_string(),
                user: Some("jsvana".to_string()),
                host: Some("localhost".to_string())
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_message_no_params() -> Result<()> {
        assert_eq!(
            Message::from_str(":jay@localhost FAKE")?,
            Message {
                prefix: Some(Prefix {
                    entity: "jay".to_string(),
                    user: None,
                    host: Some("localhost".to_string())
                }),
                command: "FAKE".to_string(),
                params: None,
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_message_only_command() -> Result<()> {
        assert_eq!(
            Message::from_str("FAKE")?,
            Message {
                prefix: None,
                command: "FAKE".to_string(),
                params: None,
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_message_notice() -> Result<()> {
        assert_eq!(
            Message::from_str(":irc-west.hs.gy NOTICE * :*** Looking up your hostname...")?,
            Message {
                prefix: Some(Prefix {
                    entity: "irc-west.hs.gy".to_string(),
                    user: None,
                    host: None
                }),
                command: "NOTICE".to_string(),
                params: Some(vec![
                    "*".to_string(),
                    "*** Looking up your hostname...".to_string()
                ]),
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_message_privmsg() -> Result<()> {
        assert_eq!(
            Message::from_str(":jay!jsvana PRIVMSG belak :test message")?,
            Message {
                prefix: Some(Prefix {
                    entity: "jay".to_string(),
                    user: Some("jsvana".to_string()),
                    host: None
                }),
                command: "PRIVMSG".to_string(),
                params: Some(vec!["belak".to_string(), "test message".to_string()]),
            },
        );

        Ok(())
    }

    #[test]
    fn test_parse_message_ping() -> Result<()> {
        assert_eq!(
            Message::from_str("PING :1234")?,
            Message {
                prefix: None,
                command: "PING".to_string(),
                params: Some(vec!["1234".to_string()]),
            },
        );

        Ok(())
    }

    #[test]
    fn test_prefix_to_string_only_entity() {
        assert_eq!(
            format!(
                "{}",
                Prefix {
                    entity: "irc-west.hs.gy".to_string(),
                    user: None,
                    host: None
                }
            ),
            "irc-west.hs.gy".to_string(),
        );
    }

    #[test]
    fn test_prefix_to_string_no_user() {
        assert_eq!(
            format!(
                "{}",
                Prefix {
                    entity: "jay".to_string(),
                    user: None,
                    host: Some("localhost".to_string())
                }
            ),
            "jay@localhost".to_string(),
        );
    }

    #[test]
    fn test_prefix_to_string_no_host() {
        assert_eq!(
            format!(
                "{}",
                Prefix {
                    entity: "jay".to_string(),
                    user: Some("jsvana".to_string()),
                    host: None
                }
            ),
            "jay!jsvana".to_string(),
        );
    }

    #[test]
    fn test_prefix_to_string_all_info() {
        assert_eq!(
            format!(
                "{}",
                Prefix {
                    entity: "jay".to_string(),
                    user: Some("jsvana".to_string()),
                    host: Some("localhost".to_string())
                }
            ),
            "jay!jsvana@localhost".to_string(),
        );
    }

    #[test]
    fn test_message_to_string_no_params() {
        assert_eq!(
            format!(
                "{}",
                Message {
                    prefix: Some(Prefix {
                        entity: "jay".to_string(),
                        user: None,
                        host: Some("localhost".to_string())
                    }),
                    command: "FAKE".to_string(),
                    params: None,
                }
            ),
            ":jay@localhost FAKE".to_string(),
        )
    }

    #[test]
    fn test_message_to_string_only_command() {
        assert_eq!(
            format!(
                "{}",
                Message {
                    prefix: None,
                    command: "FAKE".to_string(),
                    params: None,
                }
            ),
            "FAKE".to_string(),
        )
    }

    #[test]
    fn test_message_to_string_notice() {
        assert_eq!(
            format!(
                "{}",
                Message {
                    prefix: Some(Prefix {
                        entity: "irc-west.hs.gy".to_string(),
                        user: None,
                        host: None
                    }),
                    command: "NOTICE".to_string(),
                    params: Some(vec![
                        "*".to_string(),
                        "*** Looking up your hostname...".to_string()
                    ]),
                }
            ),
            ":irc-west.hs.gy NOTICE * :*** Looking up your hostname...".to_string(),
        )
    }

    #[test]
    fn test_message_to_string_privmsg() {
        assert_eq!(
            format!(
                "{}",
                Message {
                    prefix: Some(Prefix {
                        entity: "jay".to_string(),
                        user: Some("jsvana".to_string()),
                        host: None
                    }),
                    command: "PRIVMSG".to_string(),
                    params: Some(vec!["belak".to_string(), "test message".to_string()]),
                }
            ),
            ":jay!jsvana PRIVMSG belak :test message".to_string(),
        )
    }

    #[test]
    fn test_message_to_string_ping() {
        assert_eq!(
            format!(
                "{}",
                Message {
                    prefix: None,
                    command: "PING".to_string(),
                    params: Some(vec!["1234".to_string()]),
                }
            ),
            "PING :1234".to_string(),
        )
    }
}
