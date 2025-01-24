<!--
SPDX-FileCopyrightText: 2025 Joost van der Laan <joost@fashionunited.com>

SPDX-License-Identifier: AGPL-3.0-only
-->

[![CI](https://github.com/javdl/top200-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/javdl/top200-rs/actions/workflows/ci.yml)[![Daily Data Collection](https://github.com/javdl/top200-rs/actions/workflows/daily-run.yml/badge.svg)](https://github.com/javdl/top200-rs/actions/workflows/daily-run.yml)

# top200-rs

## TODO

- add a command so I can test with just a few tickers

## Run

```sh
nix develop
cargo run

# or
cargo run -- export-us
cargo run -- export-eu
cargo run -- export-combined
```

## Database

```sh
sqlx database create
sqlx migrate run

# first time only
sqlx migrate add init
```

```sh
$ sqlite3 top200.db < tests/market_caps_totals_per_year.sql

# Output
2020|99
2021|107
2023|2
2024|111
```

## Example commands

```sh
cargo run -- export-us
cargo run -- export-eu
cargo run -- export-combined
cargo run -- historical-marketcaps --start-year 2020 --end-year 2024
# which fr Windsurf to use Nix, will become:
nix develop --command cargo run -- historical-marketcaps --start-year 2020 --end-year 2024
```

## Calculate market caps history

Many analysts use the last trading day of each year to calculate yearly market cap snapshots. This ensures a consistent reference point and reflects end-of-year valuations. Other methods include averaging across each year or using the first trading day, but the closing price on December 31 (or the final trading session) is most common.

## License

To set license and copyright information, run:

```sh
reuse annotate --copyright="Joost van der Laan <joost@fashionunited.com>" --license="AGPL-3.0-only" --skip-unrecognised *

# or to check if all files are annotated correctly
reuse lint
```

## Changelog

Converted exchange prefixes to FMP format:
EPA: -> .PA (Paris)
BME: -> .MC (Madrid)
VTX: -> .SW (Swiss)
ETR: -> .DE (German)
LON: -> .L (London)
BIT: -> .MI (Milan)
STO: -> .ST (Stockholm)
TYO: -> .T (Tokyo)
HKG: -> .HK (Hong Kong)
BVMF: -> .SA (Brazil)
TSE: -> .TO (Toronto)
Updated some tickers that have changed:
COH -> TPR (Coach is now Tapestry)
KORS -> CPRI (Michael Kors is now Capri Holdings)
LB -> BBWI (L Brands is now Bath & Body Works)

## Top 100 Tickers

EPA:MC # LVMH
NYSE:NKE # Nike
BME:ITX # Inditex
EPA:CDI # Dior
EPA:KER # Kering
EPA:RMS # Hermès
NYSE:TJX # TJX
VTX:CFR # Richemont
ETR:ADS # adidas
TYO:9983 # Fast Retailing
BIT:LUX # Luxottica
NASDAQ:ROST # Ross Stores
NYSE:VFC # VF
STO:HM-B # H&M
VTX:UHR # Swatch Group
NYSE:COH # Coach
ETR:ZAL # Zalando
NYSE:GPS # Gap
NYSE:LB # L Brands
NYSE:TIF # Tiffany & Co.
HKG:1929 # Chow Tai Fook
NYSE:PVH # PVH HQ
NASDAQ:LULU # Lululemon
HKG:1913 # Prada Group
NYSE:VIPS # Vipshop Holdings
LON:BRBY # Burberry
LON:NXT # Next
NYSE:KORS # Michael Kors
NYSE:M # Macy's
BIT:MONC # Moncler
NYSE:RL # Ralph Lauren
NYSE:JWN # Nordstrom
LON:ASC # ASOS
BVMF:LREN3 # Lojas Renner
NYSE:HBI # Hanes
NYSE:UA # Under Armour
ETR:PUM # PUMA
LON:MKS # Marks & Spencer (M&S)
NYSE:SKX # Skechers
ETR:BOSS # Hugo Boss
TSE:GIL # Gildan
NYSE:CRI # Carter's
NASDAQ:COLM # Columbia Sportswear
LON:JD # JD Sports
NYSE:FL # Foot locker
BIT:SFER # Salvatore Ferragamo
BIT:YNAP # YOOX Net-a-Porter Group
SHE:002563 # Semir
NASDAQ:URBN # Urban Outfitters
NYSE:AEO # American Eagle Outfitters
NYSE:DKS # Dick's Sporting Goods
TYO:7936 # Asics
NYSE:DECK # Deckers Outdoor
SHA:600612 # Lao Feng Xiang Jewelry
NYSE:WWW # Wolverine
LON:BOO # Boohoo.Com
LON:SPD # Sports Direct
NASDAQ:SHOO # Steve Madden
BVMF:GRND3 # Grendene
BIT:TOD # TOD'S
BVMF:ALPA4 # Alpargatas - Havaianas
NASDAQ:PLCE # Children's Place
NYSE:DDS # Dillard's
BIT:BC # Brunello Cucinelli
HKG:2331 # Li Ning
LON:SGP # Supergroup
LON:TED # Ted Baker
NASDAQ:GIII # G-III Apparel Group
NSE:ABFRL # Pantaloons
NYSE:DSW # DSW
NSE:ARVIND # Arvind
TYO:8016 # Onward Holdings
NYSE:ANF # Abercrombie & Fitch
NYSE:OXM # Oxford Industries
TSE:HBC # Hudson's Bay
NYSE:GES # GUESS
TYO:7606 # United Arrows
NYSE:CAL # Caleres
NYSE:CHS # Chico's
NYSE:JCP # J.C. Penney
SHE:002269 # Metersbonwe
BVMF:HGTX3 # Cia Hering
NYSE:BKE # Buckle
HKG:3998 # Bosideng
BIT:GEO # GEOX
NYSE:GCO # Genesco
HKG:0330 # Esprit
LON:MUL # Mulberry
NASDAQ:FOSL # Fossil
EBR:VAN # Van de Velde
NYSE:EXPR # Express
NASDAQ:ASNA # Ascena Retail Group
ETR:GWI1 # Gerry Weber
NASDAQ:VRA # Vera Bradley
NYSE:CATO # Cato Fashion
NASDAQ:FRAN # Francesca's

For FinancialModelingPrep API, we need to modify these tickers to match their format. Here are some key changes needed:

Remove the exchange prefix (like NYSE:, NASDAQ:)
For non-US exchanges, we need to add the exchange suffix:

London Stock Exchange: .LSE
Tokyo Stock Exchange: .T
Hong Kong: .HK
European exchanges generally need country codes

Here's the converted list:
MC.PA # LVMH
NKE # Nike
ITX.MC # Inditex
CDI.PA # Dior
KER.PA # Kering
RMS.PA # Hermès
TJX # TJX
CFR.SW # Richemont
ADS.DE # adidas
9983.T # Fast Retailing
LUX.MI # Luxottica
ROST # Ross Stores
VFC # VF
HM-B.ST # H&M
UHR.SW # Swatch Group
COH # Coach
ZAL.DE # Zalando
GPS # Gap
LB # L Brands
TIF # Tiffany & Co.
1929.HK # Chow Tai Fook
PVH # PVH HQ
LULU # Lululemon
1913.HK # Prada Group
VIPS # Vipshop Holdings
BRBY.L # Burberry
NXT.L # Next
KORS # Michael Kors
M # Macy's
MONC.MI # Moncler
RL # Ralph Lauren
JWN # Nordstrom
ASC.L # ASOS
LREN3.SA # Lojas Renner
HBI # Hanes
UA # Under Armour
PUM.DE # PUMA
MKS.L # Marks & Spencer (M&S)
SKX # Skechers
BOSS.DE # Hugo Boss
GIL.TO # Gildan
CRI # Carter's
COLM # Columbia Sportswear
JD.L # JD Sports
FL # Foot locker
SFER.MI # Salvatore Ferragamo
YNAP.MI # YOOX Net-a-Porter Group
002563.SZ # Semir
URBN # Urban Outfitters
AEO # American Eagle Outfitters
DKS # Dick's Sporting Goods
7936.T # Asics
DECK # Deckers Outdoor
600612.SS # Lao Feng Xiang Jewelry
WWW # Wolverine
BOO.L # Boohoo.Com
SPD.L # Sports Direct
SHOO # Steve Madden
GRND3.SA # Grendene
TOD.MI # TOD'S
ALPA4.SA # Alpargatas - Havaianas
PLCE # Children's Place
DDS # Dillard's
BC.MI # Brunello Cucinelli
2331.HK # Li Ning
SGP.L # Supergroup
TED.L # Ted Baker
GIII # G-III Apparel Group
ABFRL.NS # Pantaloons
DSW # DSW
ARVIND.NS # Arvind
8016.T # Onward Holdings
ANF # Abercrombie & Fitch
OXM # Oxford Industries
HBC.TO # Hudson's Bay
GES # GUESS
7606.T # United Arrows
CAL # Caleres
CHS # Chico's
JCP # J.C. Penney
002269.SZ # Metersbonwe
HGTX3.SA # Cia Hering
BKE # Buckle
3998.HK # Bosideng
GEO.MI # GEOX
GCO # Genesco
0330.HK # Esprit
MUL.L # Mulberry
FOSL # Fossil
VAN.BR # Van de Velde
EXPR # Express
ASNA # Ascena Retail Group
GWI1.DE # Gerry Weber
VRA # Vera Bradley
CATO # Cato Fashion
FRAN # Francesca's

Here's a list of just the US stocks, formatted for Polygon API. For Polygon, we can use just the ticker symbol without any exchange prefix:
TJX # TJX
ROST # Ross Stores
VFC # VF
COH # Coach
GPS # Gap
LB # L Brands
TIF # Tiffany & Co.
PVH # PVH HQ
LULU # Lululemon
VIPS # Vipshop Holdings
KORS # Michael Kors
M # Macy's
RL # Ralph Lauren
JWN # Nordstrom
HBI # Hanes
UA # Under Armour
SKX # Skechers
CRI # Carter's
COLM # Columbia Sportswear
FL # Foot locker
URBN # Urban Outfitters
AEO # American Eagle Outfitters
DKS # Dick's Sporting Goods
DECK # Deckers Outdoor
WWW # Wolverine
SHOO # Steve Madden
PLCE # Children's Place
DDS # Dillard's
GIII # G-III Apparel Group
DSW # DSW
ANF # Abercrombie & Fitch
OXM # Oxford Industries
GES # GUESS
CAL # Caleres
CHS # Chico's
JCP # J.C. Penney
BKE # Buckle
GCO # Genesco
FOSL # Fossil
EXPR # Express
ASNA # Ascena Retail Group
VRA # Vera Bradley
CATO # Cato Fashion
FRAN # Francesca's
Note: Since this list appears to be from a past date, some of these tickers might have changed or companies might have been delisted. It would be good practice to verify the current status of each ticker before using them with the Polygon API.

Here's the list of non-US stocks formatted for FinancialModelingPrep API:
MC.PA # LVMH
ITX.MC # Inditex
CDI.PA # Dior
KER.PA # Kering
RMS.PA # Hermès
CFR.SW # Richemont
ADS.DE # adidas
9983.T # Fast Retailing
LUX.MI # Luxottica
HM-B.ST # H&M
UHR.SW # Swatch Group
ZAL.DE # Zalando
1929.HK # Chow Tai Fook
1913.HK # Prada Group
BRBY.L # Burberry
NXT.L # Next
MONC.MI # Moncler
ASC.L # ASOS
LREN3.SA # Lojas Renner
PUM.DE # PUMA
MKS.L # Marks & Spencer (M&S)
BOSS.DE # Hugo Boss
GIL.TO # Gildan
JD.L # JD Sports
SFER.MI # Salvatore Ferragamo
YNAP.MI # YOOX Net-a-Porter Group
002563.SZ # Semir
7936.T # Asics
600612.SS # Lao Feng Xiang Jewelry
BOO.L # Boohoo.Com
SPD.L # Sports Direct
GRND3.SA # Grendene
TOD.MI # TOD'S
ALPA4.SA # Alpargatas - Havaianas
BC.MI # Brunello Cucinelli
2331.HK # Li Ning
SGP.L # Supergroup
TED.L # Ted Baker
ABFRL.NS # Pantaloons
ARVIND.NS # Arvind
8016.T # Onward Holdings
HBC.TO # Hudson's Bay
7606.T # United Arrows
002269.SZ # Metersbonwe
HGTX3.SA # Cia Hering
3998.HK # Bosideng
GEO.MI # GEOX
0330.HK # Esprit
MUL.L # Mulberry
VAN.BR # Van de Velde
GWI1.DE # Gerry Weber
Note: Some markets might have limited data availability through FinancialModelingPrep API, especially for smaller international companies. It would be good to verify data availability for these symbols before using them in your application.

## Changelog

COH - Coach, Inc -> TPR (Tapestry)
KORS - Michael Kors, Inc. -> CPRI (Michael Kors is now Capri Holdings)
LB - L Brands, Inc. -> BBWI (L Brands is now Bath & Body Works)
Updated some tickers that have changed:

TIF (Tiffany & Co.) - Delisted after being acquired by LVMH in January 2021
YNAP.MI (YOOX Net-a-Porter Group) - Delisted after being fully acquired by Richemont in 2018
SPD.L (Sports Direct) - Changed name and ticker to Frasers Group (FRAS.L) in 2019
ALPA4.SA (Alpargatas) - Still trades on Brazilian exchange as ALPA4.SA and ALPA3.SA - might be a data provider issue
SGP.L (Supergroup) - Changed name and ticker to Superdry (SDRY.L)
TED.L (Ted Baker) - Delisted in 2023 after being acquired by Authentic Brands Group
DSW (Designer Shoe Warehouse) - Changed name and ticker to Designer Brands Inc. (DBI)
HBC.TO (Hudson's Bay) - Delisted in March 2020 after going private
HGTX3.SA (Cia Hering) - Merged with Grupo Soma in 2021, now part of SOMA3.SA
ASNA (Ascena Retail Group) - Filed for bankruptcy in 2020 and was delisted
GWI1.DE (Gerry Weber) - Filed for insolvency in 2019, restructured and now trades as GWI2.DE
FRAN (Francesca's) - Filed for bankruptcy in 2020 and was delisted

Fetching data for DLTI.TA
⚠️  Warning: No conversion rate found for ILA to USD
⚠️  Warning: No conversion rate found for ILA to EUR
⚠️  Warning: No conversion rate found for ILA to USD

Zou ILS moeten zijn wss?
