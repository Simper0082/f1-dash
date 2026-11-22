Team logos for the wheel screen banners (pit in / pit out).

Add one square PNG per team named by the team's slug, e.g. `red-bull-racing.png`, `mercedes.png`, `ferrari.png`.
The slug is the feed's team name lower-cased with non-alphanumerics turned into `-`
(`/api/wheel` returns it as `driver.team_slug`). A missing file just hides the logo.
On the ESP32 the same files, converted to BMP, go on the SD card under `/teams/<slug>.bmp`.
