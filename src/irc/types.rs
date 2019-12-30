#[derive(Debug)]
pub struct Prefix<'a> {
    entity: &'a str,
    user: Option<&'a str>,
    host: Option<&'a str>,
}

impl<'a> PartialEq for Prefix<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity && self.user == other.user && self.host == other.host
    }
}

impl<'a> Prefix<'a> {
    fn split_on_string<'b>(s: &str, message: &'b str) -> (&'b str, Option<&'b str>) {
        let at = message.find(s);
        (
            at.map_or(message, |idx| &message[..idx]),
            at.map(|idx| &message[idx + 1..]),
        )
    }

    fn parse(message: &str) -> Prefix {
        let (entity, rest) = Prefix::split_on_string("!", message);
        if let Some(rest) = rest {
            let (user, host) = Prefix::split_on_string("@", rest);
            Prefix {
                entity,
                user: Some(user),
                host,
            }
        } else {
            let (entity, host) = Prefix::split_on_string("@", entity);
            Prefix {
                entity,
                user: None,
                host,
            }
        }
    }

    fn to_string(&self) -> String {
        let mut prefix_str = format!("{}", self.entity);
        if let Some(user) = self.user {
            prefix_str += &format!("!{}", user);
        }
        if let Some(host) = self.host {
            prefix_str += &format!("@{}", host);
        }
        prefix_str
    }
}

#[derive(Debug)]
pub struct Message<'a> {
    prefix: Option<Prefix<'a>>,
    command: &'a str,
    params: Option<Vec<&'a str>>,
}

impl<'a> PartialEq for Message<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.prefix == other.prefix && self.command == other.command && self.params == other.params
    }
}

impl<'a> Message<'a> {
    pub fn parse(message: &str) -> Result<Message, &str> {
        if message.len() == 0 {
            return Err("Message is invalid (has zero length)");
        }

        let mut space = match message.find(" ") {
            Some(idx) => idx,
            None => {
                return Ok(Message {
                    prefix: None,
                    command: message,
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
                Some(Prefix::parse(&message[1..old_space]))
            }
            _ => None,
        };

        let command = &message_iter[..space];

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
                params.push(&message_iter[1..]);
                break;
            }

            message_iter = match &message_iter[..1] {
                ":" => {
                    params.push(&message_iter[1..]);
                    ""
                }
                _ => match message_iter.find(" ") {
                    Some(idx) => {
                        params.push(&message_iter[..idx]);
                        &message_iter[idx + 1..]
                    }
                    None => {
                        params.push(message_iter);
                        ""
                    }
                },
            }
        }

        Ok(Message {
            prefix,
            command,
            params: Some(params),
        })
    }

    fn to_string(&self) -> String {
        let mut message_str = match &self.prefix {
            Some(prefix) => format!(":{} ", prefix.to_string()),
            None => "".to_string(),
        };

        message_str += &format!("{}", self.command);

        if let Some(ref params) = self.params {
            for (i, param) in params.iter().enumerate() {
                if i == params.len() - 1 {
                    message_str += &format!(" :{}", param);
                } else {
                    message_str += &format!(" {}", param);
                }
            }
        }
        message_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prefix_only_entity() {
        assert_eq!(
            Prefix::parse("irc-west.hs.gy"),
            Prefix {
                entity: "irc-west.hs.gy",
                user: None,
                host: None
            },
        );
    }

    #[test]
    fn test_parse_prefix_no_user() {
        assert_eq!(
            Prefix::parse("jay@localhost"),
            Prefix {
                entity: "jay",
                user: None,
                host: Some("localhost")
            },
        );
    }

    #[test]
    fn test_parse_prefix_no_host() {
        assert_eq!(
            Prefix::parse("jay!jsvana"),
            Prefix {
                entity: "jay",
                user: Some("jsvana"),
                host: None
            },
        );
    }

    #[test]
    fn test_parse_prefix_all_info() {
        assert_eq!(
            Prefix::parse("jay!jsvana@localhost"),
            Prefix {
                entity: "jay",
                user: Some("jsvana"),
                host: Some("localhost")
            },
        );
    }

    #[test]
    fn test_parse_message_no_params() {
        let res = Message::parse(":jay@localhost FAKE").unwrap();
        assert_eq!(
            res,
            Message {
                prefix: Some(Prefix {
                    entity: "jay",
                    user: None,
                    host: Some("localhost")
                }),
                command: "FAKE",
                params: None,
            },
        )
    }

    #[test]
    fn test_parse_message_only_command() {
        assert_eq!(
            Message::parse("FAKE").unwrap(),
            Message {
                prefix: None,
                command: "FAKE",
                params: None,
            },
        )
    }

    #[test]
    fn test_parse_message_notice() {
        assert_eq!(
            Message::parse(":irc-west.hs.gy NOTICE * :*** Looking up your hostname...").unwrap(),
            Message {
                prefix: Some(Prefix {
                    entity: "irc-west.hs.gy",
                    user: None,
                    host: None
                }),
                command: "NOTICE",
                params: Some(vec!["*", "*** Looking up your hostname..."]),
            },
        )
    }

    #[test]
    fn test_parse_message_privmsg() {
        assert_eq!(
            Message::parse(":jay!jsvana PRIVMSG belak :test message").unwrap(),
            Message {
                prefix: Some(Prefix {
                    entity: "jay",
                    user: Some("jsvana"),
                    host: None
                }),
                command: "PRIVMSG",
                params: Some(vec!["belak", "test message"]),
            },
        )
    }

    #[test]
    fn test_parse_message_ping() {
        assert_eq!(
            Message::parse("PING :1234").unwrap(),
            Message {
                prefix: None,
                command: "PING",
                params: Some(vec!["1234"]),
            },
        )
    }

    #[test]
    fn test_prefix_to_string_only_entity() {
        assert_eq!(
            Prefix {
                entity: "irc-west.hs.gy",
                user: None,
                host: None
            }
            .to_string(),
            "irc-west.hs.gy",
        );
    }

    #[test]
    fn test_prefix_to_string_no_user() {
        assert_eq!(
            Prefix {
                entity: "jay",
                user: None,
                host: Some("localhost")
            }
            .to_string(),
            "jay@localhost",
        );
    }

    #[test]
    fn test_prefix_to_string_no_host() {
        assert_eq!(
            Prefix {
                entity: "jay",
                user: Some("jsvana"),
                host: None
            }
            .to_string(),
            "jay!jsvana",
        );
    }

    #[test]
    fn test_prefix_to_string_all_info() {
        assert_eq!(
            Prefix {
                entity: "jay",
                user: Some("jsvana"),
                host: Some("localhost")
            }
            .to_string(),
            "jay!jsvana@localhost",
        );
    }

    #[test]
    fn test_message_to_string_no_params() {
        assert_eq!(
            Message {
                prefix: Some(Prefix {
                    entity: "jay",
                    user: None,
                    host: Some("localhost")
                }),
                command: "FAKE",
                params: None,
            }
            .to_string(),
            ":jay@localhost FAKE",
        )
    }

    #[test]
    fn test_message_to_string_only_command() {
        assert_eq!(
            Message {
                prefix: None,
                command: "FAKE",
                params: None,
            }
            .to_string(),
            "FAKE",
        )
    }

    #[test]
    fn test_message_to_string_notice() {
        assert_eq!(
            Message {
                prefix: Some(Prefix {
                    entity: "irc-west.hs.gy",
                    user: None,
                    host: None
                }),
                command: "NOTICE",
                params: Some(vec!["*", "*** Looking up your hostname..."]),
            }
            .to_string(),
            ":irc-west.hs.gy NOTICE * :*** Looking up your hostname...",
        )
    }

    #[test]
    fn test_message_to_string_privmsg() {
        assert_eq!(
            Message {
                prefix: Some(Prefix {
                    entity: "jay",
                    user: Some("jsvana"),
                    host: None
                }),
                command: "PRIVMSG",
                params: Some(vec!["belak", "test message"]),
            }
            .to_string(),
            ":jay!jsvana PRIVMSG belak :test message",
        )
    }

    #[test]
    fn test_message_to_string_ping() {
        assert_eq!(
            Message {
                prefix: None,
                command: "PING",
                params: Some(vec!["1234"]),
            }
            .to_string(),
            "PING :1234",
        )
    }
}
