use std::fmt::Display;
use std::str::FromStr;
use thiserror::*;

// Specification of a version with possible wildcard
#[derive(Clone, Copy)]
pub struct VerSpec {
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
}

impl Display for VerSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.", self.major)?;
        match self.minor {
            None => write!(f, "*."),
            Some(m) => write!(f, "{}.", m),
        }?;
        match self.patch {
            None => write!(f, "*"),
            Some(m) => write!(f, "{}", m),
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseVerSpecError {
    #[error("empty version spec string")]
    EmptyString,
    #[error("Invalid trailing string {0}")]
    Trailing(String),
    #[error("major not a number {0}")]
    MajorNotNumber(std::num::ParseIntError),
    #[error("minor not valid {0}")]
    MinorNotValid(std::num::ParseIntError),
    #[error("patch not valid {0}")]
    PatchNotValid(std::num::ParseIntError),
}

impl FromStr for VerSpec {
    type Err = ParseVerSpecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut a = s.split(".");

        let major_str = a.next().ok_or(ParseVerSpecError::EmptyString)?;
        let (minor_str, patch_str) = if let Some(minor_str) = a.next() {
            let patch_str = if let Some(patch_str) = a.next() {
                if let Some(trailing) = a.next() {
                    return Err(ParseVerSpecError::Trailing(trailing.to_string()));
                }
                patch_str
            } else {
                "*"
            };
            (minor_str, patch_str)
        } else {
            ("*", "*")
        };

        let major = u64::from_str(major_str).map_err(ParseVerSpecError::MajorNotNumber)?;
        let minor = match minor_str {
            "*" => None,
            s => Some(u64::from_str(s).map_err(ParseVerSpecError::MinorNotValid)?),
        };
        let patch = match patch_str {
            "*" => None,
            s => Some(u64::from_str(s).map_err(ParseVerSpecError::PatchNotValid)?),
        };
        Ok(VerSpec {
            major,
            minor,
            patch,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CompareOp {
    Not,
    Eq,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

impl Display for CompareOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            CompareOp::Not => "~",
            CompareOp::Eq => "=",
            CompareOp::Greater => ">",
            CompareOp::GreaterEqual => ">=",
            CompareOp::Less => "<",
            CompareOp::LessEqual => "<=",
        };
        write!(f, "{}", op)
    }
}

impl FromStr for CompareOp {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ">=" => Ok(CompareOp::GreaterEqual),
            ">" => Ok(CompareOp::Greater),
            "<=" => Ok(CompareOp::LessEqual),
            "<" => Ok(CompareOp::Less),
            "=" => Ok(CompareOp::Eq),
            "~" => Ok(CompareOp::Not),
            _ => Err(s.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct Spec(CompareOp, VerSpec);

impl Display for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.0, self.1)
    }
}

#[derive(Clone, Debug, Error)]
pub enum ParseSpecError {
    #[error("empty string")]
    EmptyString,
    #[error("missing version")]
    MissingVersion,
    #[error("trailing content")]
    Trailing(String),
    #[error("invalid operator {0}")]
    InvalidOp(String),
    #[error("invalid version")]
    InvalidVersion(#[from] ParseVerSpecError),
}

impl FromStr for Spec {
    type Err = ParseSpecError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut x = s.split(" ");

        let op_str = x.next().ok_or(ParseSpecError::EmptyString)?;
        let ver_str = x.next().ok_or(ParseSpecError::MissingVersion)?;

        if let Some(next) = x.next() {
            return Err(ParseSpecError::Trailing(next.to_string()));
        }

        let op = CompareOp::from_str(op_str).map_err(|s| ParseSpecError::InvalidOp(s))?;
        let ver = VerSpec::from_str(ver_str)?;
        Ok(Self(op, ver))
    }
}

impl Spec {
    pub fn fullfill(&self, major: u64, minor: u64, patch: u64) -> bool {
        match self.0 {
            CompareOp::Eq => eq(&self.1, major, minor, patch),
            CompareOp::Not => !eq(&self.1, major, minor, patch),
            CompareOp::GreaterEqual => {
                eq(&self.1, major, minor, patch) || greater(&self.1, major, minor, patch)
            }
            CompareOp::LessEqual => {
                eq(&self.1, major, minor, patch) || !greater(&self.1, major, minor, patch)
            }
            CompareOp::Greater => greater(&self.1, major, minor, patch),
            CompareOp::Less => {
                !greater(&self.1, major, minor, patch) && !eq(&self.1, major, minor, patch)
            }
        }
    }
}

pub fn greater(v: &VerSpec, major: u64, minor: u64, patch: u64) -> bool {
    if major > v.major {
        return true;
    }
    if major < v.major {
        return false;
    }

    match (v.minor, v.patch) {
        (None, None) => false,
        (None, Some(_)) => false,
        (Some(v_minor), None) => minor > v_minor,
        (Some(v_minor), Some(v_patch)) => minor > v_minor || (minor == v_minor && patch > v_patch),
    }
}

pub fn eq(v: &VerSpec, major: u64, minor: u64, patch: u64) -> bool {
    major == v.major && v.minor.map_or(true, |m| minor == m) && v.patch.map_or(true, |r| patch == r)
}
