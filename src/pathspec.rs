use crate::errors::*;
use chrono::{DateTime, Utc, Datelike, Timelike};
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

pub struct UploadContext {
    format: String,
    dt: DateTime<Utc>,
    remote: String,
    filename: String,
    path: String,
    full_path: String,
}

impl UploadContext {
    pub fn new(format: String, remote: String, filename: String, path: String, full_path: String) -> UploadContext {
        UploadContext {
            format,
            dt: Utc::now(),
            remote,
            filename,
            path,
            full_path,
        }
    }

    pub fn generate(&self) -> Result<(String, bool)> {
        let mut chars = self.format.chars();

        let mut out = String::new();
        let mut deterministic = true;

        while let Some(c) = chars.next() {
            if c == '%' {
                match chars.next() {
                    Some('%') => out.push('%'),

                    Some('Y') => out.push_str(&format!("{:04}", self.dt.year())),
                    Some('m') => out.push_str(&format!("{:02}", self.dt.month())),
                    Some('d') => out.push_str(&format!("{:02}", self.dt.day())),

                    Some('H') => out.push_str(&format!("{:02}", self.dt.hour())),
                    Some('M') => out.push_str(&format!("{:02}", self.dt.minute())),
                    Some('S') => out.push_str(&format!("{:02}", self.dt.second())),

                    Some('h') => out.push_str(&self.remote),
                    Some('f') => out.push_str(&self.filename),
                    Some('p') => out.push_str(&self.path),
                    Some('P') => out.push_str(&self.full_path),

                    Some('r') => {
                        deterministic = false;
                        out.extend(
                            thread_rng()
                                .sample_iter(&Alphanumeric)
                                .take(6)
                        );
                    },

                    Some(_) => bail!("Invalid escape sequence"),
                    None => bail!("Unterminated percent escape"),
                }
            } else {
                out.push(c);
            }
        }

        Ok((out, deterministic))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(format: &str) -> UploadContext {
        UploadContext {
            format: format.to_string(),
            dt: "1996-12-19T16:39:57Z".parse::<DateTime<Utc>>().unwrap(),
            remote: "192.0.2.1".to_string(),
            filename: "ohai.txt".to_string(),
            path: "b/c/ohai.txt".to_string(),
            full_path: "a/b/c/ohai.txt".to_string(),
        }
    }

    #[test]
    fn date_folders() {
        let (p, d) = ctx("%Y-%m-%d/%f").generate().unwrap();
        assert_eq!((p.as_str(), d), ("1996-12-19/ohai.txt", true));
    }

    #[test]
    fn http_mirror() {
        let (p, d) = ctx("%h/%P").generate().unwrap();
        assert_eq!((p.as_str(), d), ("192.0.2.1/a/b/c/ohai.txt", true));
    }

    #[test]
    fn random_prefix() {
        let (p, d) = ctx("%r-%f").generate().unwrap();
        assert_eq!(p.len(), 15);
        assert!(!d)
    }

    #[test]
    fn literal_percent() {
        let (p, d) = ctx("%%").generate().unwrap();
        assert_eq!((p.as_str(), d), ("%", true));
    }

    #[test]
    fn trailing_percent() {
        let r = ctx("foo%").generate();
        assert!(r.is_err());
    }

    #[test]
    fn invalid_escape() {
        let r = ctx("%/").generate();
        assert!(r.is_err());
    }
}
