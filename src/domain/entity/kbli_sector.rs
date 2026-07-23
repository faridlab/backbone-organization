use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr;
#[cfg(feature = "openapi")]
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "kbli_sector", rename_all = "snake_case")]
pub enum KBLISector {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
}

impl std::fmt::Display for KBLISector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "a"),
            Self::B => write!(f, "b"),
            Self::C => write!(f, "c"),
            Self::D => write!(f, "d"),
            Self::E => write!(f, "e"),
            Self::F => write!(f, "f"),
            Self::G => write!(f, "g"),
            Self::H => write!(f, "h"),
            Self::I => write!(f, "i"),
            Self::J => write!(f, "j"),
            Self::K => write!(f, "k"),
            Self::L => write!(f, "l"),
            Self::M => write!(f, "m"),
            Self::N => write!(f, "n"),
            Self::O => write!(f, "o"),
            Self::P => write!(f, "p"),
            Self::Q => write!(f, "q"),
            Self::R => write!(f, "r"),
            Self::S => write!(f, "s"),
            Self::T => write!(f, "t"),
            Self::U => write!(f, "u"),
        }
    }
}

impl FromStr for KBLISector {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "a" => Ok(Self::A),
            "b" => Ok(Self::B),
            "c" => Ok(Self::C),
            "d" => Ok(Self::D),
            "e" => Ok(Self::E),
            "f" => Ok(Self::F),
            "g" => Ok(Self::G),
            "h" => Ok(Self::H),
            "i" => Ok(Self::I),
            "j" => Ok(Self::J),
            "k" => Ok(Self::K),
            "l" => Ok(Self::L),
            "m" => Ok(Self::M),
            "n" => Ok(Self::N),
            "o" => Ok(Self::O),
            "p" => Ok(Self::P),
            "q" => Ok(Self::Q),
            "r" => Ok(Self::R),
            "s" => Ok(Self::S),
            "t" => Ok(Self::T),
            "u" => Ok(Self::U),
            _ => Err(format!("Unknown KBLISector variant: {}", s)),
        }
    }
}

impl Default for KBLISector {
    fn default() -> Self {
        Self::A
    }
}
