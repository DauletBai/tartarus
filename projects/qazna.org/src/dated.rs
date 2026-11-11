//! Offline identity-preserving dated flow. A trusted checkpoint anchor is not
//! physical attestation, a digital signature, or permission to change a ledger.
use crate::mathematics::MathError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const MAX_ITEMS: usize = 100_000;
const MAX_BYTES: usize = 8_000_000;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Origin {
    resource: usize,
    quantity: u64,
    available: u32,
    expires: u32,
    basis: String,
    released: bool,
    withdrawn: u64,
    expired: u64,
    damaged: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Movement {
    origin: u64,
    target: usize,
    quantity: u64,
    due: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct State {
    version: u32,
    scope: String,
    unit: String,
    resources: usize,
    period: Option<u32>,
    revision: u64,
    next_movement: u64,
    origins: BTreeMap<u64, Origin>,
    stocks: BTreeMap<usize, BTreeMap<u64, u64>>,
    transit: BTreeMap<u64, Movement>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatedFlow {
    state: State,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct DatedBalance {
    pub revision: u64,
    pub period: Option<u32>,
    pub released: u128,
    pub committed_births: u128,
    pub withdrawn: u128,
    pub expired: u128,
    pub damaged: u128,
    pub in_transit: u128,
    pub stock_by_resource: Vec<u128>,
    pub origin_count: usize,
    pub pending_movements: usize,
}
impl DatedFlow {
    pub fn new(scope: &str, unit: &str, resources: usize) -> Result<Self, MathError> {
        let x = Self {
            state: State {
                version: 1,
                scope: scope.into(),
                unit: unit.into(),
                resources,
                period: None,
                revision: 0,
                next_movement: 0,
                origins: BTreeMap::new(),
                stocks: BTreeMap::new(),
                transit: BTreeMap::new(),
            },
        };
        x.validate()?;
        Ok(x)
    }
    fn today(&self) -> Result<u32, MathError> {
        self.state
            .period
            .ok_or(MathError::Invalid("begin dated period first"))
    }
    fn validate(&self) -> Result<(), MathError> {
        let s = &self.state;
        if s.version != 1
            || s.scope.trim().is_empty()
            || s.unit.trim().is_empty()
            || s.scope.len() > 1024
            || s.unit.len() > 128
            || !(1..=64).contains(&s.resources)
            || s.origins.len() > MAX_ITEMS
            || s.transit.len() > MAX_ITEMS
            || s.stocks.len() > s.resources
        {
            return Err(MathError::Invalid("dated checkpoint schema or limits"));
        }
        if s.period.is_none()
            && (!s.origins.is_empty()
                || !s.stocks.is_empty()
                || !s.transit.is_empty()
                || s.revision != 0
                || s.next_movement != 0)
        {
            return Err(MathError::Invalid("unstarted dated state"));
        }
        let mut present = BTreeMap::<u64, u128>::new();
        for (&resource, lots) in &s.stocks {
            if resource >= s.resources || lots.len() > MAX_ITEMS {
                return Err(MathError::Invalid("dated location"));
            }
            for (&id, &q) in lots {
                let o = s
                    .origins
                    .get(&id)
                    .ok_or(MathError::Invalid("unknown stock origin"))?;
                if q == 0 || !o.released || Some(o.expires) <= s.period {
                    return Err(MathError::Invalid("unusable dated stock"));
                }
                *present.entry(id).or_default() += u128::from(q);
            }
        }
        for (&id, m) in &s.transit {
            let o = s
                .origins
                .get(&m.origin)
                .ok_or(MathError::Invalid("unknown transit origin"))?;
            if id >= s.next_movement
                || m.target >= s.resources
                || m.quantity == 0
                || !o.released
                || Some(m.due) <= s.period
                || m.due >= o.expires
            {
                return Err(MathError::Invalid("dated movement identity or age"));
            }
            *present.entry(m.origin).or_default() += u128::from(m.quantity);
        }
        for (&id, o) in &s.origins {
            if o.resource >= s.resources
                || o.quantity == 0
                || o.expires <= o.available
                || o.basis.trim().is_empty()
                || o.basis.len() > 2048
                || o.released != (Some(o.available) <= s.period)
            {
                return Err(MathError::Invalid("dated origin provenance or release"));
            }
            let accounted = present.get(&id).copied().unwrap_or(0)
                + u128::from(o.withdrawn)
                + u128::from(o.expired)
                + u128::from(o.damaged);
            if accounted
                != if o.released {
                    u128::from(o.quantity)
                } else {
                    0
                }
            {
                return Err(MathError::Infeasible(format!(
                    "dated origin conservation:{id}"
                )));
            }
        }
        Ok(())
    }
    fn transaction<F>(&mut self, f: F) -> Result<(), MathError>
    where
        F: FnOnce(&mut Self) -> Result<(), MathError>,
    {
        let mut next = self.clone();
        f(&mut next)?;
        next.state.revision = next
            .state
            .revision
            .checked_add(1)
            .ok_or(MathError::Overflow)?;
        next.validate()?;
        *self = next;
        Ok(())
    }
    fn add_stock(&mut self, r: usize, id: u64, q: u64) -> Result<(), MathError> {
        let v = self
            .state
            .stocks
            .entry(r)
            .or_default()
            .entry(id)
            .or_default();
        *v = v.checked_add(q).ok_or(MathError::Overflow)?;
        Ok(())
    }
    /// Complete prior commitments and expire stock before new actions. Periods
    /// are absolute and contiguous; expiry is exclusive, including in transit.
    pub fn advance(&mut self, period: u32) -> Result<(), MathError> {
        self.transaction(|x| {
            let expected = match x.state.period {
                None => 0,
                Some(t) => t.checked_add(1).ok_or(MathError::Overflow)?,
            };
            if period != expected {
                return Err(MathError::Invalid("noncontiguous dated period"));
            }
            x.state.period = Some(period);
            let origins: Vec<_> = x
                .state
                .origins
                .iter()
                .map(|(&id, o)| (id, o.clone()))
                .collect();
            for (id, o) in origins {
                if !o.released && o.available == period {
                    x.state.origins.get_mut(&id).unwrap().released = true;
                    x.add_stock(o.resource, id, o.quantity)?;
                }
            }
            let moving = std::mem::take(&mut x.state.transit);
            for (id, m) in moving {
                if m.due == period {
                    x.add_stock(m.target, m.origin, m.quantity)?;
                } else {
                    x.state.transit.insert(id, m);
                }
            }
            for lots in x.state.stocks.values_mut() {
                let expired: Vec<_> = lots
                    .iter()
                    .filter(|(id, _)| x.state.origins[id].expires <= period)
                    .map(|(&id, &q)| (id, q))
                    .collect();
                for (id, q) in expired {
                    lots.remove(&id);
                    let o = x.state.origins.get_mut(&id).unwrap();
                    o.expired = o.expired.checked_add(q).ok_or(MathError::Overflow)?;
                }
            }
            Ok(())
        })
    }
    /// A declared opening lot or committed new output. Original expiry never
    /// changes. A future birth is not counted as released stock until available.
    pub fn introduce(
        &mut self,
        id: u64,
        resource: usize,
        quantity: u64,
        available: u32,
        expires: u32,
        basis: &str,
    ) -> Result<(), MathError> {
        self.transaction(|x| {
            let now = x.today()?;
            if resource >= x.state.resources
                || quantity == 0
                || available < now
                || expires <= available
                || x.state.origins.contains_key(&id)
            {
                return Err(MathError::Invalid("invalid or reused dated origin"));
            }
            x.state.origins.insert(
                id,
                Origin {
                    resource,
                    quantity,
                    available,
                    expires,
                    basis: basis.into(),
                    released: available == now,
                    withdrawn: 0,
                    expired: 0,
                    damaged: 0,
                },
            );
            if available == now {
                x.add_stock(resource, id, quantity)?;
            }
            Ok(())
        })
    }
    fn take(
        &mut self,
        resource: usize,
        quantity: u64,
        valid_at: u32,
    ) -> Result<Vec<(u64, u64)>, MathError> {
        if resource >= self.state.resources || quantity == 0 {
            return Err(MathError::Invalid("dated selection"));
        }
        let mut order: Vec<_> = self
            .state
            .stocks
            .get(&resource)
            .into_iter()
            .flat_map(|lots| lots.iter())
            .filter(|(id, _)| self.state.origins[id].expires > valid_at)
            .map(|(&id, &q)| (self.state.origins[&id].expires, id, q))
            .collect();
        order.sort();
        if order.iter().map(|(_, _, q)| u128::from(*q)).sum::<u128>() < u128::from(quantity) {
            return Err(MathError::Invalid("insufficient age-qualified dated stock"));
        }
        let mut remaining = quantity;
        let mut taken = vec![];
        for (_, id, q) in order {
            let n = q.min(remaining);
            if n == 0 {
                break;
            }
            let lots = self.state.stocks.get_mut(&resource).unwrap();
            if n == q {
                lots.remove(&id);
            } else {
                *lots.get_mut(&id).unwrap() -= n;
            }
            taken.push((id, n));
            remaining -= n;
        }
        Ok(taken)
    }
    /// Identity transfer or processing route: quantities and original expiry
    /// survive. Conversion yields belong in a separately validated recipe.
    pub fn transfer(
        &mut self,
        source: usize,
        target: usize,
        quantity: u64,
        due: u32,
    ) -> Result<(), MathError> {
        self.transaction(|x| {
            if source == target || target >= x.state.resources || due <= x.today()? {
                return Err(MathError::Invalid("dated route or arrival"));
            }
            for (origin, q) in x.take(source, quantity, due)? {
                let id = x.state.next_movement;
                x.state.next_movement = id.checked_add(1).ok_or(MathError::Overflow)?;
                x.state.transit.insert(
                    id,
                    Movement {
                        origin,
                        target,
                        quantity: q,
                        due,
                    },
                );
            }
            Ok(())
        })
    }
    /// Seeds, processing inputs or actual resident consumption are distinct
    /// caller actions; this counter does not claim all withdrawals feed people.
    pub fn withdraw(&mut self, resource: usize, quantity: u64) -> Result<(), MathError> {
        self.transaction(|x| {
            for (id, q) in x.take(resource, quantity, x.today()?)? {
                let o = x.state.origins.get_mut(&id).unwrap();
                o.withdrawn = o.withdrawn.checked_add(q).ok_or(MathError::Overflow)?;
            }
            Ok(())
        })
    }
    pub fn damage(&mut self, movement: u64, quantity: u64) -> Result<(), MathError> {
        self.transaction(|x| {
            let m = x
                .state
                .transit
                .get_mut(&movement)
                .ok_or(MathError::Invalid("unknown dated movement"))?;
            if quantity == 0 || quantity > m.quantity {
                return Err(MathError::Invalid("dated transport damage"));
            }
            m.quantity -= quantity;
            let origin = m.origin;
            if m.quantity == 0 {
                x.state.transit.remove(&movement);
            }
            let o = x.state.origins.get_mut(&origin).unwrap();
            o.damaged = o.damaged.checked_add(quantity).ok_or(MathError::Overflow)?;
            Ok(())
        })
    }
    pub fn balance(&self) -> DatedBalance {
        let mut b = DatedBalance {
            revision: self.state.revision,
            period: self.state.period,
            released: 0,
            committed_births: 0,
            withdrawn: 0,
            expired: 0,
            damaged: 0,
            in_transit: 0,
            stock_by_resource: vec![0; self.state.resources],
            origin_count: self.state.origins.len(),
            pending_movements: self.state.transit.len(),
        };
        for o in self.state.origins.values() {
            if o.released {
                b.released += u128::from(o.quantity);
            } else {
                b.committed_births += u128::from(o.quantity);
            }
            b.withdrawn += u128::from(o.withdrawn);
            b.expired += u128::from(o.expired);
            b.damaged += u128::from(o.damaged);
        }
        for (&r, lots) in &self.state.stocks {
            b.stock_by_resource[r] = lots.values().map(|&q| u128::from(q)).sum();
        }
        b.in_transit = self
            .state
            .transit
            .values()
            .map(|m| u128::from(m.quantity))
            .sum();
        b
    }
    pub fn checkpoint(&self) -> Result<Vec<u8>, MathError> {
        self.validate()?;
        let bytes = serde_json::to_vec(&self.state)
            .map_err(|_| MathError::Invalid("dated checkpoint encoding"))?;
        if bytes.len() > MAX_BYTES {
            return Err(MathError::Invalid("dated checkpoint size"));
        }
        Ok(bytes)
    }
    /// expected_bytes must come from an independently trusted, current store,
    /// never from the same untrusted sender as payload. Exact matching avoids a
    /// new crypto scheme; this offline helper does not implement that store.
    pub fn restore(
        payload: &[u8],
        expected_bytes: &[u8],
        scope: &str,
        revision: u64,
    ) -> Result<Self, MathError> {
        if payload.len() > MAX_BYTES || payload != expected_bytes {
            return Err(MathError::Invalid("changed dated checkpoint"));
        }
        let state: State = serde_json::from_slice(payload)
            .map_err(|_| MathError::Invalid("dated checkpoint encoding"))?;
        let flow = Self { state };
        flow.validate()?;
        if flow.state.scope != scope || flow.state.revision != revision {
            return Err(MathError::Invalid("stale or foreign dated checkpoint"));
        }
        if flow.checkpoint()?.as_slice() != payload {
            return Err(MathError::Invalid("noncanonical dated checkpoint"));
        }
        Ok(flow)
    }
}
