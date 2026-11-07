use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error};
use uuid::Uuid;

/// Positive integer minor units; the economic denomination is not yet fixed.
/// JSON uses strings so browsers cannot round large amounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount(i64);

impl Amount {
    pub fn parse(value: &str) -> Result<Self, &'static str> {
        if value.is_empty() || value.starts_with('0') || !value.bytes().all(|b| b.is_ascii_digit())
        {
            return Err("amount must be a canonical positive integer string");
        }
        let amount = value.parse::<i64>().map_err(|_| "amount exceeds i64")?;
        Ok(Self(amount))
    }

    pub fn units(self) -> i64 {
        self.0
    }
}

impl Serialize for Amount {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Amount {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Transfer {
    pub from: Uuid,
    pub to: Uuid,
    pub amount: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Receipt {
    pub transaction_id: Uuid,
    pub transfer: Transfer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    Treasury,
    Resident,
    Producer,
}

impl AccountKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Treasury => "treasury",
            Self::Resident => "resident",
            Self::Producer => "producer",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewAccount {
    pub id: Uuid,
    pub kind: AccountKind,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub kind: String,
    pub balance: String,
    pub reserved: String,
    pub available: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ambiguous_or_invalid_money() {
        for bad in [
            "",
            "0",
            "01",
            "-1",
            "+1",
            "1.0",
            "1e3",
            " 1",
            "١",
            "9223372036854775808",
        ] {
            assert!(Amount::parse(bad).is_err(), "{bad}");
        }
        assert!(serde_json::from_str::<Amount>("42").is_err());
    }

    #[test]
    fn preserves_amount_above_javascript_safe_integer() {
        let value = Amount::parse("9007199254740993").unwrap();
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            "\"9007199254740993\""
        );
        assert_eq!(
            Amount::parse(&i64::MAX.to_string()).unwrap().units(),
            i64::MAX
        );
    }
}
