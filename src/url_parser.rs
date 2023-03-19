use nom::bytes::complete::{tag, take_until, take_while};
use nom::character::complete::{alphanumeric0, char};
use nom::combinator::{opt, peek};
use nom::sequence::{preceded, separated_pair, terminated};
use nom::IResult;

fn key_value(i: &str) -> IResult<&str, (&str, &str)> {
    separated_pair(
        take_while(|c: char| c.is_alphabetic() || c == '.'),
        opt(char(':')),
        alphanumeric0,
    )(i)
}

fn end_with<'a>(split: &'a str) -> impl FnMut(&'a str) -> IResult<&'a str, &'a str> {
    move |i| terminated(take_until(split), tag(split))(i)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct URL {
    pub scheme: String,
    pub username: String,
    pub password: String,
    pub origin: String,
    pub host: String,
    pub port: String,
    pub path: String,
    pub query: String,
    pub query_pairs: Vec<(String, String)>,
    //    pub hash: String,
}

impl URL {
    pub fn parse(i: &'static str) -> Result<URL, Box<dyn std::error::Error>> {
        let (i, scheme) = URL::parse_scheme(i)?;
        let (i, (username, password)) = URL::parse_username_password(i)?;
        let (i, (host, port)) = URL::parse_host_port(i)?;
        let (i, path) = URL::parse_path(i)?;
        let (_, query) = URL::parse_query(i)?;
        // let (_, hash) = URL::parse_hash(i)?;

        let query_seg: Vec<&str> = query.split("=").collect();

        let query_pairs: Vec<_> = query_seg
            .into_iter()
            .filter(|x| x.trim().len() > 0)
            .map(|x| {
                let v: Vec<&str> = x.split("&").collect();
                if v.len() >= 2 {
                    return (v[0].to_string(), v[1].to_string());
                } else {
                    return (v[0].to_string(), "".to_string());
                }
            })
            .collect();

        Ok(URL {
            scheme: String::from(scheme),
            username: String::from(username),
            password: String::from(password),
            origin: format!("{}:{}", host, port),
            host: String::from(host),
            port: String::from(port),
            path: String::from(path),
            query: String::from(query),
            query_pairs,
            // hash: String::from(hash),
        })
    }

    /// parse struct to string
    ///
    /// ### example  
    /// ``` rust
    /// URL::stringify(&url_obj);
    /// ```
    pub fn stringify(obj: &URL) -> String {
        let mut link: String = format!("{}//", obj.scheme);
        if !obj.username.is_empty() {
            link.push_str(&obj.username);
            if !obj.password.is_empty() {
                link.push_str(&format!(":{}@", obj.password));
            }
        }

        format!("{}{}{}{}", link, obj.origin, obj.path, obj.query)
    }

    fn parse_scheme(i: &str) -> IResult<&str, &str> {
        end_with("//")(i)
    }

    fn parse_username_password(i: &str) -> IResult<&str, (&str, &str)> {
        let (i, pattern) = opt(end_with("@"))(i)?;
        if let Some(pattern) = pattern {
            let (_, tulp) = key_value(pattern)?;
            return Ok((i, tulp));
        }
        Ok((i, ("", "")))
    }

    fn parse_host_port(i: &str) -> IResult<&str, (&str, &str)> {
        terminated(key_value, peek(tag("/")))(i)
    }

    fn parse_path(i: &str) -> IResult<&str, &str> {
        let chars = "#?";
        take_while(move |c| !chars.contains(c))(i)
    }

    fn parse_query(i: &str) -> IResult<&str, &str> {
        preceded(peek(opt(tag("?"))), take_while(|c| c != ' '))(i)
    }

    // fn parse_hash(i: &str) -> IResult<&str, &str> {
    //     preceded(peek(opt(tag("#"))), take_while(|c: char| c != ' '))(i)
    // }
}
