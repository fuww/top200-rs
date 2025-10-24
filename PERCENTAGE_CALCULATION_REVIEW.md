# Market Cap Comparison Percentage Calculation Review

## Summary

I've thoroughly reviewed the percentage calculation code in `src/compare_marketcaps.rs` and **the calculation logic is mathematically correct**. However, I've added debug logging to help identify potential data quality issues.

## Code Review Findings

### ✅ Percentage Calculation (Lines 206-217)

The percentage calculation is **CORRECT**:

```rust
let abs_change = to_val - from_val;
let pct_change = if from_val != 0.0 {
    (abs_change / from_val) * 100.0  // ← Standard percentage formula
} else {
    0.0
};
```

**Formula**: `((new_value - old_value) / old_value) × 100`

### ✅ Tests Added

Added comprehensive tests to verify calculation accuracy:
- `test_market_cap_comparison_calculation()` - Basic 10% change
- `test_market_cap_large_realistic_values()` - $3T → $3.3T (10% change)
- `test_market_cap_small_percentage()` - $1T → $1.02T (2% change)
- `test_market_cap_comparison_negative_change()` - Negative percentage test

All tests pass ✓

### ✅ Display Formatting (Line 443)

The markdown output format is correct:
```rust
"+{:.2}%"  // Formats as +10.00% for a 10% gain
```

The percentage value is already multiplied by 100 in the calculation, so the display format is appropriate.

## Potential Issues to Investigate

### 1. Data Consistency Between Files

**Hypothesis**: The two CSV files being compared might have inconsistent data formats.

**Scenarios that could cause "way too high" percentages**:

**Scenario A**: Mixed units (one file in dollars, another incorrectly scaled)
```
From file: 3,000,000 (should be $3T but missing zeros)
To file:   3,300,000,000,000 (correctly $3.3T)
Result: 109,999,900% gain! ← WRONG
```

**Scenario B**: API returned data in different format on different dates
```
From file: Values in millions from API
To file:   Values in full dollars from API
Result: Percentages off by factor of 1,000,000
```

### 2. Debugging Changes Made

I've added extensive debug logging to help diagnose the issue:

1. **Sample value logging** (lines 194-210):
   - Prints first 3 ticker values from both dates
   - Shows values in both full dollars and billions
   - Calculates and displays percentage change

2. **Top gainers logging** (lines 423-447):
   - Prints detailed breakdown of top 3 gainers
   - Shows from/to values, absolute change, and percentage

This debug output will print to stderr when running the comparison.

## Recommended Actions

### To use the debug output:

```bash
# Run comparison and see debug info
cargo run -- compare-market-caps --from 2025-XX-XX --to 2025-YY-YY 2>&1 | tee comparison_debug.log
```

### What to look for in debug output:

1. **Are market cap values realistic?**
   - Apple should be ~$3 trillion ($3,000,000,000,000)
   - Not $3 million ($3,000,000) or $3 billion ($3,000,000,000)

2. **Are values consistent between dates?**
   - Same company should have similar magnitude between dates
   - A 10% change should be reasonable for market movements

3. **Do percentages make sense?**
   - Normal market changes: -50% to +200%
   - Suspicious: +10,000% or +1,000,000%

## Files Modified

- `src/compare_marketcaps.rs`:
  - Added debug logging at lines 194-210 (sample values)
  - Added debug logging at lines 423-447 (top gainers)
  - Added 2 new test cases for realistic scenarios

## Next Steps

1. **Run a comparison with debug output enabled** to see actual values
2. **Check the raw CSV files** to verify market cap values are correct
3. **Compare with known reference** (e.g., check Apple's current market cap on finance.yahoo.com)
4. If values are confirmed correct, the percentages should also be correct
5. If values are in wrong units, the issue is in the data fetching/storage, not the comparison logic

## Conclusion

The percentage calculation code is **mathematically sound**. If you're seeing "way too high" percentages, the issue is most likely:

1. **Data quality issue** - CSV files contain incorrect values
2. **Data consistency issue** - Two files have different unit scales
3. **Interpretation issue** - Maybe the percentages are actually correct but seem surprising

The debug logging I've added will help identify which of these is the actual problem.
