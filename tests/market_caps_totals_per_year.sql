-- Query to verify the number of market caps per year
SELECT 
    strftime('%Y', datetime(timestamp, 'unixepoch')) as year,
    COUNT(*) as count
FROM market_caps 
GROUP BY year 
ORDER BY year;
