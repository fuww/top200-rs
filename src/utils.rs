// SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::collections::HashMap;

pub fn convert_currency(
    amount: f64,
    from_currency: &str,
    to_currency: &str,
    rate_map: &HashMap<String, f64>,
) -> f64 {
    if from_currency == to_currency {
        return amount;
    }

    // Handle special cases for currency subunits and alternative codes
    let (adjusted_amount, adjusted_from_currency) = match from_currency {
        "GBp" => (amount / 100.0, "GBP"), // Convert pence to pounds
        "ZAc" => (amount / 100.0, "ZAR"),
        "ILA" => (amount, "ILS"),
        _ => (amount, from_currency),
    };

    // Adjust target currency if needed
    let adjusted_to_currency = match to_currency {
        "GBp" => "GBP", // Also handle GBp as target currency
        "ZAc" => "ZAR", // Also handle ZAc as target currency
        "ILA" => "ILS",
        _ => to_currency,
    };

    // Try direct conversion first
    let direct_rate = format!("{}/{}", adjusted_from_currency, adjusted_to_currency);
    if let Some(&rate) = rate_map.get(&direct_rate) {
        let result = adjusted_amount * rate;
        return match to_currency {
            "GBp" => result * 100.0,
            "ZAc" => result * 100.0,
            _ => result,
        };
    }

    // Try reverse rate
    let reverse_rate = format!("{}/{}", adjusted_to_currency, adjusted_from_currency);
    if let Some(&rate) = rate_map.get(&reverse_rate) {
        let result = adjusted_amount * (1.0 / rate);
        return match to_currency {
            "GBp" => result * 100.0,
            "ZAc" => result * 100.0,
            _ => result,
        };
    }

    // If no conversion rate is found, return the original amount
    amount
}
