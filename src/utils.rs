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
        "GBp" => (amount / 100.0, "GBP"),  // Convert pence to pounds
        "ZAc" => (amount / 100.0, "ZAR"),
        "ILA" => (amount, "ILS"),
        _ => (amount, from_currency),
    };

    // Adjust target currency if needed
    let adjusted_to_currency = match to_currency {
        "GBp" => "GBP",  // Also handle GBp as target currency
        "ZAc" => "ZAR",  // Also handle ZAc as target currency
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

    // If no direct or reverse rate, try via USD
    let to_usd = format!("{}/USD", adjusted_from_currency);
    let from_usd = format!("USD/{}", adjusted_to_currency);
    
    let usd_to_target = if adjusted_to_currency == "USD" {
        1.0
    } else if let Some(&rate) = rate_map.get(&from_usd) {
        rate
    } else if let Some(&rate) = rate_map.get(&format!("{}/USD", adjusted_to_currency)) {
        1.0 / rate
    } else {
        println!("⚠️  Warning: No conversion rate found for USD to {}", adjusted_to_currency);
        1.0 // Return 1.0 as fallback
    };

    // Do the conversion
    if let Some(&rate) = rate_map.get(&to_usd) {
        let result = adjusted_amount * rate * usd_to_target;
        // Convert back to pence or cents if needed
        match to_currency {
            "GBp" => result * 100.0,
            "ZAc" => result * 100.0,
            _ => result,
        }
    } else if let Some(&rate) = rate_map.get(&format!("USD/{}", adjusted_from_currency)) {
        let result = adjusted_amount * (1.0 / rate) * usd_to_target;
        // Convert back to pence or cents if needed
        match to_currency {
            "GBp" => result * 100.0,
            "ZAc" => result * 100.0,
            _ => result,
        }
    } else {
        println!("⚠️  Warning: No conversion rate found for {} to USD", adjusted_from_currency);
        amount // Return original amount as fallback
    }
}
