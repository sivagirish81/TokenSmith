use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Fast,
    Balanced,
    Quality,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Fast => "fast",
            Mode::Balanced => "balanced",
            Mode::Quality => "quality",
        }
    }

    pub fn quantization_preferences(&self) -> Vec<&'static str> {
        match self {
            Mode::Fast => vec!["q4_k_m", "q5_k_m", "q8_0"],
            Mode::Balanced => vec!["q5_k_m", "q4_k_m", "q8_0"],
            Mode::Quality => vec!["q8_0", "q6_k", "q5_k_m"],
        }
    }

    pub fn context_targets(&self) -> Vec<u32> {
        match self {
            Mode::Fast => vec![4096, 2048],
            Mode::Balanced => vec![16384, 8192, 4096],
            Mode::Quality => vec![32768, 16384, 8192, 4096],
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fast" => Ok(Self::Fast),
            "balanced" => Ok(Self::Balanced),
            "quality" => Ok(Self::Quality),
            _ => Err(format!("invalid mode: {s}")),
        }
    }
}

impl clap::ValueEnum for Mode {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Fast, Self::Balanced, Self::Quality]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::Mode;

    #[test]
    fn mode_mapping() {
        assert_eq!(Mode::Fast.context_targets()[0], 4096);
        assert_eq!(Mode::Balanced.quantization_preferences()[0], "q5_k_m");
        assert_eq!(Mode::Quality.context_targets()[0], 32768);
    }
}
