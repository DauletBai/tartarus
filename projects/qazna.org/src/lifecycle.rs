//! Offline, exact lifecycle transitions. No ledger authority or live attestations.
use crate::mathematics::MathError;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Location {
    Source,
    Destination,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Lot {
    quantity: u64,
    expires: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Shipment {
    lot: u64,
    quantity: u64,
    expires: u32,
    due: u32,
}
/// Quantities refer to ONE commodity in one declared physical unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LotBook {
    day: Option<u32>,
    lots: BTreeMap<(Location, u64), Lot>,
    shipments: BTreeMap<u64, Shipment>,
    lot_ids: BTreeSet<u64>,
    shipment_ids: BTreeSet<u64>,
    supplied: u64,
    consumed: u64,
    expired: u64,
    transport_loss: u64,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LotBalance {
    pub supplied: u64,
    pub consumed: u64,
    pub expired: u64,
    pub transport_loss: u64,
    pub source: u64,
    pub destination: u64,
    pub in_transit: u64,
}
impl LotBook {
    fn today(&self) -> Result<u32, MathError> {
        self.day.ok_or(MathError::Invalid("begin a period first"))
    }
    /// Expiry is exclusive: stock expiring at t is unusable from the start of t.
    /// Periods must be contiguous, beginning with zero. Expiry includes transit.
    pub fn begin_period(&mut self, day: u32) -> Result<(), MathError> {
        let expected = match self.day {
            None => 0,
            Some(d) => d.checked_add(1).ok_or(MathError::Overflow)?,
        };
        if day != expected {
            return Err(MathError::Invalid("noncontiguous lifecycle period"));
        }
        self.lots.retain(|_, lot| {
            if lot.expires <= day {
                self.expired += lot.quantity;
                false
            } else {
                true
            }
        });
        self.shipments.retain(|_, shipment| {
            if shipment.expires <= day {
                self.expired += shipment.quantity;
                false
            } else {
                true
            }
        });
        self.day = Some(day);
        Ok(())
    }
    /// Explicit boundary supply, not endogenous production or an issuance event.
    pub fn supply(&mut self, id: u64, quantity: u64, expires: u32) -> Result<(), MathError> {
        if quantity == 0 || expires <= self.today()? || self.lot_ids.contains(&id) {
            return Err(MathError::Invalid("invalid or duplicate dated supply"));
        }
        let supplied = self
            .supplied
            .checked_add(quantity)
            .ok_or(MathError::Overflow)?;
        self.lot_ids.insert(id);
        self.lots
            .insert((Location::Source, id), Lot { quantity, expires });
        self.supplied = supplied;
        Ok(())
    }
    pub fn dispatch(
        &mut self,
        id: u64,
        lot: u64,
        quantity: u64,
        due: u32,
    ) -> Result<(), MathError> {
        let source = self
            .lots
            .get(&(Location::Source, lot))
            .ok_or(MathError::Invalid("unknown source lot"))?;
        if quantity == 0
            || quantity > source.quantity
            || due <= self.today()?
            || due >= source.expires
            || self.shipment_ids.contains(&id)
        {
            return Err(MathError::Invalid(
                "invalid dispatch or expired planned arrival",
            ));
        }
        let expires = source.expires;
        self.lots
            .get_mut(&(Location::Source, lot))
            .unwrap()
            .quantity -= quantity;
        self.shipment_ids.insert(id);
        self.shipments.insert(
            id,
            Shipment {
                lot,
                quantity,
                expires,
                due,
            },
        );
        Ok(())
    }
    /// Explicit observed loss; no inferred replacement supply or payment.
    pub fn damage(&mut self, id: u64, quantity: u64) -> Result<(), MathError> {
        let shipment = self
            .shipments
            .get_mut(&id)
            .ok_or(MathError::Invalid("unknown or closed shipment"))?;
        if quantity == 0 || quantity > shipment.quantity {
            return Err(MathError::Invalid("invalid transport loss"));
        }
        shipment.quantity -= quantity;
        self.transport_loss += quantity;
        Ok(())
    }
    /// Late arrival is allowed only while the original lot remains unexpired.
    pub fn receive(&mut self, id: u64) -> Result<u64, MathError> {
        let shipment = self
            .shipments
            .get(&id)
            .ok_or(MathError::Invalid("unknown or closed shipment"))?;
        if shipment.due > self.today()? {
            return Err(MathError::Invalid("arrival before transit completion"));
        }
        let shipment = self.shipments.remove(&id).unwrap();
        let lot = self
            .lots
            .entry((Location::Destination, shipment.lot))
            .or_insert(Lot {
                quantity: 0,
                expires: shipment.expires,
            });
        lot.quantity += shipment.quantity;
        Ok(shipment.quantity)
    }
    pub fn due_shipments(&self) -> Vec<(u64, u64)> {
        self.shipments
            .iter()
            .filter(|(_, s)| Some(s.due) <= self.day)
            .map(|(&id, s)| (id, s.quantity))
            .collect()
    }
    pub fn source_lots(&self) -> Vec<(u64, u64, u32)> {
        let mut lots: Vec<_> = self
            .lots
            .iter()
            .filter(|((location, _), l)| *location == Location::Source && l.quantity > 0)
            .map(|((_, id), l)| (*id, l.quantity, l.expires))
            .collect();
        lots.sort_by_key(|&(id, _, expires)| (expires, id));
        lots
    }
    /// First-expiring-first-out (FEFO). Returns actual provision, never a promise.
    pub fn consume(&mut self, requested: u64) -> Result<u64, MathError> {
        self.today()?;
        let mut order: Vec<_> = self
            .lots
            .iter()
            .filter(|((location, _), _)| *location == Location::Destination)
            .map(|(&key, l)| (l.expires, key))
            .collect();
        order.sort();
        let mut remaining = requested;
        for (_, key) in order {
            let lot = self.lots.get_mut(&key).unwrap();
            let used = remaining.min(lot.quantity);
            lot.quantity -= used;
            remaining -= used;
            if remaining == 0 {
                break;
            }
        }
        let delivered = requested - remaining;
        self.consumed += delivered;
        Ok(delivered)
    }
    pub fn balance(&self) -> LotBalance {
        let sum = |location| {
            self.lots
                .iter()
                .filter(|((l, _), _)| *l == location)
                .map(|(_, lot)| lot.quantity)
                .sum()
        };
        LotBalance {
            supplied: self.supplied,
            consumed: self.consumed,
            expired: self.expired,
            transport_loss: self.transport_loss,
            source: sum(Location::Source),
            destination: sum(Location::Destination),
            in_transit: self.shipments.values().map(|s| s.quantity).sum(),
        }
    }
}

/// One pre-existing standby unit. Calendar maintenance is a declared assumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandbyUnit {
    day: Option<u32>,
    age: u32,
    interval: u32,
    service_duration: u32,
    remaining: u32,
    capacity: u64,
    spare_cost: u64,
    labour_per_period: u64,
}
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MaintenanceStep {
    pub available_capacity: u64,
    pub spares_used: u64,
    pub labour_used: u64,
    pub service_remaining: u32,
    pub age_at_close: u32,
}
impl StandbyUnit {
    pub fn new(
        age: u32,
        interval: u32,
        service_duration: u32,
        capacity: u64,
        spare_cost: u64,
        labour_per_period: u64,
    ) -> Result<Self, MathError> {
        if interval == 0
            || service_duration == 0
            || capacity == 0
            || spare_cost == 0
            || labour_per_period == 0
        {
            return Err(MathError::Invalid(
                "maintenance parameters must be positive",
            ));
        }
        Ok(Self {
            day: None,
            age,
            interval,
            service_duration,
            remaining: 0,
            capacity,
            spare_cost,
            labour_per_period,
        })
    }
    /// Failed starts consume nothing. Ongoing service pauses without labour;
    /// a unit cannot operate during service, while overdue, or during an outage.
    pub fn step(
        &mut self,
        day: u32,
        request_service: bool,
        spares: u64,
        labour: u64,
        failed: bool,
    ) -> Result<MaintenanceStep, MathError> {
        let expected = match self.day {
            None => 0,
            Some(d) => d.checked_add(1).ok_or(MathError::Overflow)?,
        };
        if expected != day {
            return Err(MathError::Invalid("noncontiguous maintenance period"));
        }
        let mut remaining = self.remaining;
        let mut spares_used = 0;
        if request_service
            && remaining == 0
            && spares >= self.spare_cost
            && labour >= self.labour_per_period
        {
            remaining = self.service_duration;
            spares_used = self.spare_cost;
        }
        let servicing = remaining > 0;
        let available_capacity = if !servicing && self.age < self.interval && !failed {
            self.capacity
        } else {
            0
        };
        let labour_used = if servicing && labour >= self.labour_per_period {
            self.labour_per_period
        } else {
            0
        };
        if labour_used > 0 {
            remaining -= 1;
        }
        let age = if servicing && remaining == 0 {
            0
        } else if servicing {
            self.age
        } else {
            self.age.checked_add(1).ok_or(MathError::Overflow)?
        };
        self.day = Some(day);
        self.age = age;
        self.remaining = remaining;
        Ok(MaintenanceStep {
            available_capacity,
            spares_used,
            labour_used,
            service_remaining: remaining,
            age_at_close: age,
        })
    }
    pub fn service_due(&self) -> bool {
        self.age >= self.interval || self.remaining > 0
    }
}
