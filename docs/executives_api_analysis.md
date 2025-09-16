# Financial Modeling Prep Executives API Analysis

## API Endpoint
```
https://financialmodelingprep.com/api/v3/key-executives/{ticker}?apikey={api_key}
```

## Available Data Fields

### Core Fields (Always Present)
| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `title` | String | Executive's job title | "Chief Executive Officer & Director" |
| `name` | String | Executive's full name | "Mr. Timothy D. Cook" |
| `currencyPay` | String | Currency for compensation | "USD" |

### Optional Fields (Sometimes Present)
| Field | Type | Description | Example | Availability |
|-------|------|-------------|---------|--------------|
| `pay` | Number/null | Annual compensation | 16239562 | ~60% of executives |
| `gender` | String/null | Gender identification | "male", "female" | ~90% of executives |
| `yearBorn` | Number/null | Birth year | 1961 | ~60% of executives |
| `titleSince` | String/null | Date assumed current role | "2021-01-15" | Rarely provided |

## Current Implementation

Currently extracting:
- **CEO Name**: Successfully identifying and extracting CEO by searching for titles containing "Chief Executive" or "CEO"

## Potential Additional Extractions

### 1. CEO Compensation (HIGH VALUE)
```rust
// Add to Details struct
pub ceo_compensation: Option<f64>,
pub ceo_compensation_currency: Option<String>,
```
**Use Case**: Corporate governance analysis, peer comparison, compensation trends

### 2. Executive Team Size (MEDIUM VALUE)
```rust
// Add to Details struct
pub executive_count: Option<i32>,
```
**Use Case**: Organizational complexity indicator, management efficiency metrics

### 3. Gender Diversity Metrics (HIGH VALUE - ESG)
```rust
// Add to Details struct
pub female_executive_percentage: Option<f64>,
pub female_in_c_suite: Option<bool>,
```
**Use Case**: ESG reporting, diversity metrics, governance scores

### 4. C-Suite Leadership Team (HIGH VALUE)
```rust
// Add to Details struct
pub cfo_name: Option<String>,
pub coo_name: Option<String>,
pub cto_name: Option<String>,
```
**Use Case**: Complete leadership picture, succession planning analysis

### 5. CEO Age/Experience (MEDIUM VALUE)
```rust
// Add to Details struct
pub ceo_age: Option<i32>,  // Calculated from yearBorn
pub ceo_tenure_years: Option<i32>,  // If titleSince available
```
**Use Case**: Leadership stability, succession planning indicators

## Sample Data Structure

```json
{
  "title": "Chief Executive Officer & Director",
  "name": "Mr. Timothy D. Cook",
  "pay": 16239562,
  "currencyPay": "USD",
  "gender": "male",
  "yearBorn": 1961,
  "titleSince": null
}
```

## Implementation Recommendations

### Priority 1 (High Impact, Easy Implementation)
- [ ] CEO Compensation - Direct financial metric
- [ ] Executive Count - Simple counting
- [ ] CFO Name - Important for financial leadership

### Priority 2 (Medium Impact, Moderate Complexity)
- [ ] Gender Diversity Percentage - Requires calculation
- [ ] CEO Age - Requires yearBorn and calculation
- [ ] COO/CTO Names - Pattern matching on titles

### Priority 3 (Lower Impact or Complex)
- [ ] CEO Tenure - Rarely available data
- [ ] Full executive team details - Large data structure
- [ ] Historical executive changes - Would require tracking

## Usage in Existing Code

The executives data is fetched in parallel with other API calls in `src/api.rs`:

```rust
let executives_url = format!(
    "https://financialmodelingprep.com/api/v3/key-executives/{}?apikey={}",
    ticker, self.api_key
);

// Fetched in parallel with profile, ratios, and income data
let executives = self.make_request::<Vec<FMPExecutive>>(executives_url)?;

// Currently only extracting CEO name
let ceo_name = executives.iter()
    .find(|exec| exec.title.to_lowercase().contains("chief executive") || 
                 exec.title.to_lowercase().contains("ceo"))
    .map(|exec| exec.name.clone());
```

## Database Migration Requirements

For each new field added, you would need:
1. Database migration to add column(s)
2. Update `TickerDetails` struct
3. Update SQL queries in relevant modules
4. Regenerate SQLx offline query cache
5. Update CSV export headers and data

## Example Companies Tested

- **AAPL**: 10 executives, CEO Tim Cook ($16.2M compensation)
- **MSFT**: 10 executives, CEO Satya Nadella ($7.9M compensation)
- **NKE**: 10 executives, CEO Elliott Hill ($3.7M compensation)
- **LULU**: 10 executives, CEO Calvin McDonald ($3.6M compensation)
- **TJX**: 10 executives, CEO Ernie Herrman ($9.3M compensation)

## Notes

- The API typically returns 9-10 executives per company
- Not all companies have complete data for all fields
- Some companies may have multiple CEOs listed (e.g., interim, co-CEOs)
- Compensation data is typically from the most recent proxy filing
- The `titleSince` field is rarely populated in the current data