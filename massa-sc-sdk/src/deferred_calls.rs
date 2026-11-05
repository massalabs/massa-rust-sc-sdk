//! Deferred calls: book the execution of a function in a precise future slot
//! (guaranteed autonomous smart contracts). The booking price depends on the
//! slot demand, the gas and the parameter size; it is paid when registering.

use alloc::string::String;

use crate::constants::THREAD_COUNT;
use crate::context::Slot;
use crate::{env, Address};

/// Price in nanoMAS of booking a call in `slot` with `max_gas` and a
/// serialized parameter of `params_size` bytes. `0` when the slot has no room
/// left (or is out of the bookable range).
#[inline]
pub fn quote(slot: Slot, max_gas: u64, params_size: u64) -> u64 {
    env::deferred_call_quote(slot.into(), max_gas, params_size)
}

/// Book the execution of `function` at `address` in `slot`, sending `coins`
/// along. The booking fee ([`quote`]) is paid by the current contract now.
/// Returns the deferred call id (keep it to [`cancel`] or check [`exists`]).
pub fn register(
    address: &Address,
    function: &str,
    slot: Slot,
    max_gas: u64,
    args: impl AsRef<[u8]>,
    coins: u64,
) -> String {
    env::deferred_call_register(address.as_str(), function, slot.into(), max_gas, args.as_ref(), coins)
}

/// `true` while the call is scheduled (not yet executed nor cancelled).
#[inline]
pub fn exists(id: &str) -> bool {
    env::deferred_call_exists(id)
}

/// Cancel a scheduled call: the coins are returned, the booking fee is not.
#[inline]
pub fn cancel(id: &str) {
    env::deferred_call_cancel(id)
}

/// Cheapest *available* slot to book within `[start_period, end_period]`
/// (all threads), or `None` when every slot of the range is full. Ties go to
/// the earliest slot. Each quote costs gas: keep the range small.
pub fn find_cheapest_slot(
    start_period: u64,
    end_period: u64,
    max_gas: u64,
    params_size: u64,
) -> Option<Slot> {
    let mut best: Option<(Slot, u64)> = None;
    for period in start_period..=end_period {
        for thread in 0..THREAD_COUNT {
            let slot = Slot::new(period, thread);
            let price = quote(slot, max_gas, params_size);
            if price == 0 {
                continue; // full / not bookable
            }
            if best.is_none_or(|(_, p)| price < p) {
                best = Some((slot, price));
            }
        }
    }
    best.map(|(slot, _)| slot)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock;

    #[test]
    fn book_and_cancel() {
        mock::reset();
        let target = Address::from("AS1target");
        let slot = find_cheapest_slot(10, 11, 1_000_000, 0).unwrap();
        assert_eq!(slot.period, 10);
        let id = register(&target, "tick", slot, 1_000_000, [], 0);
        assert!(exists(&id));
        cancel(&id);
        assert!(!exists(&id));
    }
}
