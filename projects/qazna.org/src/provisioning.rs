use crate::domain::Amount;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Food,
    Water,
    Energy,
}
impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Water => "water",
            Self::Energy => "energy",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllocationRequest {
    pub treasury: Uuid,
    pub resident: Uuid,
    pub period: String,
    pub category: Category,
    pub amount: Amount,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationReceipt {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub allocation: AllocationRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceRequest {
    pub id: Uuid,
    pub producer: Uuid,
    pub category: Category,
    pub unit: String,
    pub quantity: Amount,
    pub unit_measure: Amount,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: Uuid,
    pub producer: Uuid,
    pub category: String,
    pub unit: String,
    pub unit_measure: String,
    pub on_hand: String,
    pub reserved: String,
    pub available: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationRequest {
    pub resident: Uuid,
    pub resource: Uuid,
    pub quantity: Amount,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reservation {
    pub id: Uuid,
    pub resident: Uuid,
    pub resource: Uuid,
    pub quantity: String,
    pub amount: String,
    pub state: String,
    pub transaction_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionRequest {
    pub basis: String,
}

pub(crate) fn valid_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
}

pub(crate) fn valid_period(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y: u32 = value[0..4].parse().unwrap();
    let m: usize = value[5..7].parse().unwrap();
    let d: u32 = value[8..10].parse().unwrap();
    let leap = y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400));
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    y > 0 && (1..=12).contains(&m) && d > 0 && d <= days[m - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_calendar_period_and_basis() {
        for date in ["2026-11-02", "2028-02-29", "2000-02-29"] {
            assert!(valid_period(date));
        }
        for date in [
            "2026-02-29",
            "1900-02-29",
            "0000-01-01",
            "2026-13-01",
            "2026-1-01",
            "2026-01-00",
            "éééé-01",
        ] {
            assert!(!valid_period(date));
        }
        assert!(!valid_text("  ", 1000));
        assert!(!valid_text("a\nb", 1000));
    }
}
