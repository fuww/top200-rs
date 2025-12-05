# Implementation Plan: Web Interface for Top200-rs

## Overview
Add a web interface to the existing CLI application using Axum, Askama templating, Datastar for reactivity, and WorkOS for authentication. The web interface will expose all major CLI functionality including viewing reports, generating comparisons, fetching data, and advanced analytics.

## Requirements Summary
- **Scope**: Full-featured web interface (view reports, generate comparisons, fetch data, analytics)
- **Authentication**: WorkOS with role-based access (Admin/Viewer) for small team (5-20 users) - Already have WorkOS account
- **Long Operations**: Start with blocking + progress UI using Datastar SSE, refactor to background jobs later if needed
- **Deployment**: Docker container for Fly.io/Docker deployment
- **Styling**: Tailwind CSS for utility-first styling
- **Implementation Strategy**: MVP-first approach - Phases 1-6, then extend with analytics

## Architecture

### Technology Stack
- **Web Framework**: Axum (Rust async web framework)
- **Templates**: Askama (compile-time HTML templating)
- **Frontend Reactivity**: Datastar (hypermedia-driven reactivity with SSE)
- **Authentication**: WorkOS (JWT-based auth with role management)
- **Database**: Existing SQLite with SQLx
- **Deployment**: Docker container

### Key Design Decisions
1. **Dual Mode Operation**: CLI and web server modes in same binary
2. **File-based Data**: Continue using output/ directory for CSV/SVG/MD files
3. **Simple Progress**: SSE streaming for long operations (30-60s operations)
4. **Stateless API**: JWT tokens, no server-side sessions initially
5. **Role-Based Access**: Admin can trigger fetches/comparisons, Viewer is read-only

## Project Structure

```
src/
├── main.rs                         # Entry point (CLI + web server modes)
├── web/
│   ├── mod.rs                      # Web module exports
│   ├── server.rs                   # Axum server setup and router
│   ├── routes/
│   │   ├── mod.rs                  # Route module exports
│   │   ├── pages.rs                # HTML page rendering endpoints
│   │   ├── api.rs                  # JSON API endpoints
│   │   └── auth.rs                 # Authentication routes (login/logout)
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── auth.rs                 # JWT validation middleware
│   │   └── roles.rs                # Role-based access control
│   ├── models/
│   │   ├── mod.rs
│   │   ├── api_requests.rs         # API request types
│   │   ├── api_responses.rs        # API response types
│   │   └── auth.rs                 # Auth-related types (Claims, User, Role)
│   ├── state.rs                    # AppState (DB pool, WorkOS client, config)
│   └── utils.rs                    # Web utilities (file listing, parsing)
├── templates/
│   ├── base.html                   # Base layout with header/nav
│   ├── login.html                  # Login page
│   ├── dashboard.html              # Main dashboard with overview
│   ├── comparisons/
│   │   ├── list.html               # List available comparisons
│   │   ├── view.html               # View comparison details with charts
│   │   └── new.html                # Form to generate new comparison
│   ├── market_caps/
│   │   ├── list.html               # List available market cap snapshots
│   │   ├── view.html               # View market caps for a date
│   │   └── fetch.html              # Form to fetch market caps for a date
│   ├── analytics/
│   │   ├── trends.html             # Multi-date trend analysis
│   │   ├── yoy.html                # Year-over-year comparisons
│   │   ├── qoq.html                # Quarter-over-quarter
│   │   ├── rolling.html            # Rolling period comparisons
│   │   ├── benchmarks.html         # Benchmark comparisons
│   │   └── peer_groups.html        # Peer group analysis
│   └── partials/
│       ├── comparison_card.html    # Comparison summary card
│       ├── chart_viewer.html       # SVG chart display component
│       ├── progress.html           # Progress indicator (for SSE updates)
│       └── table.html              # Reusable data table
├── static/
│   ├── css/
│   │   └── output.css              # Tailwind CSS compiled output
│   ├── js/
│   │   └── app.js                  # Additional client-side JS if needed
│   └── vendor/
│       └── datastar.js             # Datastar library (or use CDN)
├── tailwind.config.js              # Tailwind configuration
├── input.css                       # Tailwind input file with @tailwind directives
```

## Database Schema Changes

### New Table: users (optional, may use WorkOS as source of truth)
```sql
CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,              -- WorkOS user ID
    email TEXT NOT NULL UNIQUE,
    name TEXT,
    role TEXT NOT NULL,               -- 'admin' or 'viewer'
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### New Table: job_status (for future background jobs)
```sql
CREATE TABLE IF NOT EXISTS job_status (
    id TEXT PRIMARY KEY,
    job_type TEXT NOT NULL,           -- 'fetch', 'compare', 'trend_analysis', etc.
    status TEXT NOT NULL,             -- 'pending', 'running', 'completed', 'failed'
    parameters TEXT,                  -- JSON blob with job parameters
    result TEXT,                      -- Path to result file or error message
    created_by TEXT,                  -- User ID
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    completed_at DATETIME,
    FOREIGN KEY (created_by) REFERENCES users(id)
);
```

## Dependencies to Add to Cargo.toml

```toml
[dependencies]
# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace", "cors"] }

# Templating
askama = { version = "0.12", features = ["with-axum"] }
askama_axum = "0.4"

# Authentication
jsonwebtoken = "9.2"
workos = "0.8"  # WorkOS Rust SDK

# Already have: tokio, serde, serde_json, anyhow, chrono, sqlx
```

## API Endpoints

### Authentication (No Auth Required)
- `POST /api/auth/login` - WorkOS login (exchange code for JWT)
- `POST /api/auth/logout` - Logout (client-side token deletion)

### Pages (HTML, Auth Required)
- `GET /` - Dashboard (redirect to /login if not authenticated)
- `GET /login` - Login page
- `GET /comparisons` - List comparisons
- `GET /comparisons/new` - New comparison form (Admin only)
- `GET /comparisons/:from/:to` - View specific comparison
- `GET /market-caps` - List market cap snapshots
- `GET /market-caps/:date` - View market caps for date
- `GET /market-caps/fetch` - Fetch form (Admin only)
- `GET /analytics/trends` - Trend analysis page (Admin only)
- `GET /analytics/yoy` - YoY comparison page (Admin only)
- `GET /analytics/qoq` - QoQ comparison page (Admin only)
- `GET /analytics/rolling` - Rolling period page (Admin only)
- `GET /analytics/benchmarks` - Benchmark comparison page (Admin only)
- `GET /analytics/peer-groups` - Peer groups page

### API Endpoints (JSON, Auth Required)
- `GET /api/comparisons` - List available comparisons (metadata from output/ dir)
- `GET /api/comparisons/:from/:to` - Get comparison data (CSV parsed to JSON)
- `POST /api/comparisons` - Generate new comparison (Admin only, SSE for progress)
- `GET /api/market-caps` - List available market cap dates
- `GET /api/market-caps/:date` - Get market cap data for date
- `POST /api/market-caps/fetch` - Fetch market caps for date (Admin only, SSE)
- `GET /api/charts/:from/:to/:type` - Get chart SVG (type: gainers_losers, distribution, rank_movements, dashboard)
- `GET /api/available-dates` - List all available dates
- `GET /api/peer-groups` - List peer groups
- `GET /api/peer-groups/:name` - Get peer group details
- `POST /api/analytics/trends` - Run trend analysis (Admin only, SSE)
- `POST /api/analytics/yoy` - Run YoY comparison (Admin only, SSE)
- `POST /api/analytics/qoq` - Run QoQ comparison (Admin only, SSE)
- `POST /api/analytics/rolling` - Run rolling comparison (Admin only, SSE)
- `POST /api/analytics/benchmark` - Run benchmark comparison (Admin only, SSE)

### Static Assets
- `GET /static/*` - Serve static files (CSS, JS, images)

## Implementation Strategy: MVP First

**MVP Scope (Phases 1-6)**: Focus on delivering core functionality first
- Phase 1: Basic Axum server with Tailwind CSS
- Phase 2: WorkOS authentication with JWT
- Phase 3: View existing comparisons with charts
- Phase 4: View market cap snapshots
- Phase 5: Generate comparisons on-demand
- Phase 6: Fetch market caps on-demand

**Post-MVP (Phases 7-12)**: Advanced analytics and deployment
- Phases 7-10: Advanced analytics (trends, YoY, QoQ, rolling, benchmarks, peer groups)
- Phase 11: Docker & Fly.io deployment
- Phase 12: Polish, testing, and optimization

The MVP (Phases 1-6) delivers a fully functional web interface that covers the most common use cases. Advanced analytics can be added incrementally based on usage patterns.

---

## Implementation Phases

### Phase 1: Basic Axum Server Setup + Tailwind CSS
**Goal**: Get a minimal web server running with static file serving, Tailwind CSS, and basic templates

**Files to Create/Modify**:
- `src/web/mod.rs` - Module declaration
- `src/web/server.rs` - Axum server setup
- `src/web/state.rs` - AppState struct
- `src/web/routes/mod.rs` - Route module
- `src/web/routes/pages.rs` - Basic page routes
- `src/main.rs` - Add web server mode (e.g., `--web` flag or `serve` subcommand)
- `templates/base.html` - Base HTML template with navigation and Tailwind
- `templates/dashboard.html` - Simple dashboard page with Tailwind components
- `tailwind.config.js` - Tailwind configuration
- `input.css` - Tailwind directives (@tailwind base/components/utilities)
- `static/css/output.css` - Generated Tailwind CSS (via npx/build script)
- `package.json` - Node dependencies for Tailwind
- `Cargo.toml` - Add axum, tower-http, askama dependencies

**Implementation Steps**:
1. Add Rust dependencies to Cargo.toml (axum, tower-http, askama, askama_axum)
2. Set up Tailwind CSS:
   - Create package.json with tailwindcss dependency
   - Create tailwind.config.js (content: templates/**/*.html)
   - Create input.css with @tailwind directives
   - Add build script to generate output.css
3. Create web module structure (src/web/mod.rs)
4. Create AppState with DB pool and config
5. Set up basic Axum router with:
   - Health check endpoint (GET /health)
   - Dashboard route (GET /)
   - Static file serving (ServeDir for /static)
6. Create base Askama template with:
   - Tailwind CSS link
   - Datastar script (CDN)
   - Navigation header
   - Main content area
7. Create dashboard template with Tailwind components:
   - Header with title
   - Grid layout for stats cards
   - Simple welcome message
8. Add route for dashboard page
9. Update main.rs to support `cargo run -- serve` subcommand
10. Test: Run Tailwind build, run server, visit http://localhost:3000, see styled dashboard

**Success Criteria**:
- Server starts on port 3000
- Dashboard page renders with Tailwind styling
- Navigation looks modern and styled
- Static CSS (Tailwind output) loads correctly
- Health check endpoint responds at /health
- Responsive design works on mobile/desktop

---

### Phase 2: WorkOS Authentication
**Goal**: Implement JWT-based authentication with WorkOS

**Files to Create/Modify**:
- `src/web/middleware/auth.rs` - JWT validation middleware
- `src/web/middleware/roles.rs` - Role-based access control
- `src/web/routes/auth.rs` - Login/logout routes
- `src/web/models/auth.rs` - Auth types (Claims, User, Role enum)
- `src/web/state.rs` - Add WorkOS client to AppState
- `templates/login.html` - Login page
- `templates/base.html` - Update with user info display
- `.env` - Add WorkOS credentials (WORKOS_API_KEY, WORKOS_CLIENT_ID)

**Environment Variables**:
```env
WORKOS_API_KEY=sk_test_...
WORKOS_CLIENT_ID=client_...
WORKOS_REDIRECT_URI=http://localhost:3000/api/auth/callback
JWT_SECRET=your-secret-key
```

**Implementation Steps**:
1. Add workos and jsonwebtoken to Cargo.toml
2. Create Role enum (Admin, Viewer)
3. Create Claims struct with user_id, email, role
4. Create JWT validation middleware
5. Create role-checking middleware (require_admin, require_viewer)
6. Implement WorkOS login flow:
   - GET /login - Show login page with WorkOS button
   - GET /api/auth/callback - Handle WorkOS callback, create JWT
   - POST /api/auth/logout - Clear client-side token
7. Update AppState with WorkOS client and JWT secret
8. Apply auth middleware to protected routes
9. Update base.html to show user info when logged in
10. Test: Login via WorkOS, verify JWT, access protected pages

**Success Criteria**:
- User can log in via WorkOS
- JWT token is issued and stored in cookie
- Protected routes require valid JWT
- Admin-only routes block Viewer role
- User info displays in navigation

---

### Phase 3: View Existing Comparisons
**Goal**: Display list of comparisons and view details with charts

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add comparison routes
- `src/web/routes/api.rs` - API endpoints for comparison data
- `src/web/utils.rs` - File listing and parsing utilities
- `templates/comparisons/list.html` - List comparisons
- `templates/comparisons/view.html` - View comparison details
- `templates/partials/comparison_card.html` - Comparison card component
- `templates/partials/chart_viewer.html` - Chart display component

**Implementation Steps**:
1. Create utility to scan output/ directory for comparison files
2. Parse comparison CSV filenames to extract date range
3. Create API endpoint GET /api/comparisons (returns list with metadata)
4. Create page route GET /comparisons (renders list.html)
5. Create list.html template with comparison cards
6. Create API endpoint GET /api/comparisons/:from/:to (returns CSV as JSON)
7. Create page route GET /comparisons/:from/:to (renders view.html)
8. Create view.html with:
   - Summary statistics
   - Top gainers/losers tables
   - Chart display (SVG embeds)
   - Link to download CSV
9. Add route to serve chart SVGs: GET /api/charts/:from/:to/:type
10. Test: View list, click comparison, see details and charts

**Success Criteria**:
- List shows all available comparisons with date ranges
- Clicking a comparison shows detailed view
- Charts render correctly (all 4 types: gainers/losers, distribution, rank movements, dashboard)
- CSV data displays in tables
- Markdown summary displays (if available)

---

### Phase 4: View Market Cap Snapshots
**Goal**: Display market cap data for specific dates

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add market cap routes
- `src/web/routes/api.rs` - Market cap API endpoints
- `src/web/utils.rs` - Parse market cap CSV files
- `templates/market_caps/list.html` - List available dates
- `templates/market_caps/view.html` - View market caps for date
- `templates/partials/table.html` - Reusable data table component

**Implementation Steps**:
1. Create utility to scan output/ for marketcaps_{date}_*.csv files
2. Create API endpoint GET /api/market-caps (returns list of dates)
3. Create API endpoint GET /api/market-caps/:date (returns CSV as JSON)
4. Create page route GET /market-caps (renders list.html)
5. Create list.html with date cards/list
6. Create page route GET /market-caps/:date (renders view.html)
7. Create view.html with:
   - Date and timestamp
   - Total market cap (USD/EUR)
   - Top 10 companies table
   - Full data table (sortable with Datastar)
   - Currency breakdown
8. Add sorting and filtering with Datastar signals
9. Test: View list, click date, see market cap data

**Success Criteria**:
- List shows all available dates
- Clicking a date shows market cap data
- Tables are sortable
- Currency conversions display correctly
- Rank numbers display correctly

---

### Phase 5: Generate Comparisons on Demand
**Goal**: Allow admins to trigger new comparisons via web UI with progress updates

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add new comparison form route
- `src/web/routes/api.rs` - POST endpoint for comparison generation
- `src/web/models/api_requests.rs` - ComparisonRequest struct
- `src/web/models/api_responses.rs` - ComparisonResponse, ProgressEvent structs
- `templates/comparisons/new.html` - Form to create comparison
- `templates/partials/progress.html` - Progress indicator component

**Implementation Steps**:
1. Create GET /comparisons/new route (Admin only)
2. Create new.html with form:
   - From date picker
   - To date picker
   - Options: Generate charts (checkbox)
   - Submit button
3. Create ComparisonRequest struct (from_date, to_date, generate_charts)
4. Create POST /api/comparisons endpoint:
   - Validate dates
   - Call compare_market_caps() function (existing code)
   - Stream progress via SSE (Datastar integration)
   - Return result (file paths)
5. Use Datastar to:
   - Handle form submission
   - Show progress updates
   - Redirect to comparison view on completion
6. Create SSE event stream for progress:
   - Checking for CSV files...
   - Parsing data...
   - Calculating changes...
   - Generating charts... (if requested)
   - Complete!
7. Test: Submit form, see progress, view result

**Success Criteria**:
- Form validates dates (must have data for both dates)
- Progress updates stream to UI
- On completion, redirects to comparison view
- Generated comparison appears in list
- Error handling for missing data

---

### Phase 6: Fetch Market Caps on Demand
**Goal**: Allow admins to fetch market cap data for a specific date

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add fetch form route
- `src/web/routes/api.rs` - POST endpoint for fetching
- `src/web/models/api_requests.rs` - FetchMarketCapsRequest struct
- `templates/market_caps/fetch.html` - Form to fetch market caps

**Implementation Steps**:
1. Create GET /market-caps/fetch route (Admin only)
2. Create fetch.html with form:
   - Date picker
   - Submit button
3. Create FetchMarketCapsRequest struct (date)
4. Create POST /api/market-caps/fetch endpoint:
   - Validate date
   - Call fetch_specific_date_marketcaps() (existing code)
   - Stream progress via SSE
   - Return result (file path)
5. Use Datastar to handle form and progress
6. Stream progress events:
   - Fetching exchange rates...
   - Fetching market caps for {ticker}... (with count/total)
   - Converting currencies...
   - Writing CSV...
   - Complete!
7. On completion, redirect to market-caps/:date view
8. Test: Submit date, see progress, view result

**Success Criteria**:
- Form validates date format
- Progress updates show ticker-by-ticker progress
- API rate limiting handled gracefully
- On completion, redirects to market cap view
- Errors handled (e.g., API key invalid, rate limit exceeded)

---

### Phase 7: Advanced Analytics - Trend Analysis
**Goal**: Multi-date trend analysis with CAGR, volatility, and visualizations

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add trend analysis routes
- `src/web/routes/api.rs` - Trend analysis API endpoint
- `src/web/models/api_requests.rs` - TrendAnalysisRequest struct
- `templates/analytics/trends.html` - Trend analysis page

**Implementation Steps**:
1. Create GET /analytics/trends route (Admin only)
2. Create trends.html with form:
   - Multiple date picker (comma-separated or multiple inputs)
   - Minimum 2 dates, maximum 20
   - Submit button
3. Create TrendAnalysisRequest struct (dates: Vec<String>)
4. Create POST /api/analytics/trends endpoint:
   - Validate dates (must have data for all dates)
   - Call trend analysis function (from advanced_comparisons.rs)
   - Stream progress via SSE
   - Return result (CSV and summary paths)
5. Display results:
   - Overall trend summary (total market cap change)
   - CAGR for period
   - Top performers table
   - Worst performers table
   - Most volatile stocks
   - Line chart showing trend over time (could add charting)
6. Test: Submit multiple dates, see trend analysis

**Success Criteria**:
- Form accepts multiple dates
- Trend analysis runs successfully
- Results display with all metrics (CAGR, volatility, max drawdown)
- Best/worst performers identified
- Can export results as CSV

---

### Phase 8: Advanced Analytics - YoY and QoQ
**Goal**: Year-over-year and quarter-over-quarter comparisons

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add YoY and QoQ routes
- `src/web/routes/api.rs` - YoY and QoQ API endpoints
- `src/web/models/api_requests.rs` - YoyRequest, QoqRequest structs
- `templates/analytics/yoy.html` - YoY page
- `templates/analytics/qoq.html` - QoQ page

**Implementation Steps**:
1. Create GET /analytics/yoy and /analytics/qoq routes
2. Create forms:
   - YoY: Date + number of years (default 3)
   - QoQ: Date + number of quarters (default 4)
3. Create POST endpoints that call existing YoY/QoQ functions
4. Display results with comparison tables
5. Test both YoY and QoQ comparisons

**Success Criteria**:
- YoY shows data for current year and N previous years
- QoQ shows data for current quarter and N previous quarters
- Comparisons display percentage changes
- Period-over-period growth rates calculated

---

### Phase 9: Advanced Analytics - Rolling & Benchmarks
**Goal**: Rolling period comparisons and benchmark comparisons

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add rolling and benchmark routes
- `src/web/routes/api.rs` - Rolling and benchmark API endpoints
- `templates/analytics/rolling.html` - Rolling period page
- `templates/analytics/benchmarks.html` - Benchmark page

**Implementation Steps**:
1. Create GET /analytics/rolling route with form:
   - Date picker
   - Period dropdown (30d, 90d, 180d, 1y, custom)
   - Custom period input (if custom selected)
2. Create POST /api/analytics/rolling endpoint
3. Create GET /analytics/benchmarks route with form:
   - From date
   - To date
   - Benchmark selection (S&P 500, MSCI World, etc.)
4. Create POST /api/analytics/benchmark endpoint
5. Display results with comparison tables
6. Test rolling and benchmark comparisons

**Success Criteria**:
- Rolling comparisons work for all period types
- Benchmark comparisons show relative performance
- Outperformers and underperformers identified

---

### Phase 10: Advanced Analytics - Peer Groups
**Goal**: Peer group comparison dashboard

**Files to Create/Modify**:
- `src/web/routes/pages.rs` - Add peer groups route
- `src/web/routes/api.rs` - Peer groups API endpoints
- `templates/analytics/peer_groups.html` - Peer groups page

**Implementation Steps**:
1. Create GET /analytics/peer-groups route
2. Create GET /api/peer-groups endpoint (list all groups)
3. Create GET /api/peer-groups/:name endpoint (get group details)
4. Create form to compare peer groups:
   - From date
   - To date
   - Group selection (multi-select: Luxury, Sportswear, Fast Fashion, etc.)
5. Create POST /api/analytics/peer-groups endpoint
6. Display results:
   - Table showing each peer group's performance
   - Best/worst performing groups
   - Individual company performance within groups
   - Market share shifts within groups
7. Test peer group comparisons

**Success Criteria**:
- All 8 peer groups accessible
- Can compare multiple groups simultaneously
- Within-group and across-group comparisons work
- Performance metrics accurate

---

### Phase 11: Docker & Deployment
**Goal**: Containerize application for Fly.io deployment

**Files to Create**:
- `Dockerfile` - Multi-stage build
- `docker-compose.yml` - Local development setup
- `fly.toml` - Fly.io configuration
- `.dockerignore` - Exclude unnecessary files

**Dockerfile Structure**:
```dockerfile
# Build stage
FROM rust:1.75 as builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y sqlite3 ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/top200-rs /app/
COPY static ./static
COPY config.toml ./
RUN mkdir -p output
ENV DATABASE_URL=sqlite:data.db
EXPOSE 3000
CMD ["./top200-rs", "serve"]
```

**Implementation Steps**:
1. Create Dockerfile with multi-stage build
2. Create .dockerignore
3. Create docker-compose.yml for local testing:
   - App service
   - Volume for output/ directory
   - Volume for data.db
   - Environment variables
4. Create fly.toml:
   - App name
   - Region
   - Internal port 3000
   - Volume for persistent storage (output/ and data.db)
   - Secrets for WORKOS_API_KEY, JWT_SECRET, FMP_API_KEY
5. Test local Docker build and run
6. Deploy to Fly.io:
   ```bash
   fly launch
   fly secrets set WORKOS_API_KEY=... JWT_SECRET=... FMP_API_KEY=...
   fly volumes create top200_data --size 1
   fly deploy
   ```

**Success Criteria**:
- Docker image builds successfully
- Container runs locally via docker-compose
- All features work in containerized environment
- Persistent storage for database and output files
- Deploys to Fly.io successfully
- Environment variables configured via secrets

---

### Phase 12: Polish & Testing
**Goal**: Improve UX, add error handling, and test all features

**Tasks**:
1. Add loading states to all forms
2. Improve error messages (user-friendly, not just anyhow errors)
3. Add toast notifications for success/error messages
4. Add input validation on all forms
5. Add rate limiting to prevent API abuse
6. Add logging (structured logging with tracing)
7. Add metrics endpoint for monitoring
8. Test all features end-to-end:
   - Auth flows (login/logout)
   - View comparisons
   - View market caps
   - Generate comparisons
   - Fetch market caps
   - All analytics features
9. Add responsive design (mobile-friendly)
10. Add dark mode toggle (optional)
11. Performance testing with large datasets
12. Security audit (JWT validation, SQL injection prevention, XSS prevention)

**Success Criteria**:
- All features tested and working
- Error handling graceful
- Performance acceptable (<2s page loads)
- Mobile-friendly UI
- Logs structured and useful
- Security best practices followed

---

## Critical Files to Modify

### src/main.rs
- Add `serve` subcommand to CLI
- Initialize web server in addition to CLI commands
- Share database pool between CLI and web

### src/web/server.rs
- Axum router setup
- Middleware stack (auth, CORS, logging)
- Static file serving
- SSE setup for progress streaming

### src/web/state.rs
```rust
pub struct AppState {
    pub db_pool: SqlitePool,
    pub workos_client: WorkOS,
    pub jwt_secret: String,
    pub config: Config,
}
```

### src/web/middleware/auth.rs
- JWT validation from Authorization header or cookie
- Extract Claims from token
- Add User to request extensions

### src/web/middleware/roles.rs
- Check user role (Admin/Viewer)
- Return 403 if insufficient permissions

### src/web/routes/api.rs
- All JSON API endpoints
- SSE streaming for long operations
- Error handling with appropriate status codes

### src/web/routes/pages.rs
- All HTML page rendering endpoints
- Pass data to Askama templates
- Handle redirects after auth

### templates/base.html
- Main layout with navigation
- User info display
- Datastar initialization
- CSS/JS includes

## Environment Variables

```env
# Database
DATABASE_URL=sqlite:data.db

# API Keys
FMP_API_KEY=your_fmp_api_key
FINANCIALMODELINGPREP_API_KEY=your_fmp_api_key

# WorkOS
WORKOS_API_KEY=sk_test_...
WORKOS_CLIENT_ID=client_...
WORKOS_REDIRECT_URI=http://localhost:3000/api/auth/callback

# JWT
JWT_SECRET=your-secret-key-change-in-production

# Server
HOST=0.0.0.0
PORT=3000
```

## Testing Strategy

1. **Unit Tests**: Test individual functions in web utilities
2. **Integration Tests**: Test API endpoints with test database
3. **E2E Tests**: Manual testing of full user flows
4. **Load Tests**: Test with large datasets (all 160 tickers)
5. **Security Tests**: Test auth flows, token validation, role enforcement

## Future Enhancements (Post-MVP)

1. Background job queue (Redis + tokio cron)
2. Real-time WebSocket updates for live market data
3. Email notifications for completed jobs
4. API rate limiting per user
5. Export reports as PDF
6. Custom peer group creation
7. Alerts for significant market cap changes
8. Historical data visualization (line charts over time)
9. Portfolio tracking (user's watch list)
10. Public API with API keys

## Success Criteria for Complete Implementation

- [ ] User can log in via WorkOS
- [ ] Admin and Viewer roles enforced
- [ ] All existing comparisons viewable on web
- [ ] Charts display correctly
- [ ] Can generate new comparisons via web UI
- [ ] Can fetch market cap data via web UI
- [ ] Progress updates stream in real-time
- [ ] All analytics features accessible
- [ ] Docker container runs successfully
- [ ] Deploys to Fly.io
- [ ] Mobile-friendly responsive design
- [ ] Error handling graceful
- [ ] Performance acceptable (<2s page loads)

## Estimated Complexity

### MVP (Phases 1-6)
- **Phase 1** (Server + Tailwind): Medium complexity - ~3-4 hours
- **Phase 2** (WorkOS Auth): Medium complexity - ~3-4 hours
- **Phase 3** (View Comparisons): Low-Medium complexity - ~3-4 hours
- **Phase 4** (View Market Caps): Low complexity - ~2-3 hours
- **Phase 5** (Generate Comparisons): Medium complexity - ~4-5 hours
- **Phase 6** (Fetch Market Caps): Medium complexity - ~3-4 hours

**MVP Total**: ~18-24 hours

### Post-MVP (Phases 7-12)
- **Phase 7-10** (Analytics): Medium complexity - ~6-8 hours
- **Phase 11** (Docker): Low complexity - ~2-3 hours
- **Phase 12** (Polish): Medium complexity - ~4-6 hours

**Post-MVP Total**: ~12-17 hours

**Complete Implementation**: 30-41 hours

## Risks and Mitigations

1. **Risk**: Long-running operations timeout
   - **Mitigation**: Start with SSE streaming, move to background jobs if needed

2. **Risk**: WorkOS integration complexity
   - **Mitigation**: Start with basic JWT validation, can swap auth providers later

3. **Risk**: File-based data storage doesn't scale
   - **Mitigation**: Keep file-based approach initially, consider moving to DB if performance issues

4. **Risk**: Datastar learning curve
   - **Mitigation**: Start with simple use cases, can replace with HTMX if needed

5. **Risk**: Container size too large
   - **Mitigation**: Multi-stage Docker build, optimize dependencies

---

## Quick Reference: MVP Implementation Order

For the MVP implementation (Phases 1-6), follow this order:

1. **Setup Dependencies** (Phase 1)
   - Add Rust crates: axum, tower-http, askama
   - Setup Tailwind CSS with npm/package.json
   - Create basic project structure

2. **Basic Server** (Phase 1 continued)
   - Create web module with routes and state
   - Setup Axum router with static file serving
   - Create base templates with Tailwind styling
   - Test: Visit dashboard, see styled page

3. **Authentication** (Phase 2)
   - Integrate WorkOS SDK
   - Create JWT middleware
   - Add login/logout routes
   - Protect routes with auth
   - Test: Login flow works

4. **View Existing Data** (Phases 3-4)
   - Scan output/ directory for files
   - Display comparisons list
   - Display comparison details with charts
   - Display market cap snapshots
   - Test: Can view all existing data

5. **Generate New Data** (Phases 5-6)
   - Add forms to trigger operations
   - Implement SSE streaming for progress
   - Connect to existing CLI functions
   - Test: Generate comparison, fetch market caps

After completing the MVP, the web interface will be fully functional for daily use. Advanced analytics (Phases 7-10) can be added based on actual usage patterns.

---

## Notes

- All existing CLI functionality remains intact
- Web interface is additive, not replacing CLI
- Database schema changes are minimal (just users and job_status tables)
- Most business logic reused from existing modules
- Datastar handles most frontend complexity (no heavy JS framework needed)
